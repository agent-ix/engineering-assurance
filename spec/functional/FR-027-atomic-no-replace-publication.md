---
id: FR-027
title: "Publish retained bytes atomically without replacing existing evidence"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: "requires"
---

# FR-027: Publish retained bytes atomically without replacing existing evidence

## Description

The `retained_files` module (capability feature `retained-files`) SHALL publish a complete byte string to a destination path so that the
destination either does not exist or holds exactly those bytes. The module never
replaces or modifies an existing destination. This is the write-side primitive a
retained-evidence writer uses (EA-18). It is a filesystem capability, so it is
admitted beside FR-019's producer execution as the second module allowed to
name filesystem APIs (see FR-014-AC-4), and no I/O-free domain module gains
filesystem access.

Scope note: every existing plain write in this crate is unchanged by this
requirement and is listed under Constraints (FR-027-CON-4). This requirement
adds the primitive only.

## Inputs

- A destination path whose parent directory the caller has already created.
- The complete bytes to publish (possibly empty).
- For the separately callable commit step: a sidecar path holding those bytes,
  the destination path, and the digest and length of the intended bytes.

## Outputs

- `PublishOutcome::Published` when this call created the destination.
- `PublishOutcome::AlreadyIdentical` when a regular file with identical bytes
  already existed.
- A typed `PublishError`: `Collision` (carrying the existing and the attempted
  `ContentDigest`), `DestinationNotRegular`, `DestinationInvalid`,
  `ParentMissing`, `TooLarge`, `LinkUnsupported`, `SidecarUnavailable`
  (unique name not obtained within 16 attempts) and `Io` (a closed stage name
  and `std::io::ErrorKind`, no path text).

## Behavior

- When the destination is absent, the publisher SHALL write the bytes to a
  sidecar in the destination's own directory, flush and fsync it, and then
  commit by `std::fs::hard_link(sidecar, destination)`.
- The publisher SHALL create the sidecar with `create_new` (exclusive create)
  and SHALL name it `.<destination file name>.tmp-<pid>-<counter>`, where the
  counter is a process-wide atomic; on an exclusive-create collision it SHALL
  retry with the next counter up to 16 attempts. Uniqueness rests on the
  exclusive create, not on the name; no clock, randomness or new dependency is
  used.
- When the destination exists as a regular file with identical bytes, the
  publisher SHALL return `AlreadyIdentical`, creating no sidecar.
- When the destination exists as a regular file with different bytes, the
  publisher SHALL return `Collision` carrying both digests, leaving the
  destination unmodified.
- When the commit link fails because the destination now exists, the commit
  step SHALL re-read the destination and return success (`AlreadyIdentical`) for
  identical bytes and `Collision` for different bytes, treating the failure as
  neither an unconditional error nor an unconditional success.
- The commit step SHALL be a separately callable public function so a test can
  create the destination between the sidecar write and the link
  deterministically.
- The publisher SHALL remove the sidecar on success, collision, error and
  panic, through a drop guard, and a removal failure does not mask the
  primary result.
- The publisher SHALL refuse a destination that is a symbolic link or a
  non-regular file with `DestinationNotRegular`, following no link, dangling
  links included.
- The publisher SHALL refuse a destination path with no file-name component, or
  whose file name is not one component, with `DestinationInvalid`.
- The publisher SHALL return `ParentMissing`, creating no directory, when the parent does not exist or is not a directory. The
  parent path itself MAY be reached through a symbolic link, because the caller
  chose it; only the final component is inspected.
- The publisher SHALL return `LinkUnsupported` when the filesystem cannot make a
  hard link (`ErrorKind::Unsupported`, or the operating-system permission,
  operation-not-supported, not-implemented or cross-device errors), and SHALL
  NOT fall back to copy or rename, because rename would replace.
- The publisher SHALL compare an existing destination by length and digest
  through the module's hardened bounded read (FR-028-CON-4, FR-027-AC-6).
- The publisher SHALL refuse input longer than the 1 GiB ceiling with
  `TooLarge` before writing, so every published file can later be compared and
  verified; an existing destination above the ceiling SHALL also return
  `TooLarge` rather than `Collision`, because its digest cannot be computed.
- The publisher SHALL set the sidecar mode to 0o644 on Unix (subject to the
  process umask); the destination shares the sidecar's inode and therefore its
  mode. Other platforms use the platform default. The publisher SHALL NOT
  change a mode afterwards.
- The destination SHALL appear atomically complete, because the link is made
  only after the sidecar is fsynced, so a concurrent reader observes no partial
  destination.
- When two producers publish identical bytes to one destination concurrently,
  both SHALL succeed, exactly one reporting `Published`.
- When two producers publish different bytes to one destination concurrently,
  exactly one SHALL succeed and the other returns `Collision`; there is no
  last-writer-wins.

## Error Conditions

Every refusal above is a distinguishable typed `PublishError`. No error path
leaves a sidecar behind, changes an existing destination, or reports success
for bytes that differ from those requested. A process killed between sidecar
creation and removal (SIGKILL, power loss) can leave an orphan sidecar with the
name pattern above; the destination is still never partial, and reclaiming
orphans is out of scope.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-027-CON-1 | The publisher SHALL NOT overwrite, truncate, rename over or otherwise modify an existing destination. | Safety | Test (TC-201, TC-202) |
| FR-027-CON-2 | The publisher SHALL NOT fall back to copy or rename when hard links are unsupported. | Safety | Test (TC-202) |
| FR-027-CON-3 | The `retained-files` feature SHALL add no dependency: `content-digest`, `serde` and `thiserror` only, and neither `tempfile`, `rustix`, `cap-std` nor `serde_json`. | Architecture | Test (TC-204) |
| FR-027-CON-4 | This requirement SHALL NOT migrate any existing write. The plain writes in `package_install`, `package_host`, `package_archive`, `package_audit_host`, `manifest_host`, `agent_evals_host`, `agent_evals_provider`, `evaluation_report_host`, `integration_evidence_host` and `onboarding_host` stay as they are: they write scratch, staging, report and installation outputs that are meant to be replaced or regenerated, or belong to host adapters whose replace semantics are specified by their own requirements. A retained-evidence writer that needs no-replace semantics adopts the primitive in its own change. | Scope | Inspection |
| FR-027-CON-5 | The primitive SHALL NOT claim protection against a path swapped between its own checks and use: the destination check and the link are separate steps and the open has no `O_NOFOLLOW`, as `ContentDigest::of_file` already states. | Limitation | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-027-AC-1 | Publishing to an absent destination creates a regular file holding exactly the bytes (including the empty string and a 65,537-byte input), returns `Published`, and leaves the directory holding only the destination (no sidecar). | Test (TC-201) |
| FR-027-AC-2 | Publishing the same bytes again returns `AlreadyIdentical` without creating a sidecar; publishing different bytes returns `Collision` carrying the existing and attempted digests, leaves the destination byte-for-byte and mtime unchanged, and leaves no sidecar. | Test (TC-201) |
| FR-027-AC-3 | The commit step, called after a test creates the destination with identical bytes, returns success and after a test creates it with different bytes returns `Collision`; in both cases the sidecar is removed by the guard. Injected failures (a directory destination, an unwritable directory, a panic between write and commit) leave no sidecar and no changed destination. | Test (TC-202) |
| FR-027-AC-4 | A destination that is a symbolic link (to a file, to a directory, dangling), a directory or a FIFO returns `DestinationNotRegular` and the link target is untouched; a missing parent returns `ParentMissing` and no directory is created; an empty or multi-component name returns `DestinationInvalid`. Where a filesystem without hard links is available, or by a fault-injecting seam, `LinkUnsupported` is returned and no copy or rename occurred. | Test (TC-202) |
| FR-027-AC-5 | Two threads (and, separately, two processes) publishing identical bytes both succeed with exactly one `Published`; two publishing different bytes yield exactly one success and one `Collision`, repeated at least 200 iterations per shape with the loser's digests naming both byte strings; while a writer runs, a polling reader sees only an absent destination or the complete bytes. | Test (TC-203) |
| FR-027-AC-6 | An existing destination is compared through the bounded hardened read (symbolic links and non-regular files refused, ceiling 1 GiB, short read a failure), input above the ceiling returns `TooLarge` before any write, and the sidecar mode is 0o644 under umask 022 on Unix. Tests use a lowered ceiling seam, not a 1 GiB file. | Test (TC-201) |
| FR-027-AC-7 | `retained-files` is a capability feature that compiles alone (`--lib`) and appears in the `src/lib.rs` feature table with module `retained_files`, features it enables `content-digest`, and dependencies `serde`, `thiserror`; it resolves no `serde_json`, `cap-std`, `clap`, `tar`, `zip`, `flate2`, `tempfile` or `rustix`, adds no duplicate crate version, and the default graph stays empty. | Test (TC-204) |
| FR-027-AC-8 | The FR-014-AC-4 audit exempts exactly two module files, `producer_execution` and `retained_files`, by an explicit allowlist compared for equality. `retained_files` is audited with the reusable-library role and may produce only `Filesystem` findings, plus the single `ChildProgram` finding for the `std::process::id` path if the pid is used; a finding of any other capability (environment, network, clock, child program by another path) fails, and adding a third exempt file fails. No new `RustSourceAuditRole` is added. | Test (TC-205) |

## Dependencies

- **Upstream**: FR-014 (Rust boundary, `content-digest` identity).
- **Downstream**: retained-evidence writers that adopt the primitive in later changes; FR-028 shares the module's bounded reader.

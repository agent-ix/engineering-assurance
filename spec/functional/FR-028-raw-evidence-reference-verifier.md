---
id: FR-028
title: "Verify declared raw-evidence references against retained files"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-027"
    type: "requires"
---

# FR-028: Verify declared raw-evidence references against retained files

## Description

The `retained_files` module SHALL verify a list of declared raw-evidence
references against the files under a repository root, re-resolving, re-reading
and re-digesting each one, and SHALL report every mismatch rather than the
first (EA-18). The reference type is new and small; the compatibility corpus's
`ReferencedInput` (which carries another system's `blake3` identity) is neither
reused nor changed.

## Inputs

- A repository root directory chosen by the caller.
- A base JSON Pointer chosen by the caller (empty, or starting with `/`).
- A list of references `RawEvidenceReference { path: String, size_bytes: u64,
  digest: String }`, where `digest` is the `sha256:<hex>` prefixed form. It is
  kept as text so one malformed digest yields a finding, not a failed parse of
  the whole list.
- An optional read ceiling in bytes, at most 1 GiB (the default).

## Outputs

- `Vec<ReferenceFinding>`: an empty list when every reference verifies (a
  value, not an error).
- `ReferenceFinding { location: String, kind: FindingKind, expected:
  Option<String>, observed: Option<String> }`, a typed serde struct that
  serializes `kind` as snake_case. No untyped `serde_json::Value` is emitted.
- `FindingKind`, a closed enum in this order: `PathInvalid`, `Missing`,
  `NotRegular`, `TooLarge`, `Unreadable`, `SizeMismatch`, `DigestMismatch`,
  `MalformedDigest`, `UnknownAlgorithm`.
- A typed `VerifyRequestError` for a request that cannot be evaluated at all:
  `RootInvalid`, `BasePointerInvalid`, `CeilingInvalid` and
  `TooManyReferences` (over 65,536).

## Behavior

- The verifier SHALL refuse a path with `PathInvalid` when it is empty,
  absolute, contains a backslash, a NUL or a control character, is `.`, ends in
  `/`, has an empty segment or a `..` segment, or is not already normalized.
- The verifier SHALL treat a symbolic link in any directory component of the
  path below the root as `PathInvalid` (the path escapes or may escape the
  root), whether or not the link points inside the root, and SHALL NOT follow
  it.
- The verifier SHALL report `Missing` when a component or the file is absent,
  `NotRegular` when the final component is a symbolic link or a non-regular
  file, `TooLarge` when the file exceeds the ceiling, and `Unreadable` when the
  read fails or is short, using the same rules as `ContentDigest::of_file`
  (FR-014-AC-11) through the module's own bounded reader.
- The verifier SHALL, for a readable file, report `SizeMismatch` when the
  observed length differs from `size_bytes`, and independently report
  `DigestMismatch` when the digest differs; both are reported for one
  reference when both differ.
- The verifier SHALL report `MalformedDigest` for a digest that lacks the
  `sha256:` prefix (a bare hexadecimal string included), has uppercase, or has
  the wrong length, and `UnknownAlgorithm` for a prefix naming any other
  algorithm, using `ContentDigest::parse_prefixed`; a reference with either
  still has its path and size checked.
- The verifier SHALL build each `location` as the base pointer, `/`, the
  zero-based reference index and `/path` for the path-level kinds, `/size_bytes`
  for `SizeMismatch`, or `/digest` for the digest kinds; `expected` and
  `observed` carry the declared and re-computed value where both exist, in
  `sha256:<hex>` or decimal form, and no path or operating-system text
  appears in them.
- The verifier SHALL continue after a finding and report every mismatch of every
  reference, ordered by reference index and then by the `FindingKind` order.
- The verifier SHALL treat the root as given: it has to be an existing directory
  (`RootInvalid` otherwise) and is not canonicalized; only components below it
  are checked.
- The verifier SHALL refuse a base pointer that is non-empty without a leading
  `/`, or has a `~` not followed by `0` or `1`, with `BasePointerInvalid`.

## Error Conditions

Per-reference defects are findings, never request errors. A request error
returns no findings. The verifier never panics on any path text, digest text or
file content.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-028-CON-1 | The verifier SHALL NOT write, create, or modify any file. | Safety | Test (TC-207) |
| FR-028-CON-2 | The verifier SHALL NOT reuse or change `ReferencedInput` or its `blake3` field. | Scope | Inspection |
| FR-028-CON-3 | The verifier SHALL NOT claim protection against a path swapped between its check and its open, or a file rewritten between reads other than by a short-read failure (the limitation `ContentDigest::of_file` states). | Limitation | Inspection |
| FR-028-CON-4 | The hardened reader in `retained_files` SHALL NOT change `ContentDigest::of_file`, which stays beside producer execution (it cannot be used here without enabling `producer-execution` and its process dependencies); a parity test holds the two readers to one set of verdicts. | Architecture | Test (TC-208) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-028-AC-1 | A list whose files match path, size and digest returns an empty finding list, including the empty list and an empty file; a reference index of 3 under base `/raw_evidence` reports at `/raw_evidence/3/...`, and an empty base yields `/3/...`. | Test (TC-206) |
| FR-028-AC-2 | Absolute, `..`, empty, backslash, control-character, `.`, trailing-slash, double-slash and non-normalized paths and a path through a symlinked directory component (pointing inside and outside the root) each yield `PathInvalid` and read nothing outside the root. | Test (TC-206) |
| FR-028-AC-3 | A missing file or component, a symlinked final component, a directory, a FIFO, an over-ceiling file (lowered ceiling seam), and an unreadable file (mode 000, skipped where running as root) yield `Missing`, `NotRegular`, `NotRegular`, `NotRegular`, `TooLarge` and `Unreadable` respectively; a short read (a file truncated during the read, by seam) yields `Unreadable`. | Test (TC-207) |
| FR-028-AC-4 | A wrong size yields `SizeMismatch` at `/size_bytes`, a wrong digest yields `DigestMismatch` at `/digest` with `expected` and `observed` in `sha256:<hex>` form, and one reference wrong in both yields both, size first. | Test (TC-206) |
| FR-028-AC-5 | A bare-hex digest, an uppercase digest, a short digest and the empty string yield `MalformedDigest`; `blake3:...` and `sha512:...` yield `UnknownAlgorithm`; each still has its size checked, and a good digest with a bad path yields only the path finding. | Test (TC-206) |
| FR-028-AC-6 | A list of at least 10 references with defects in 6 of them returns exactly those findings in reference-index then kind order regardless of the input order of independent runs; the result serializes to the same bytes on repeated runs; every finding is a typed struct (a serde round trip through the type holds). | Test (TC-207) |
| FR-028-AC-7 | An invalid root (absent, a file), an invalid base pointer, a ceiling of 0 or above 1 GiB and 65,537 references each return a typed request error and no findings; a root that is itself reached through a symlink is accepted. | Test (TC-207) |
| FR-028-AC-8 | The verifier and the module's bounded reader agree with `ContentDigest::of_file` on the same absent, symlink, directory, over-ceiling and normal cases, and the verifier writes nothing (the root's tree is byte-identical before and after). | Test (TC-208) |
| FR-028-AC-9 | Every fixture in the tests is fictional (generated file names and bytes under a temporary root); no operational, external or retained-evidence data is committed. | Inspection |

## Dependencies

- **Upstream**: FR-014, FR-027 (shared module and hardened reader).
- **Downstream**: consumers that re-verify retained raw evidence before promotion.

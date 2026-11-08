---
id: FR-032
title: "Verify a declared change scope against the candidate change footprint"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: "references"
---

# FR-032: Verify a declared change scope against the candidate change footprint

## Description

Engineering Assurance SHALL compare a declared change scope with the set of
repository paths a candidate actually changes, and SHALL report whether the
declaration covers the whole change. A consumer that seals a declared scope
(for example the scope of a release candidate's impact snapshot) uses the
result instead of asserting completeness by hand: a declaration that omits a
changed path is a false scope, and a footprint that cannot be fully observed is
incomplete, never complete.

## Inputs

- A declared scope: a list of normalized repository-relative paths, each naming
  a file or a directory.
- A change footprint: the set of changed paths between a base and a candidate
  revision, with a flag saying the set was cut short, or a statement that the
  footprint could not be observed.
- For the `scope-footprint` command, one request document
  `engineering-assurance.scope-footprint-request/v1` with the members
  `protocol`, `repository` (absolute directory path), `base`, `candidate`
  (revisions) and `scope`.

## Outputs

- A verdict with `status` (`complete`, `false_scope` or `incomplete`),
  `uncovered_paths` (changed paths no scope entry covers), `unused_scope`
  (scope entries that cover no changed path), `truncated` and
  `footprint_available`.
- The command writes one line of JSON to stdout, the verdict with the member
  `protocol` equal to `engineering-assurance.scope-footprint-result/v1`, and
  exits with status 0 for every status, as FR-029 does for every closed state.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error for capability `scope-footprint` to stdout and its message to
  stderr, and exits with status 2. The codes are
  `scope_footprint_request_unreadable`, `scope_footprint_request_invalid`,
  `scope_footprint_scope_invalid`, `scope_footprint_revision_invalid` and
  `scope_footprint_changed_path_invalid`.

## Behavior

- A path SHALL be a normalized repository-relative POSIX path: non-empty, not
  absolute, without `.` or `..` or empty segments, without a backslash or
  control character (NUL included), and without a trailing slash. The check is
  the crate's existing normalized-relative-path rule; no second validator exists.
- A scope entry SHALL cover a changed path when the two are equal or the entry
  is a whole-segment prefix of the path, so `src/a` covers `src/a` and
  `src/a/b` but not `src/ab`.
- The verdict SHALL be `false_scope` when at least one changed path is
  uncovered, including when the footprint is also truncated, because an
  observed uncovered path is a definite violation.
- The verdict SHALL be `incomplete` when no changed path is uncovered and the
  footprint is truncated or unavailable, and `complete` otherwise.
- The verdict SHALL list `uncovered_paths` and `unused_scope` sorted and
  without duplicates. An unused scope entry SHALL NOT change the status; it is
  reported so a consumer can see an over-wide declaration, and only for a fully
  observed footprint (the list is empty when it is truncated or unavailable).
- The function SHALL refuse a scope with an invalid or duplicate entry, a scope
  over 4096 entries, a changed path that is not a normalized path, and a
  footprint over 100000 paths, before it computes a verdict. It SHALL treat the
  changed paths as a set.
- The command SHALL resolve each revision to a commit and compute the footprint
  with `git diff --name-status -z --find-renames` between the two commits
  (a direct two-tree comparison, so a caller wanting the candidate's own change
  passes the merge-base as `base`),
  listing both the old and the new path of a rename or copy. It SHALL refuse a
  revision that is empty, begins with `-`, or contains a control character or
  whitespace, before it starts git, and SHALL run git without the user's or the
  system's git configuration and without repository-selecting environment
  variables.
- The command SHALL report `incomplete` with `footprint_available` false when
  git cannot start, the repository or a revision cannot be resolved, git fails
  or times out, or its output cannot be parsed. It SHALL report `truncated`
  true when git output exceeds the 8 MiB capture bound or the path count
  exceeds the bound above, keeping the sorted paths it holds.
- The command SHALL NOT modify the repository, interpret file content, or
  decide that a change is acceptable.
- The pure function SHALL perform no I/O. It lives in the `scope-footprint`
  Cargo feature, which depends on `content-rights` (for the path rule), `serde`
  and `thiserror` only; the git adapter lives in the binary.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-032-CON-1 | A footprint that was cut short or could not be observed SHALL NOT produce `complete`. | Safety | Test (TC-234) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-032-AC-1 | A scope whose entries cover every changed path, by exact file entry and by directory prefix, yields `complete` with no uncovered path. | Test (TC-232) |
| FR-032-AC-2 | A changed path outside the scope, including a sibling that shares only a textual prefix with an entry, yields `false_scope` with that path sorted into `uncovered_paths`, also when the footprint is truncated. | Test (TC-233) |
| FR-032-AC-3 | A truncated footprint with no uncovered path and an unavailable footprint each yield `incomplete`, and neither yields `complete`. | Test (TC-234) |
| FR-032-AC-4 | A scope entry or changed path that is empty, absolute, contains `.`, `..` or an empty segment, a backslash, a NUL, or a trailing slash is refused, as are a duplicate scope entry and an over-bound scope or footprint. | Test (TC-235) |
| FR-032-AC-5 | Scope entries covering no changed path are listed in `unused_scope`, sorted, without altering the status. | Test (TC-236) |
| FR-032-AC-6 | On a real repository the command reports `complete` for an in-scope modify, add and delete, `false_scope` for an out-of-scope change, both paths of a rename, and `incomplete` with `footprint_available` false for an unknown revision and for a directory that is not a repository, each with exit 0. | Test (TC-237) |
| FR-032-AC-7 | An unreadable or non-conforming request, an invalid scope entry, and a revision beginning with `-` yield the matching machine error and exit status 2, and do not start git. | Test (TC-238) |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the Cargo
  feature discipline; [FR-028](./FR-028-shared-cli-output.md) for stdout and
  machine-error encoding; [FR-029](./FR-029-producer-execute-command.md) for the
  exit-code convention.
- **Downstream**: the TL stable-release impact snapshot (TL-63) that must stop
  forcing `complete` to false.

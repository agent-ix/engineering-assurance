---
id: FR-046
title: "Declare a Baseline as a named set of configuration items at one git commit"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "references"
---

# FR-046: Declare a Baseline as a named set of configuration items at one git commit

## Description

Engineering Assurance SHALL provide a `Baseline` archetype, installed with a
closed frontmatter schema and a skeleton as the other Engineering Assurance
archetypes are, that names a set of configuration items. A configuration item
is a spec artifact, a SOUP record or an evidence file, identified by its
repository path. A Baseline is read at exactly one git commit, and git's commit
hash is its identity: the same Baseline definition read at a different commit
is a different baseline. The archetype records no digest, hash, version counter
or file listing of its own, because the commit already fixes every byte of
every item.

The `Baseline` archetype is unrelated to the `baseline` member and the
`Baseline` value of a MeasurementPlan decision rule
([FR-021](./FR-021-measurement-plan-decision-rule.md)), which mean a reference
measurement value; the two live in different namespaces (an artifact `type`
against a MeasurementPlan member) and share nothing.

The Baseline file cannot name the commit that contains it, so the commit is
never a member of the Baseline. A release record
([FR-047](./FR-047-release-record-and-signed-release-tag.md)) names the commit
at which a Baseline is read.

Resolution has one entry point: a function in the release adapter,
`resolve_baseline(repository, commit, path)`, called by `release-verify`
(FR-047) and by the release-decision workflow (FR-048). It runs git, so it lives
in the adapter, never in the process-free library. It has no command of its
own.

Out of scope: the content of the items (their own archetypes and the Quoin
SOUP and evidence archetypes own it), computing evidence or SOUP status, any
git write, and any digest or manifest of the items.

## Inputs

- One Baseline document with frontmatter `id`, `title`, `type: Baseline`,
  `name` (a non-empty human name) and `items`, a list of one to 1024 objects with
  `path` (a repository-relative path: no `..` or `.` segment, no leading `/`,
  no empty segment, no glob character, no control character, and no segment
  starting with a space) and
  `kind`, one of `spec_artifact`, `soup_record`, `evidence`.
- For resolution: a repository directory, a git commit hash and the
  repository-relative path of the Baseline file.

## Outputs

- A schema-valid Baseline document, or a refusal naming the member at fault.
- For resolution: the Baseline `name`, the commit hash as given by git, and per
  item its `path`, `kind` and `present` or `absent` at that commit; or, when
  the Baseline file itself is absent or invalid at that commit, the error
  `baseline_file_absent` or `baseline_file_invalid` and no item list.

## Behavior

- The archetype SHALL refuse a member other than those named under
  Inputs.
- The archetype SHALL refuse a `kind` outside the three named.
- The archetype SHALL refuse more than 1024 items, so that one git call tests
  every item path.
- When a Baseline is resolved, the resolution SHALL read the Baseline file
  from the supplied commit with git object access.
- When a Baseline is resolved, the resolution SHALL test each item path with
  git at the supplied commit.
- The resolution SHALL report the commit hash that git resolves.
- The resolution SHALL NOT report a hash it computed.
- If a Baseline lists the same `path` more than once, then the resolution SHALL
  report `baseline_file_invalid`. Frontmatter schemas and the module's lint rules
  cannot compare two list items by a member, so the archetype refuses an identical
  repeated item and the resolution refuses a repeated `path`.
- If an item path is absent at the commit, then the resolution SHALL report
  that path `absent`.
- If an item path is absent at the commit, then the resolution SHALL NOT
  substitute another commit or the working tree.
- The resolution SHALL NOT read the working tree.
- The resolution SHALL NOT write to the repository.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | A Baseline with a name and items of each of the three kinds validates, and the skeleton validates as authored. | Test |
| FR-046-AC-2 | A Baseline with an unknown member, an empty `items`, more than 1024 items, an absolute path, a path with `..`, a glob, or an unknown `kind` is refused with the member named. | Test |
| FR-046-AC-3 | Resolving one Baseline at two commits whose item contents differ reports the two commit hashes git gives, and the result has no member that Engineering Assurance computed from file content. | Test |
| FR-046-AC-4 | An item path deleted at the resolved commit but present in the working tree is reported `absent`; an item present at the commit but modified in the working tree is reported `present`. | Test |
| FR-046-AC-5 | Resolution leaves the repository's refs, index and working tree unchanged. | Test |
| FR-046-AC-6 | A Baseline file absent at the commit yields `baseline_file_absent`, and one whose frontmatter is invalid or that lists a `path` twice or lists more than 1024 items yields `baseline_file_invalid`; neither yields an item list. | Test |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the versioned
  machine boundary the resolution result is written through.
- **Downstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md).

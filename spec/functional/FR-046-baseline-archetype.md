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
frontmatter schema and a skeleton as the other Engineering Assurance archetypes
are, that names a set of configuration items. A configuration item is a spec
artifact, a SOUP record or an evidence file, identified by its repository path.
A Baseline is read at exactly one git commit, and git's commit hash is its
identity: the same Baseline definition read at a different commit is a
different baseline. The archetype records no digest, hash, version counter or
file listing of its own, because the commit already fixes every byte of every
item.

The Baseline file cannot name the commit that contains it, so the commit is
never a member of the Baseline. A release record
([FR-047](./FR-047-release-record-and-signed-release-tag.md)) names the commit
at which a Baseline is read.

Out of scope: the content of the items (their own archetypes and the Quoin
SOUP and evidence archetypes own it), computing evidence or SOUP status, any
git write, and any digest or manifest of the items.

## Inputs

- One Baseline document with frontmatter `id`, `title`, `type: Baseline`,
  `name` (a non-empty human name) and `items`, a non-empty list of objects with
  `path` (a repository-relative path, no `..`, no leading `/`, no glob) and
  `kind`, one of `spec_artifact`, `soup_record`, `evidence`.
- A git commit hash and a repository directory, when a Baseline is resolved.

## Outputs

- A schema-valid Baseline document, or a refusal naming the member at fault.
- For resolution, the Baseline `name`, the commit hash as given by git, and per
  item its `path`, `kind` and `present` or `absent` at that commit.

## Behavior

- The archetype SHALL reject, as invalid, any frontmatter member named
  `commit`, `digest`, `hash`, `sha`, `checksum` or `files`.
- The archetype SHALL reject a duplicate `path` and a `kind` outside the three
  named.
- When a Baseline is resolved, the resolution SHALL read the Baseline file and
  test each item path with git at the supplied commit and SHALL report the
  commit hash that git resolves, never a hash it computed.
- If a Baseline file or an item path is absent at the commit, then the
  resolution SHALL report that path `absent` and SHALL NOT substitute another
  commit or the working tree.
- The resolution SHALL read the commit only through git object access and SHALL
  NOT read the working tree.
- The resolution SHALL NOT write to the repository.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | A Baseline with a name and items of each of the three kinds validates, and the skeleton validates as authored. | Test |
| FR-046-AC-2 | A Baseline carrying any of `commit`, `digest`, `hash`, `sha`, `checksum` or `files` is refused with the member named. | Test |
| FR-046-AC-3 | A Baseline with an empty `items`, a duplicate `path`, an absolute path, a path with `..`, a glob, or an unknown `kind` is refused with the member named. | Test |
| FR-046-AC-4 | Resolving one Baseline at two commits whose item contents differ reports the two commit hashes git gives, and no member of either result is a digest computed by Engineering Assurance. | Test |
| FR-046-AC-5 | An item path deleted at the resolved commit but present in the working tree is reported `absent`; an item present at the commit but modified in the working tree is reported `present`. | Test |
| FR-046-AC-6 | Resolution leaves the repository's refs, index and working tree unchanged. | Test |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the versioned
  machine boundary the resolution result is written through.
- **Downstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md).

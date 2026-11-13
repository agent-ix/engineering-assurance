---
id: FR-079
title: "Admit the organization's attestation schema prefix in the content-rights URL check"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
---

# FR-079: Admit the organization's attestation schema prefix in the content-rights URL check

## Description

Tracked by Linear EA-97 and EA-98. The predicate types of [FR-058](./FR-058-approval-statement-and-predicate.md) and [FR-072](./FR-072-change-record-predicate.md) are URIs under the organization's own schema host. The content-rights check rejects an external URL that is not on its allowlist, so a literal predicate type cannot be written in code, tests or documents until the check admits it. Once it does, the code and tests write the literal approval and change-record predicate URIs in place of the spelled-out form this specification uses. Admitting the prefix adds no hosting requirement: the URIs remain identity strings.

## Behavior

- The content-rights check SHALL admit a URL whose scheme is `https`, whose host is exactly `schemas.agent-ix.ai` and whose path begins with `/attestation/`, in every file kind it scans.
- The content-rights check SHALL reject a URL on a different host that contains the admitted text.
- The content-rights check SHALL reject a URL with the same host and a path outside `/attestation/`.
- The content-rights check SHALL reject a URL with the same host over a scheme other than `https`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-079-AC-1 | The two literal predicate URIs pass the check in a Markdown file and in a Rust source file. | Test |
| FR-079-AC-2 | A lookalike host (the admitted host as a subdomain of another domain, or followed by a credential separator) is rejected with the unapproved external URL finding. | Test |
| FR-079-AC-3 | The same host with the path `/other/` and with the `http` scheme is rejected. | Test |
| FR-079-AC-4 | The full tree check passes on a tree that contains the literal URIs. | Test |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md), [FR-072](./FR-072-change-record-predicate.md).

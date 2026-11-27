---
id: FR-091
title: "Install pinned external test tools from a reviewed registry"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-079"
    type: "references"
---

# FR-091: Install pinned external test tools from a reviewed registry

## Description

Conformance tests may need a fixed external command-line tool. Engineering
Assurance keeps the approved source and integrity metadata in a repository-owned
registry so the content-rights checker can distinguish a reviewed exact pin
from an unregistered external download.

## Behavior

The registry SHALL be a closed JSON document with protocol
`engineering-assurance.test-tool-download-registry/v1`. Every entry SHALL
identify a tool, source repository, version, exact HTTPS release-asset URL,
lowercase SHA-256 digest, SPDX license identifier, rights owner, purpose, and
the repository-relative consumer paths. A source URL SHALL name the declared
GitHub repository and version. An entry with missing, duplicate, malformed, or
unknown fields SHALL be rejected.

The content-rights check SHALL admit a registered source URL only in the
registry or one of the entry's listed consumer paths. A direct release URL at
another path, an unregistered version, or a consumer line that does not carry
the registry digest SHALL be rejected. The installation workflow SHALL read
the version, URL, and digest from the registry and verify the downloaded bytes
before installing the executable.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | The registry accepts the reviewed Syft 1.57.1 Linux x86-64 entry and rejects missing or unknown fields, a duplicate entry, a malformed source URL, an invalid version, or a non-lowercase SHA-256 digest. | Test |
| FR-091-AC-2 | A registered source URL with its registered digest is admitted only at a listed consumer path; a changed URL version, digest, or unlisted path is rejected. | Test |
| FR-091-AC-3 | The Syft installer reads its version, source URL, and SHA-256 from the registry, checks the archive before extraction, installs only the pinned `syft` executable, and the customer guide documents the install command and destination. | Test |


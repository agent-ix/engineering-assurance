---
id: IU-001
title: Engineering Assurance intended use
type: IntendedUse
tool: engineering-assurance
relationships: []
---
# Engineering Assurance intended use

## Purpose

Engineering Assurance provides a Rust library and command-line interface for
collecting, checking, and publishing engineering evidence. It helps repository
maintainers inventory existing assurance context, execute configured evidence
producers, evaluate declared measurements and assurance profiles, verify signed
approvals and release records, and retain provenance for changes and releases.
It also supplies resumable workflows that leave terminal decisions to named
people.

## Records produced

| Record kind | Command that writes it |
|---|---|
| Onboarding and evaluation reports | `engineering-assurance onboard-report`, `engineering-assurance agent-evals` |
| Measurement, qualification, and assurance results | `engineering-assurance measurement-verify`, `engineering-assurance profile-verify`, `engineering-assurance integration-evidence` |
| Signature, approval, scope, release, and receipt verification results | `engineering-assurance signature-verify`, `engineering-assurance approval-verify`, `engineering-assurance scope-footprint`, `engineering-assurance release-verify`, `engineering-assurance receipt-verify` |
| Change-assurance records and signed approval or release tags | `engineering-assurance approval-tag`, `engineering-assurance release-decision` |
| Release attestations, receipts, and published evidence records | `engineering-assurance attest`, `engineering-assurance receipt-record`, `engineering-assurance record-sink` |
| Producer execution results and retained producer artifacts | `engineering-assurance producer-execute` |

## Who relies on them

Repository maintainers and engineering teams use the outputs to inspect
provenance, test and measurement evidence, policy checks, approvals, release
state, and retained records. A named decision owner or reviewer may use a
verified record as input to a separate engineering or release decision.

## Limits

Engineering Assurance is an evidence and workflow tool, not a compliance
oracle. Its outputs do not establish that a product, process, repository, or
clinical use is compliant, safe, effective, or accepted. It does not assign
feature-risk ratings or decide whether a person should accept a risk. Results
depend on the requested operation, repository contents, configured policies,
producer tools, and available signature identities. People remain responsible
for interpreting evidence and making terminal decisions.

## Operating context

The Rust library and CLI run in local development environments and CI jobs.
Commands may read repository files, Git objects, workflow and policy
configuration, module schemas, and caller-supplied evidence. Depending on the
operation, the CLI invokes `git`, `ssh-keygen`, `gitsign`, `gh`, `ix-flow`,
`quoin`, or `quire`; interactive signing may use `/usr/bin/script`. Producer
execution uses the executable selected in its request; supported repository
workflows use tools such as `syft`, `osv-scanner`, `pandoc`, and `typst`. The
public receipt sink uses `curl` to submit a configured HTTPS record. Optional
commands may be unavailable, and the tool reports that condition rather than
claiming the operation succeeded. Workflows that use remote services require
the caller's network and credentials; local verification operations do not
claim to independently validate the remote service or host configuration.

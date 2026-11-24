---
id: SUR-001
title: Engineering Assurance receipt evidence suite registry
type: SuiteRegistry
---

# Engineering Assurance receipt evidence suite registry

## Suites

| ID | Name | Command | Tool | Evidence Kind |
|---|---|---|---|---|
| SUITE-001 | Receipt workflow contract gate | `cargo test --locked --features full --test receipt_workflow_contract receipt_workflow_jobs_have_exact_permissions_and_keep_code_out_of_record_job -- --exact` | Cargo via engineering-assurance 0.10.0 | Integration |
| SUITE-004 | Real-tool integration checks | `make real-tool-test` | real-tool-test lane | Integration |

Quoin records the actual process result from the protected receipt-gate policy.
The JUnit adapter input is derived from that process result and carries the
trace identifiers declared on the executed Rust test. The registry does not
claim that this one focused gate represents the repository's complete CI suite.

`SUITE-004` records each applicable ignored integration check against its real
tool. Each entry retains the exact tool version and result-affecting inputs;
the OSV-Scanner entry includes the caller-supplied `osv_mirror_date`. A required
tool or input that is unavailable is recorded with `outcome: error` and
`config.execution_state: not-executed`, never as a pass. This local lane is
independent of release CI.

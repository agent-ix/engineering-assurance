---
id: SR-2026112016223475846355060
title: PLAT-1822 PR322 code-review
type: SpecReview
analysis: code-review
scope: agent-ix/engineering-assurance; .github/workflows/change-receipt.yml; tests/receipt_workflow_contract.rs
review_set: subset
---

## Summary

Ticket: PLAT-1822. PR322 head e305774e7b990b90d5c3b1470faca7082da71a1a.

Rust-review lane included. Reviewed the complete two-file diff, protected policy and surrounding workflow, request serialization, process environment clearing and descriptor-based execution, and existing test trace bindings. Rustup resolution runs in source cwd, matching the installed toolchain file. Cargo resolves to the real toolchain executable, and RUSTUP_TOOLCHAIN survives the producer environment allowlist so rustc lookup works from a descriptor-based cwd. Non-Cargo gates retain their old behavior. Policy remains root cargo test --locked. No dependency revision moves, new functions or production Rust functions, duplicate logic, weakened gate, or new untraced test. No applicable AssuranceProfile is declared in spec/.

Executed the actual changed Python AST statements using real Rustup and an external subprocess capture seam. Serialized request contains the real toolchain Cargo path and active toolchain. Executed that Cargo with a rewritten argv[0] and --version successfully; rustc --version from a temporary directory with exactly the request environment resolved 1.98.1. This verifies resolution and compiler lookup; it is not an end-to-end receipt run. Dispatching lead reports the focused receipt contract test passed 1/1 through locked-build; no redundant Cargo rerun or broad tests were launched. git diff --check passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for this PR diff; no findings requiring disposition. CI actions, Python and JavaScript analyses passed; Rust analysis remains pending at review time; review-gate skipped.

## Coverage

```yaml
scope:
- id: FR-070-AC-4
  path: spec/functional/FR-070-release-and-receipt-workflow-files.md
  role: examined
  excerpt: 'The `gates` job holds only `contents: read`, the `record` job runs no
    step that executes a repository file or a command from the receipt policy, and
    the `notes` job holds no `id-token`.'
- id: FR-070-AC-7
  path: spec/functional/FR-070-release-and-receipt-workflow-files.md
  role: examined
  excerpt: The `gates` job declares no `secrets` and no `environment`, and its `runs-on`
    is a GitHub-hosted label or an ephemeral runner label.
- id: FR-070-AC-15
  path: spec/functional/FR-070-release-and-receipt-workflow-files.md
  role: examined
  excerpt: The receipt workflow passes serialized JSON request content to `producer-execute
    --request`; it does not pass the temporary request file path as the request value.
- id: .github/workflows/change-receipt.yml:Cargo-resolution
  path: .github/workflows/change-receipt.yml
  role: examined
  excerpt: "executable = subprocess.run([\"rustup\", \"which\", \"cargo\"], cwd=source,\n\
    \                            check=True, capture_output=True, text=True).stdout.strip()"
- id: tests/receipt_workflow_contract.rs:Cargo-contract
  path: tests/receipt_workflow_contract.rs
  role: examined
  excerpt: 'assert_eq!(policy["gates"][0]["argv"][1], "test");

    assert_eq!(policy["gates"][0]["argv"][2], "--locked");'
- id: src/producer_execution.rs:build_command
  path: src/producer_execution.rs
  role: context_only
  excerpt: let mut command = Command::new(&request.producer.executable);
```

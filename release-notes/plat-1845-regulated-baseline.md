# PLAT-1845 regulated baseline

This change makes the request contracts discoverable with `--schema request`
for `release-history`, `release-decision`, `release-verify`, and `approval-tag`.
Each command prints its own JSON Schema and does not execute the request. The
schemas describe the actual closed request objects, including their protocol
identifiers, required members, optional members and enumerated values.

The guide at `docs/regulated-assurance-path.md` now gives the schema command
for each of these four capabilities and explains how to submit a request.

## Known limits and anomalies

- The main CI workflow is dispatch-only. A push or pull request does not run
  that workflow automatically; this change does not alter workflow triggers.
- Tag receipts record signed evidence about a pushed tag. They are not EA-139
  release attestations and do not make the release attestation workflow run.
- Request schemas describe JSON structure and types. Command-specific
  semantic checks, filesystem state, git state, and workflow state remain
  validated when the request is executed.

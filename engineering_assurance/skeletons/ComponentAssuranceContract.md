---
id: CAC-001
title: Juniper retention monitor contract
type: ComponentAssuranceContract
status: proposed
owner: juniper-operations-owner
kind: deterministic
responsibility: detect accepted requests without a retained result
inputs: [accepted request identity, retained result identity]
outputs: [matched outcome, missing-result alert]
invariants: [one terminal classification per accepted request]
failure_behaviors: [emit an unhealthy state when either input stream is unavailable]
version_pins:
  monitor: fictional-monitor-v1
controls:
  surfaces: [disable switch, alert route, health endpoint]
  fallback: stop accepting new requests
  abstention: classify the outcome as unknown
  escalation: notify the accountable operator
isolation: run outside the request-processing component
replacement: preserve input and output semantics when replacing the monitor
relationships:
  - target: ix://example/juniper/AP-001
    type: references
---

# Juniper retention monitor contract

## Component Boundary

The monitor observes accepted-request and retained-result identities. It does
not modify request processing or storage.

## Required Behavior

Every accepted request reaches exactly one matched, missing, or unknown
terminal state within the declared observation window.

## Failure Handling

Unavailable or inconsistent inputs produce an unhealthy state and an unknown
classification rather than a successful result.

## Controls

Operators can disable intake, inspect health, route alerts, and invoke the
declared fallback.

## Replacement

A replacement must demonstrate the same inputs, outputs, terminal states,
failure behavior, and operator controls before activation.

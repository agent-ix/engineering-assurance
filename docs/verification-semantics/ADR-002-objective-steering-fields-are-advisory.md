# ADR-002: Objective steering fields are advisory, never a gate

Status: accepted for specification and implementation (FR-026, Linear
PLAT-967, epic PLAT-956)

## Context

PLAT-956 aims EA at steering improvement, not only proving it happened.
Adam Chlipala's post (cited from the epic) gives each objective a bounty and
discounts that reward by how long delivery takes, so a system never polishes
forever. EA had no way to say which improvement matters more, or sooner: a
`MeasurementPlan`'s `objective` stated only a direction and an optional bound
(FR-020), with no notion of relative priority, urgency, or cost.

quoin's `MP-227` establishes the pattern this decision follows: a plan whose
whole `Comparison and Enforcement` section reads "This plan assigns no
verdict and no bar" and whose `Interpretation` states "A measure only. A slow
run must never fail a gate." MP-227 measures Jev lens cost, latency, and
throughput to decide *which lenses run on every commit versus on demand* --
a steering decision made by a human or a scheduler reading the numbers, never
a decision the measurement itself enforces.

## Decision

The `objective` block gains three optional steering fields:

- `weight` -- this objective's relative value against the project's other
  objectives, in units the plan's body states;
- `value_half_life` -- how quickly the value of improving this objective
  decays, so sooner ranks higher, in units (e.g. days) the plan's body
  states;
- `budget` -- time, tokens, or compute allowed per attempt at this
  objective, in units the plan's body states, used only to normalize
  comparisons across objectives with different costs per attempt.

**These fields are advisory only. They never gate anything, and they never
feed a decision rule.** This is enforced structurally, not just by
convention:

1. `statistical_design.decision_rule` (FR-021) remains the only part of a
   `MeasurementPlan` that is evaluated. `DecisionRule::holds` takes an
   estimate and an optional baseline value; it has never taken an
   `Objective` and does not gain one now. A decision rule's verdict is
   therefore structurally unable to read the steering fields, whatever they
   contain -- including adversarial values such as `weight: 0` or
   `budget: 0`. `FR-026-AC-4` / `TC-169` checks this directly: two otherwise
   identical plans, one carrying extreme steering values and one carrying
   none, are asserted to reach the identical Gate/Ratchet/Target verdict
   for the same estimate.
2. The steering fields are excluded from the plan's measurement definition.
   `direction` and `bound` remain part of it (an edit to either still
   requires a new `definition_version`, per FR-020), but `weight`,
   `value_half_life`, and `budget` do not: `Objective::definitional()`
   returns only `(direction, bound)`, and the FR-020 definition-change check
   compares that projection rather than the whole `Objective`. Changing a
   steering field alone is therefore never reported as a definition change.
   This mirrors FR-024's exclusion of `negative_controls` from the
   definition: both are declarations *about* the measurement (priority,
   guarded scenarios) rather than declarations of *how the number is
   computed*.

## Consequences

- A future portfolio-ranking consumer may read `weight`, `value_half_life`,
  and `budget` to order objectives across a project's several
  `MeasurementPlan`s -- e.g. to decide which improvement to attempt next.
  That consumer is out of scope for FR-026 and would be specified
  separately under PLAT-956; this decision only fixes that such a consumer
  can never turn these fields into a pass/fail signal, because no gate
  computation reads them.
- Because the fields are excluded from the measurement definition, a plan
  author can retune priority (raise a `weight`, shorten a `value_half_life`
  as a deadline approaches) without bumping `definition_version` and without
  starting a new, incomparable result series. Only a change to `direction`
  or `bound` -- what the metric is actually judged against -- does that.
- The schema enforces the finite-and-non-negative shape it can express
  (`minimum: 0` for `weight` and `budget`, `exclusiveMinimum: 0` for
  `value_half_life`); the Rust `Objective` type additionally refuses NaN and
  infinity, which JSON Schema has no keyword for, following the same split
  FR-020 already uses for `bound`.

## Alternatives considered

- **A separate top-level `steering` block**, sibling to `objective`, instead
  of nesting the fields inside `objective`. Rejected: the epic and PLAT-967
  both describe these as fields "on the objective," and nesting keeps
  `weight`/`value_half_life`/`budget` next to the `direction`/`bound` they
  qualify, which is what a reader wants when judging how much an objective
  matters.
- **Making the steering fields part of the measurement definition**, like
  `direction` and `bound`. Rejected: that would force a `definition_version`
  bump -- and a fresh, incomparable result series -- every time a project
  re-weighted its priorities, which is exactly the "polishes forever versus
  reprioritizes freely" tension PLAT-956 exists to resolve. Advisory fields
  that force a new series on every edit are not advisory in practice.
- **A single `priority` scalar** instead of three fields. Rejected: weight,
  decay, and budget answer three different questions (how much, how soon,
  how expensive) that a single scalar cannot separate, and Chlipala's model
  -- which the epic cites directly -- keeps them distinct for the same
  reason.

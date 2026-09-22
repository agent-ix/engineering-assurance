// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `MeasurementPlan` definition types owned by Engineering Assurance (FR-020,
//! FR-021).
//!
//! This module owns the typed form of a `MeasurementPlan`'s optional `objective`
//! block, the check that an objective edit came with a new
//! `definition_version`, and the closed `statistical_design.estimator` and
//! `statistical_design.decision_rule` vocabulary with the rule's evaluation.
//! It performs no filesystem, process, environment, network, clock, or
//! persistence access: callers parse plan frontmatter and pass the relevant
//! fields in, and compute any baseline value themselves.
//!
//! The wire names of [`Direction`], [`Estimator`], [`Comparator`], and
//! [`Baseline`] are the matching enums of
//! `engineering_assurance/schemas/measurement-plan-frontmatter.schema.json`;
//! tests assert each pair of sets is equal.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Which way a `MeasurementPlan`'s metric is supposed to move.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// A larger metric value is better.
    Higher,
    /// A smaller metric value is better.
    Lower,
    /// The metric is expected to be exactly zero; any non-zero value is a
    /// deviation.
    Zero,
    /// The metric is expected to reach the objective's bound, which is then
    /// required.
    Target,
}

impl Direction {
    /// Every direction, in declaration order.
    pub const ALL: [Self; 4] = [Self::Higher, Self::Lower, Self::Zero, Self::Target];

    /// The frontmatter wire name of this direction.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Higher => "higher",
            Self::Lower => "lower",
            Self::Zero => "zero",
            Self::Target => "target",
        }
    }

    /// Whether an objective with this direction must carry a bound.
    #[must_use]
    pub const fn requires_bound(self) -> bool {
        match self {
            Self::Target => true,
            Self::Higher | Self::Lower | Self::Zero => false,
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

/// Why an objective is invalid.
#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ObjectiveError {
    /// A `target` objective carried no bound to reach.
    #[error("objective direction `target` requires a bound")]
    TargetWithoutBound,
    /// The bound was NaN or infinite.
    #[error("objective bound {bound} is not a finite number")]
    NonFiniteBound {
        /// The refused bound.
        bound: f64,
    },
}

/// A validated `MeasurementPlan` objective: a direction and an optional finite
/// bound, where a `target` direction always has a bound.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "ObjectiveFields", into = "ObjectiveFields")]
pub struct Objective {
    direction: Direction,
    bound: Option<f64>,
}

impl Objective {
    /// Build an objective from its direction and optional bound.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectiveError::TargetWithoutBound`] when `direction` is
    /// [`Direction::Target`] and `bound` is absent, and
    /// [`ObjectiveError::NonFiniteBound`] when `bound` is NaN or infinite.
    pub fn new(direction: Direction, bound: Option<f64>) -> Result<Self, ObjectiveError> {
        if let Some(value) = bound
            && !value.is_finite()
        {
            return Err(ObjectiveError::NonFiniteBound { bound: value });
        }
        if direction.requires_bound() && bound.is_none() {
            return Err(ObjectiveError::TargetWithoutBound);
        }
        Ok(Self { direction, bound })
    }

    /// The direction the metric is supposed to move.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// The threshold the plan measures against, when stated.
    #[must_use]
    pub const fn bound(&self) -> Option<f64> {
        self.bound
    }
}

/// The closed wire shape of an `objective` block, before validation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectiveFields {
    direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bound: Option<f64>,
}

impl TryFrom<ObjectiveFields> for Objective {
    type Error = ObjectiveError;

    fn try_from(fields: ObjectiveFields) -> Result<Self, Self::Error> {
        Self::new(fields.direction, fields.bound)
    }
}

impl From<Objective> for ObjectiveFields {
    fn from(objective: Objective) -> Self {
        Self {
            direction: objective.direction,
            bound: objective.bound,
        }
    }
}

/// The parts of one `MeasurementPlan`'s frontmatter that identify its
/// measurement definition for the objective check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanDefinition<'a> {
    /// The plan's `definition_version`, when present.
    pub definition_version: Option<&'a str>,
    /// The plan's `objective`, when present.
    pub objective: Option<Objective>,
}

/// An objective that was added, removed, or changed while the plan's
/// `definition_version` stayed the same.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveChangedWithoutVersionBump {
    /// The `definition_version` shared by both plan revisions.
    pub definition_version: Option<String>,
    /// The objective before the edit.
    pub before: Option<Objective>,
    /// The objective after the edit.
    pub after: Option<Objective>,
}

/// Report an objective edit between two revisions of one plan that did not
/// come with a genuine `definition_version` bump.
///
/// Returns `None` when the objective is unchanged, or when `after`'s
/// `definition_version` is present and differs from `before`'s (a genuine
/// bump). Clearing `definition_version` (`after` is `None`) is not a bump:
/// it is treated the same as leaving the version unchanged, so an objective
/// edit alongside a removed `definition_version` still yields a finding.
#[must_use]
pub fn objective_change_without_version_bump(
    before: &PlanDefinition<'_>,
    after: &PlanDefinition<'_>,
) -> Option<ObjectiveChangedWithoutVersionBump> {
    if before.objective == after.objective {
        return None;
    }
    let genuinely_bumped = match after.definition_version {
        None => false,
        Some(after_version) => Some(after_version) != before.definition_version,
    };
    if genuinely_bumped {
        return None;
    }
    Some(ObjectiveChangedWithoutVersionBump {
        definition_version: before.definition_version.map(str::to_owned),
        before: before.objective,
        after: after.objective,
    })
}

/// How a `MeasurementPlan`'s metric is computed from its collected population
/// (`statistical_design.estimator`).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Estimator {
    /// Matching items divided by examined items, in `[0, 1]`.
    Proportion,
    /// The number of matching items.
    Count,
    /// The arithmetic mean of per-item or per-repetition values, including a
    /// weighted mean the plan's Measure Definition states.
    Mean,
    /// The middle per-item value.
    Median,
    /// One total divided by another, unbounded.
    Ratio,
}

impl Estimator {
    /// Every estimator, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::Proportion,
        Self::Count,
        Self::Mean,
        Self::Median,
        Self::Ratio,
    ];

    /// The frontmatter wire name of this estimator.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Proportion => "proportion",
            Self::Count => "count",
            Self::Mean => "mean",
            Self::Median => "median",
            Self::Ratio => "ratio",
        }
    }
}

impl fmt::Display for Estimator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

/// How a decision rule compares the estimate with its reference: the rule
/// holds when `estimate <comparator> reference`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Comparator {
    /// `estimate > reference`.
    Gt,
    /// `estimate >= reference`.
    Ge,
    /// `estimate < reference`.
    Lt,
    /// `estimate <= reference`.
    Le,
    /// `estimate == reference`.
    Eq,
}

impl Comparator {
    /// Every comparator, in declaration order.
    pub const ALL: [Self; 5] = [Self::Gt, Self::Ge, Self::Lt, Self::Le, Self::Eq];

    /// The frontmatter wire name of this comparator.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Gt => "gt",
            Self::Ge => "ge",
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Eq => "eq",
        }
    }

    /// Whether `estimate <self> reference` holds.
    #[must_use]
    #[expect(
        clippy::float_cmp,
        reason = "`eq` is exact equality by definition (FR-021); a tolerance would be a different comparator"
    )]
    pub fn holds(self, estimate: f64, reference: f64) -> bool {
        match self {
            Self::Gt => estimate > reference,
            Self::Ge => estimate >= reference,
            Self::Lt => estimate < reference,
            Self::Le => estimate <= reference,
            Self::Eq => estimate == reference,
        }
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

/// A reference value a decision rule computes at evaluation time rather than
/// stating in the plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Baseline {
    /// The score of the best constant answer per answer family, computed from
    /// the corpus.
    ConstantPredictor,
    /// The plan's metric in the collection this result is compared against
    /// under the same `definition_version`.
    PriorCollection,
    /// The best value of the plan's metric across every accepted collection
    /// under the same `definition_version`.
    BestSeen,
}

impl Baseline {
    /// Every baseline, in declaration order.
    pub const ALL: [Self; 3] = [Self::ConstantPredictor, Self::PriorCollection, Self::BestSeen];

    /// The frontmatter wire name of this baseline.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::ConstantPredictor => "constant-predictor",
            Self::PriorCollection => "prior-collection",
            Self::BestSeen => "best-seen",
        }
    }
}

impl fmt::Display for Baseline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

/// What a decision rule compares the estimate against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuleReference {
    /// A fixed, finite value stated in the plan.
    Threshold(f64),
    /// A value computed at evaluation time, plus a finite signed margin
    /// (`0.0` when the plan states none).
    Baseline {
        /// Which baseline to compute.
        baseline: Baseline,
        /// Added to the baseline value to give the reference.
        margin: f64,
    },
}

/// Why a decision rule is invalid.
#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum DecisionRuleError {
    /// Neither `threshold` nor `baseline` was stated.
    #[error("decision rule requires exactly one of `threshold` or `baseline`; neither is present")]
    MissingReference,
    /// Both `threshold` and `baseline` were stated.
    #[error("decision rule requires exactly one of `threshold` or `baseline`; both are present")]
    ThresholdAndBaseline,
    /// A `margin` was stated without a `baseline`.
    #[error("decision rule `margin` is allowed only with `baseline`")]
    MarginWithoutBaseline,
    /// The threshold was NaN or infinite.
    #[error("decision rule threshold {threshold} is not a finite number")]
    NonFiniteThreshold {
        /// The refused threshold.
        threshold: f64,
    },
    /// The margin was NaN or infinite.
    #[error("decision rule margin {margin} is not a finite number")]
    NonFiniteMargin {
        /// The refused margin.
        margin: f64,
    },
}

/// Why a decision rule could not be evaluated against supplied values.
#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum RuleEvaluationError {
    /// The estimate was NaN or infinite.
    #[error("estimate {estimate} is not a finite number")]
    NonFiniteEstimate {
        /// The refused estimate.
        estimate: f64,
    },
    /// A baseline rule was evaluated without the baseline's value.
    #[error("decision rule against baseline `{baseline}` needs that baseline's value")]
    MissingBaselineValue {
        /// The baseline whose value was not supplied.
        baseline: Baseline,
    },
    /// A threshold rule was given a baseline value it has no use for.
    #[error("decision rule against a threshold takes no baseline value")]
    UnexpectedBaselineValue,
    /// The supplied baseline value was NaN or infinite.
    #[error("baseline value {value} is not a finite number")]
    NonFiniteBaselineValue {
        /// The refused baseline value.
        value: f64,
    },
}

/// A validated `statistical_design.decision_rule`: a comparator and exactly
/// one finite reference.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "DecisionRuleFields", into = "DecisionRuleFields")]
pub struct DecisionRule {
    comparator: Comparator,
    reference: RuleReference,
}

impl DecisionRule {
    /// Build a rule against a fixed threshold.
    ///
    /// # Errors
    ///
    /// Returns [`DecisionRuleError::NonFiniteThreshold`] when `threshold` is
    /// NaN or infinite.
    pub fn against_threshold(
        comparator: Comparator,
        threshold: f64,
    ) -> Result<Self, DecisionRuleError> {
        if !threshold.is_finite() {
            return Err(DecisionRuleError::NonFiniteThreshold { threshold });
        }
        Ok(Self {
            comparator,
            reference: RuleReference::Threshold(threshold),
        })
    }

    /// Build a rule against a baseline plus an optional signed margin.
    ///
    /// # Errors
    ///
    /// Returns [`DecisionRuleError::NonFiniteMargin`] when `margin` is NaN or
    /// infinite.
    pub fn against_baseline(
        comparator: Comparator,
        baseline: Baseline,
        margin: Option<f64>,
    ) -> Result<Self, DecisionRuleError> {
        let margin = margin.unwrap_or(0.0);
        if !margin.is_finite() {
            return Err(DecisionRuleError::NonFiniteMargin { margin });
        }
        Ok(Self {
            comparator,
            reference: RuleReference::Baseline { baseline, margin },
        })
    }

    /// How the estimate is compared with the reference.
    #[must_use]
    pub const fn comparator(&self) -> Comparator {
        self.comparator
    }

    /// What the estimate is compared against.
    #[must_use]
    pub const fn reference(&self) -> RuleReference {
        self.reference
    }

    /// Whether the rule holds for `estimate`.
    ///
    /// `baseline_value` is the caller-computed value of the rule's baseline,
    /// and must be `None` for a threshold rule. The reference is the
    /// threshold, or `baseline_value + margin`.
    ///
    /// # Errors
    ///
    /// Returns [`RuleEvaluationError::NonFiniteEstimate`] or
    /// [`RuleEvaluationError::NonFiniteBaselineValue`] for a NaN or infinite
    /// input, [`RuleEvaluationError::MissingBaselineValue`] when a baseline
    /// rule gets no baseline value, and
    /// [`RuleEvaluationError::UnexpectedBaselineValue`] when a threshold rule
    /// gets one.
    pub fn holds(
        &self,
        estimate: f64,
        baseline_value: Option<f64>,
    ) -> Result<bool, RuleEvaluationError> {
        if !estimate.is_finite() {
            return Err(RuleEvaluationError::NonFiniteEstimate { estimate });
        }
        let reference = match (self.reference, baseline_value) {
            (RuleReference::Threshold(threshold), None) => threshold,
            (RuleReference::Threshold(_), Some(_)) => {
                return Err(RuleEvaluationError::UnexpectedBaselineValue);
            }
            (RuleReference::Baseline { baseline, .. }, None) => {
                return Err(RuleEvaluationError::MissingBaselineValue { baseline });
            }
            (RuleReference::Baseline { margin, .. }, Some(value)) => {
                if !value.is_finite() {
                    return Err(RuleEvaluationError::NonFiniteBaselineValue { value });
                }
                value + margin
            }
        };
        Ok(self.comparator.holds(estimate, reference))
    }
}

/// The closed wire shape of a `decision_rule` block, before validation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DecisionRuleFields {
    comparator: Comparator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    baseline: Option<Baseline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    margin: Option<f64>,
}

impl TryFrom<DecisionRuleFields> for DecisionRule {
    type Error = DecisionRuleError;

    fn try_from(fields: DecisionRuleFields) -> Result<Self, Self::Error> {
        match (fields.threshold, fields.baseline, fields.margin) {
            (Some(_), Some(_), _) => Err(DecisionRuleError::ThresholdAndBaseline),
            (None, None, _) => Err(DecisionRuleError::MissingReference),
            (Some(_), None, Some(_)) => Err(DecisionRuleError::MarginWithoutBaseline),
            (Some(threshold), None, None) => Self::against_threshold(fields.comparator, threshold),
            (None, Some(baseline), margin) => {
                Self::against_baseline(fields.comparator, baseline, margin)
            }
        }
    }
}

impl From<DecisionRule> for DecisionRuleFields {
    fn from(rule: DecisionRule) -> Self {
        let (threshold, baseline, margin) = match rule.reference {
            RuleReference::Threshold(threshold) => (Some(threshold), None, None),
            RuleReference::Baseline { baseline, margin } => {
                (None, Some(baseline), (margin != 0.0).then_some(margin))
            }
        };
        Self {
            comparator: rule.comparator,
            threshold,
            baseline,
            margin,
        }
    }
}

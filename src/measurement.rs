// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `MeasurementPlan` definition types owned by Engineering Assurance (FR-020,
//! FR-021, FR-024).
//!
//! This module owns the typed form of a `MeasurementPlan`'s optional `objective`
//! block, the closed `statistical_design.estimator` and
//! `statistical_design.decision_rule` vocabulary with the rule's evaluation,
//! the `protected_apparatus` path list and the closed `negative_controls`
//! vocabulary, and the check that an edit to the objective, estimator, rule,
//! or protected list came with a new `definition_version`. Only the decision rule is evaluated; an objective's
//! `bound` is informational.
//! It performs no filesystem, process, environment, network, clock, or
//! persistence access: callers parse plan frontmatter and pass the relevant
//! fields in, and compute any baseline value themselves.
//!
//! The wire names of [`Direction`], [`Estimator`], [`Comparator`],
//! [`Baseline`], and [`NegativeControlKind`] are the matching enums of
//! `engineering_assurance/schemas/measurement-plan-frontmatter.schema.json`;
//! tests assert each pair of sets is equal. The schema's `protected_apparatus`
//! item pattern and [`ApparatusPath`] are checked against one shared case
//! table.

use std::{collections::BTreeSet, fmt, str::FromStr};

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
/// bound, where a `target` direction always has a bound. The bound is the goal
/// the metric should reach; it is informational and never evaluated.
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

    /// The goal the metric should reach, when stated. Informational, never
    /// evaluated: only a [`DecisionRule`] is evaluated.
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
    /// weighted mean whose weights the plan's body states.
    Mean,
    /// The middle per-item value.
    Median,
    /// One total divided by another, unbounded.
    Ratio,
}

// `eq` against a `Mean` or `Ratio` estimate is exact floating-point equality
// (FR-021); nothing here rounds.

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
    ///
    /// This does not guard NaN: every comparator, `eq` included, is false when
    /// either side is NaN. [`DecisionRule::holds`] refuses non-finite inputs
    /// before it gets here.
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
    /// The agreement rate of the highest-scoring constant answer in each
    /// answer family, combined as the size-weighted mean over families (total
    /// best-constant agreements over total items), computed from the corpus.
    /// Allowed only with [`Estimator::Proportion`].
    ConstantPredictor,
    /// The plan's metric in the collection this result is compared against
    /// under the same `definition_version`.
    PriorCollection,
    /// Over every accepted collection of the plan's metric under the same
    /// `definition_version` (one the measurement intake admitted rather than
    /// refused): the maximum for a `gt`/`ge` rule, and the minimum for an
    /// `lt`/`le` rule. An `eq` rule cannot use it.
    BestSeen,
}

impl Baseline {
    /// Every baseline, in declaration order.
    pub const ALL: [Self; 3] = [
        Self::ConstantPredictor,
        Self::PriorCollection,
        Self::BestSeen,
    ];

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
    /// A value computed at evaluation time, moved by a finite margin (`0.0`
    /// when the plan states none).
    Baseline {
        /// Which baseline to compute.
        baseline: Baseline,
        /// How far the estimate must improve on the baseline, in the metric's
        /// own units: positive demands an improvement of at least this much,
        /// negative tolerates a regression of up to this much.
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
    /// An `eq` rule named `best-seen`, which has no best value to equal.
    #[error("decision rule comparator `eq` cannot use baseline `best-seen`")]
    EqAgainstBestSeen,
    /// An `eq` rule stated a `margin`, which has no direction of improvement
    /// to move the reference in.
    #[error("decision rule comparator `eq` cannot take a `margin`")]
    MarginWithEq,
    /// The comparator disagrees with the plan objective's direction.
    #[error(
        "decision rule comparator `{comparator}` disagrees with objective direction `{direction}`"
    )]
    DirectionMismatch {
        /// The objective's direction.
        direction: Direction,
        /// The rule's comparator.
        comparator: Comparator,
    },
    /// A `constant-predictor` rule was paired with an estimator other than
    /// `proportion`.
    #[error(
        "decision rule baseline `constant-predictor` requires estimator `proportion`, not `{estimator}`"
    )]
    ConstantPredictorRequiresProportion {
        /// The plan's estimator.
        estimator: Estimator,
    },
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
    /// The baseline value moved by the margin overflowed to a non-finite
    /// reference.
    #[error(
        "reference {reference} computed from the baseline value and margin is not a finite number"
    )]
    NonFiniteReference {
        /// The non-finite reference.
        reference: f64,
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
    /// Returns [`DecisionRuleError::EqAgainstBestSeen`] for an `eq` rule
    /// against [`Baseline::BestSeen`], [`DecisionRuleError::MarginWithEq`]
    /// for an `eq` rule with a margin, and
    /// [`DecisionRuleError::NonFiniteMargin`] when `margin` is NaN or
    /// infinite.
    pub fn against_baseline(
        comparator: Comparator,
        baseline: Baseline,
        margin: Option<f64>,
    ) -> Result<Self, DecisionRuleError> {
        if comparator == Comparator::Eq {
            if baseline == Baseline::BestSeen {
                return Err(DecisionRuleError::EqAgainstBestSeen);
            }
            if margin.is_some() {
                return Err(DecisionRuleError::MarginWithEq);
            }
        }
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

    /// Check that this rule's comparator agrees with the plan's objective:
    /// `higher` takes `gt` or `ge`, `lower` takes `lt` or `le`, `zero` takes
    /// `eq` or `le` against threshold 0, and `target` takes any comparator.
    ///
    /// # Errors
    ///
    /// Returns [`DecisionRuleError::DirectionMismatch`] when they disagree.
    pub fn check_against(&self, objective: &Objective) -> Result<(), DecisionRuleError> {
        let direction = objective.direction();
        let agrees = match direction {
            Direction::Higher => matches!(self.comparator, Comparator::Gt | Comparator::Ge),
            Direction::Lower => matches!(self.comparator, Comparator::Lt | Comparator::Le),
            Direction::Zero => match (self.comparator, self.reference) {
                (Comparator::Eq, _) => true,
                (Comparator::Le, RuleReference::Threshold(threshold)) => threshold == 0.0,
                _ => false,
            },
            Direction::Target => true,
        };
        if agrees {
            Ok(())
        } else {
            Err(DecisionRuleError::DirectionMismatch {
                direction,
                comparator: self.comparator,
            })
        }
    }

    /// Check that this rule's baseline is allowed with the plan's estimator:
    /// [`Baseline::ConstantPredictor`] requires [`Estimator::Proportion`].
    ///
    /// # Errors
    ///
    /// Returns [`DecisionRuleError::ConstantPredictorRequiresProportion`] for
    /// a constant-predictor rule under any other estimator.
    pub fn check_estimator(&self, estimator: Estimator) -> Result<(), DecisionRuleError> {
        match (self.reference, estimator) {
            (
                RuleReference::Baseline {
                    baseline: Baseline::ConstantPredictor,
                    ..
                },
                Estimator::Count | Estimator::Mean | Estimator::Median | Estimator::Ratio,
            ) => Err(DecisionRuleError::ConstantPredictorRequiresProportion { estimator }),
            _ => Ok(()),
        }
    }

    /// Whether the rule holds for `estimate`.
    ///
    /// `baseline_value` is the caller-computed value of the rule's baseline,
    /// and must be `None` for a threshold rule. The reference is the
    /// threshold, or the baseline value moved by the margin in the direction
    /// of improvement: `baseline_value + margin` for `gt`/`ge`, and
    /// `baseline_value - margin` for `lt`/`le` (an `eq` rule has no margin).
    ///
    /// # Errors
    ///
    /// Returns [`RuleEvaluationError::NonFiniteEstimate`] or
    /// [`RuleEvaluationError::NonFiniteBaselineValue`] for a NaN or infinite
    /// input, [`RuleEvaluationError::NonFiniteReference`] when the baseline
    /// value moved by the margin overflows,
    /// [`RuleEvaluationError::MissingBaselineValue`] when a baseline rule gets
    /// no baseline value, and
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
                let reference = match self.comparator {
                    Comparator::Gt | Comparator::Ge | Comparator::Eq => value + margin,
                    Comparator::Lt | Comparator::Le => value - margin,
                };
                if !reference.is_finite() {
                    return Err(RuleEvaluationError::NonFiniteReference { reference });
                }
                reference
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

/// Why a `protected_apparatus` entry is not a safe repository-relative path or
/// glob.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApparatusPathError {
    /// The entry was the empty string.
    #[error("protected apparatus path is empty")]
    Empty,
    /// The entry started with `/`.
    #[error("protected apparatus path `{path}` is absolute; it must be repository-relative")]
    Absolute {
        /// The refused entry.
        path: String,
    },
    /// The entry contained a character outside the path and glob syntax: `\`,
    /// `?`, `[`, `]`, `{`, `}`, `:`, or a control character.
    #[error("protected apparatus path `{path}` contains the forbidden character {character:?}")]
    ForbiddenCharacter {
        /// The refused entry.
        path: String,
        /// The first forbidden character.
        character: char,
    },
    /// The entry had an empty segment: `//`, or a trailing `/`.
    #[error("protected apparatus path `{path}` has an empty segment")]
    EmptySegment {
        /// The refused entry.
        path: String,
    },
    /// The entry had a `.` segment.
    #[error("protected apparatus path `{path}` has a `.` segment")]
    CurrentDirectorySegment {
        /// The refused entry.
        path: String,
    },
    /// The entry had a `..` segment.
    #[error("protected apparatus path `{path}` has a `..` segment")]
    ParentDirectorySegment {
        /// The refused entry.
        path: String,
    },
    /// A segment contained `**` without being exactly `**`.
    #[error(
        "protected apparatus path `{path}` uses `**` inside a segment; `**` must be a whole segment"
    )]
    PartialDoubleStar {
        /// The refused entry.
        path: String,
    },
}

/// One `protected_apparatus` entry: a repository-relative file path or glob
/// naming a file that produces the plan's number.
///
/// Segments are separated by `/`. A segment is either exactly `**` (zero or
/// more whole directories) or a name in which each `*` matches zero or more
/// characters other than `/`. Every other character is literal; `\`, `?`,
/// `[`, `]`, `{`, `}`, `:`, and control characters are refused, as are an
/// empty entry, a leading `/`, an empty segment, and a `.` or `..` segment.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ApparatusPath(String);

impl ApparatusPath {
    /// Validate one entry.
    ///
    /// # Errors
    ///
    /// Returns the [`ApparatusPathError`] naming the first rule the entry
    /// breaks, checked in the order: empty, absolute, forbidden character,
    /// then per segment empty, `.`, `..`, and partial `**`.
    pub fn new(path: impl Into<String>) -> Result<Self, ApparatusPathError> {
        let path = path.into();
        if path.is_empty() {
            return Err(ApparatusPathError::Empty);
        }
        if path.starts_with('/') {
            return Err(ApparatusPathError::Absolute { path });
        }
        if let Some(character) = path.chars().find(|character| {
            u32::from(*character) < 0x20
                || matches!(character, '\\' | '?' | '[' | ']' | '{' | '}' | ':')
        }) {
            return Err(ApparatusPathError::ForbiddenCharacter { path, character });
        }
        for segment in path.split('/') {
            let refusal = match segment {
                "" => ApparatusPathError::EmptySegment { path },
                "." => ApparatusPathError::CurrentDirectorySegment { path },
                ".." => ApparatusPathError::ParentDirectorySegment { path },
                "**" => continue,
                _ if segment.contains("**") => ApparatusPathError::PartialDoubleStar { path },
                _ => continue,
            };
            return Err(refusal);
        }
        Ok(Self(path))
    }

    /// The entry as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the entry is a glob (contains `*`) rather than one file path.
    #[must_use]
    pub fn is_glob(&self) -> bool {
        self.0.contains('*')
    }
}

impl fmt::Display for ApparatusPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for ApparatusPath {
    type Err = ApparatusPathError;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Self::new(path)
    }
}

impl TryFrom<String> for ApparatusPath {
    type Error = ApparatusPathError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl From<ApparatusPath> for String {
    fn from(path: ApparatusPath) -> Self {
        path.0
    }
}

/// Why a `protected_apparatus` list is invalid.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProtectedApparatusError {
    /// The list was empty.
    #[error("protected_apparatus must name at least one path")]
    Empty,
    /// The same entry appeared twice.
    #[error("protected_apparatus names `{path}` more than once")]
    Duplicate {
        /// The repeated entry.
        path: ApparatusPath,
    },
}

/// A validated `protected_apparatus`: a non-empty set of distinct
/// [`ApparatusPath`] entries naming the files that produce the plan's number.
///
/// It is a set: two lists naming the same entries in a different order are
/// equal, and it serializes in sorted order.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "Vec<ApparatusPath>", into = "Vec<ApparatusPath>")]
pub struct ProtectedApparatus(BTreeSet<ApparatusPath>);

impl ProtectedApparatus {
    /// Build the set from its entries.
    ///
    /// # Errors
    ///
    /// Returns [`ProtectedApparatusError::Empty`] for no entries and
    /// [`ProtectedApparatusError::Duplicate`] naming the first repeated one.
    pub fn new(
        paths: impl IntoIterator<Item = ApparatusPath>,
    ) -> Result<Self, ProtectedApparatusError> {
        let mut set = BTreeSet::new();
        for path in paths {
            if set.contains(&path) {
                return Err(ProtectedApparatusError::Duplicate { path });
            }
            set.insert(path);
        }
        if set.is_empty() {
            return Err(ProtectedApparatusError::Empty);
        }
        Ok(Self(set))
    }

    /// The entries, in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &ApparatusPath> {
        self.0.iter()
    }

    /// How many entries the set holds; never zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: a validated set is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `path` is one of the entries, compared as written (a glob is
    /// not expanded).
    #[must_use]
    pub fn contains(&self, path: &ApparatusPath) -> bool {
        self.0.contains(path)
    }
}

impl TryFrom<Vec<ApparatusPath>> for ProtectedApparatus {
    type Error = ProtectedApparatusError;

    fn try_from(paths: Vec<ApparatusPath>) -> Result<Self, Self::Error> {
        Self::new(paths)
    }
}

impl From<ProtectedApparatus> for Vec<ApparatusPath> {
    fn from(apparatus: ProtectedApparatus) -> Self {
        apparatus.0.into_iter().collect()
    }
}

/// A gaming scenario a `MeasurementPlan`'s measurement must catch
/// (`negative_controls[].kind`).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NegativeControlKind {
    /// An unfavourable item or run is left out of the collected population.
    SuppressedObservation,
    /// An improvement smaller than the plan's stated uncertainty is claimed
    /// as a gain.
    GainWithinNoise,
    /// A result collected against an earlier subject version or apparatus
    /// is presented as current.
    StaleEvidence,
    /// A protected-apparatus file, such as the harness, the answer key, or
    /// the checker configuration, is edited alongside the change it grades.
    ApparatusEdit,
    /// Only a favourable run or variant is reported out of several tried.
    SelectiveReporting,
}

impl NegativeControlKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::SuppressedObservation,
        Self::GainWithinNoise,
        Self::StaleEvidence,
        Self::ApparatusEdit,
        Self::SelectiveReporting,
    ];

    /// The frontmatter wire name of this kind.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::SuppressedObservation => "suppressed-observation",
            Self::GainWithinNoise => "gain-within-noise",
            Self::StaleEvidence => "stale-evidence",
            Self::ApparatusEdit => "apparatus-edit",
            Self::SelectiveReporting => "selective-reporting",
        }
    }
}

impl fmt::Display for NegativeControlKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

/// Why a negative control, or a `negative_controls` list, is invalid.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NegativeControlError {
    /// A control had an empty `description`.
    #[error("negative control `{kind}` has an empty description")]
    EmptyDescription {
        /// The control's kind.
        kind: NegativeControlKind,
    },
    /// The list was empty.
    #[error("negative_controls must name at least one control")]
    Empty,
    /// The same control (kind and description) appeared twice.
    #[error("negative_controls lists the `{kind}` control `{description}` more than once")]
    Duplicate {
        /// The repeated control's kind.
        kind: NegativeControlKind,
        /// The repeated control's description.
        description: String,
    },
}

/// One `negative_controls` entry: a gaming scenario and how this plan's
/// measurement catches it.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "NegativeControlFields", into = "NegativeControlFields")]
pub struct NegativeControl {
    kind: NegativeControlKind,
    description: String,
}

impl NegativeControl {
    /// Build a control from its kind and description.
    ///
    /// # Errors
    ///
    /// Returns [`NegativeControlError::EmptyDescription`] when `description`
    /// is empty.
    pub fn new(
        kind: NegativeControlKind,
        description: impl Into<String>,
    ) -> Result<Self, NegativeControlError> {
        let description = description.into();
        if description.is_empty() {
            return Err(NegativeControlError::EmptyDescription { kind });
        }
        Ok(Self { kind, description })
    }

    /// The gaming scenario.
    #[must_use]
    pub const fn kind(&self) -> NegativeControlKind {
        self.kind
    }

    /// How this plan's measurement catches the scenario.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// The closed wire shape of a `negative_controls` entry, before validation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NegativeControlFields {
    kind: NegativeControlKind,
    description: String,
}

impl TryFrom<NegativeControlFields> for NegativeControl {
    type Error = NegativeControlError;

    fn try_from(fields: NegativeControlFields) -> Result<Self, Self::Error> {
        Self::new(fields.kind, fields.description)
    }
}

impl From<NegativeControl> for NegativeControlFields {
    fn from(control: NegativeControl) -> Self {
        Self {
            kind: control.kind,
            description: control.description,
        }
    }
}

/// A validated `negative_controls` list: at least one control, no two
/// identical, in the order written.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "Vec<NegativeControl>", into = "Vec<NegativeControl>")]
pub struct NegativeControls(Vec<NegativeControl>);

impl NegativeControls {
    /// Build the list from its controls.
    ///
    /// # Errors
    ///
    /// Returns [`NegativeControlError::Empty`] for no controls and
    /// [`NegativeControlError::Duplicate`] naming the first repeated one.
    pub fn new(
        controls: impl IntoIterator<Item = NegativeControl>,
    ) -> Result<Self, NegativeControlError> {
        let mut list: Vec<NegativeControl> = Vec::new();
        for control in controls {
            if list.contains(&control) {
                return Err(NegativeControlError::Duplicate {
                    kind: control.kind,
                    description: control.description,
                });
            }
            list.push(control);
        }
        if list.is_empty() {
            return Err(NegativeControlError::Empty);
        }
        Ok(Self(list))
    }

    /// The controls, in the order written.
    #[must_use]
    pub fn as_slice(&self) -> &[NegativeControl] {
        &self.0
    }

    /// Whether any control is of `kind`.
    #[must_use]
    pub fn covers(&self, kind: NegativeControlKind) -> bool {
        self.0.iter().any(|control| control.kind == kind)
    }
}

impl TryFrom<Vec<NegativeControl>> for NegativeControls {
    type Error = NegativeControlError;

    fn try_from(controls: Vec<NegativeControl>) -> Result<Self, Self::Error> {
        Self::new(controls)
    }
}

impl From<NegativeControls> for Vec<NegativeControl> {
    fn from(controls: NegativeControls) -> Self {
        controls.0
    }
}

/// One member of a `MeasurementPlan`'s measurement definition whose edit
/// requires a new `definition_version`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DefinitionMember {
    /// The `objective` block.
    Objective,
    /// `statistical_design.estimator`.
    Estimator,
    /// `statistical_design.decision_rule`.
    DecisionRule,
    /// The `protected_apparatus` list (the entries, not the content of the
    /// files they name).
    ProtectedApparatus,
}

impl DefinitionMember {
    /// Every member, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::Objective,
        Self::Estimator,
        Self::DecisionRule,
        Self::ProtectedApparatus,
    ];

    /// The frontmatter path of this member.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Objective => "objective",
            Self::Estimator => "statistical_design.estimator",
            Self::DecisionRule => "statistical_design.decision_rule",
            Self::ProtectedApparatus => "protected_apparatus",
        }
    }
}

impl fmt::Display for DefinitionMember {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.path())
    }
}

/// The versioned members of one `MeasurementPlan`'s measurement definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasurementDefinition {
    /// The plan's `objective`, when present.
    pub objective: Option<Objective>,
    /// The plan's `statistical_design.estimator`, when present.
    pub estimator: Option<Estimator>,
    /// The plan's `statistical_design.decision_rule`, when present.
    pub decision_rule: Option<DecisionRule>,
    /// The plan's `protected_apparatus`, when present. Compared as a set of
    /// entries: reordering the list is not a change, and an edit to a file an
    /// entry names is not visible here (Quoin's intake digests the files).
    pub protected_apparatus: Option<ProtectedApparatus>,
}

impl MeasurementDefinition {
    /// The members that differ between `self` and `other`, in
    /// [`DefinitionMember::ALL`] order.
    #[must_use]
    pub fn changed_members(&self, other: &Self) -> Vec<DefinitionMember> {
        DefinitionMember::ALL
            .into_iter()
            .filter(|member| match member {
                DefinitionMember::Objective => self.objective != other.objective,
                DefinitionMember::Estimator => self.estimator != other.estimator,
                DefinitionMember::DecisionRule => self.decision_rule != other.decision_rule,
                DefinitionMember::ProtectedApparatus => {
                    self.protected_apparatus != other.protected_apparatus
                }
            })
            .collect()
    }
}

/// The parts of one `MeasurementPlan`'s frontmatter that identify its
/// measurement definition for the definition-change check.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanDefinition<'a> {
    /// The plan's `definition_version`, when present.
    pub definition_version: Option<&'a str>,
    /// The plan's versioned definition members.
    pub definition: MeasurementDefinition,
}

/// A measurement-definition member that was added, removed, or changed while
/// the plan's `definition_version` stayed the same.
#[derive(Clone, Debug, PartialEq)]
pub struct DefinitionChangedWithoutVersionBump {
    /// The `definition_version` shared by both plan revisions.
    pub definition_version: Option<String>,
    /// Which members changed, in [`DefinitionMember::ALL`] order; never empty.
    pub changed: Vec<DefinitionMember>,
    /// The definition before the edit.
    pub before: MeasurementDefinition,
    /// The definition after the edit.
    pub after: MeasurementDefinition,
}

/// Report an edit to the objective, estimator, decision rule, or protected
/// apparatus list between two revisions of one plan that did not come with a
/// genuine `definition_version` bump.
///
/// Returns `None` when no member changed, or when `after`'s
/// `definition_version` is present and differs from `before`'s (a genuine
/// bump). Clearing `definition_version` (`after` is `None`) is not a bump:
/// it is treated the same as leaving the version unchanged, so an edit
/// alongside a removed `definition_version` still yields a finding.
#[must_use]
pub fn definition_change_without_version_bump(
    before: &PlanDefinition<'_>,
    after: &PlanDefinition<'_>,
) -> Option<DefinitionChangedWithoutVersionBump> {
    let changed = before.definition.changed_members(&after.definition);
    if changed.is_empty() {
        return None;
    }
    let genuinely_bumped = match after.definition_version {
        None => false,
        Some(after_version) => Some(after_version) != before.definition_version,
    };
    if genuinely_bumped {
        return None;
    }
    Some(DefinitionChangedWithoutVersionBump {
        definition_version: before.definition_version.map(str::to_owned),
        changed,
        before: before.definition.clone(),
        after: after.definition.clone(),
    })
}

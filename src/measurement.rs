// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! MeasurementPlan definition types owned by Engineering Assurance (FR-020).
//!
//! This module owns the typed form of a MeasurementPlan's optional `objective`
//! block and the check that an objective edit came with a new
//! `definition_version`. It performs no filesystem, process, environment,
//! network, clock, or persistence access: callers parse plan frontmatter and
//! pass the relevant fields in.
//!
//! The wire names of [`Direction`] are the `direction` enum of
//! `engineering_assurance/schemas/measurement-plan-frontmatter.schema.json`;
//! a test asserts the two sets are equal.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Which way a MeasurementPlan's metric is supposed to move.
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

/// A validated MeasurementPlan objective: a direction and an optional finite
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

/// The parts of one MeasurementPlan's frontmatter that identify its
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
/// change `definition_version`.
///
/// Returns `None` when the objective is unchanged or when the
/// `definition_version` differs between the revisions.
#[must_use]
pub fn objective_change_without_version_bump(
    before: &PlanDefinition<'_>,
    after: &PlanDefinition<'_>,
) -> Option<ObjectiveChangedWithoutVersionBump> {
    if before.objective == after.objective || before.definition_version != after.definition_version
    {
        return None;
    }
    Some(ObjectiveChangedWithoutVersionBump {
        definition_version: before.definition_version.map(str::to_owned),
        before: before.objective,
        after: after.objective,
    })
}

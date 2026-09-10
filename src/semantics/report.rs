// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded assurance-report validation and rendering.

use serde::{Deserialize, Serialize};

use super::{
    REPORT_PROJECTION_PROTOCOL, SemanticError, SemanticErrorKind, non_empty, validate_identity,
};

/// One bounded report claim.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportClaim {
    /// Stable claim identity.
    pub claim_id: String,
    /// Human-readable claim statement.
    pub statement: String,
    /// Closed claim status.
    pub status: ReportClaimStatus,
}

/// Closed bounded-report claim status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportClaimStatus {
    /// Claim is open.
    Open,
    /// Available evidence supports the claim.
    Supported,
    /// Available counterevidence challenges the claim.
    Challenged,
}

impl std::fmt::Display for ReportClaimStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Open => "open",
            Self::Supported => "supported",
            Self::Challenged => "challenged",
        })
    }
}

/// One bounded report relationship.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReference {
    /// Semantic reference identity.
    pub semantic_ref: String,
    /// Closed relationship kind.
    pub relation: ReportRelation,
}

/// Closed bounded-report relationship kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportRelation {
    /// Reference supports the claim set.
    Supports,
    /// Reference challenges the claim set.
    Challenges,
    /// Reference supplies context without deciding a claim.
    Context,
}

impl std::fmt::Display for ReportRelation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Supports => "supports",
            Self::Challenges => "challenges",
            Self::Context => "context",
        })
    }
}

/// One explicit assurance gap.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportGap {
    /// Stable gap identity.
    pub gap_id: String,
    /// Human-readable gap summary.
    pub summary: String,
    /// Named gap owner.
    pub owner: String,
    /// Required next action.
    pub action: String,
}

/// Read-only bounded report projection with no aggregate verdict field.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportProjection {
    /// Projection protocol discriminator.
    pub projection_type: String,
    /// Stable report identity.
    pub report_id: String,
    /// Subject being reviewed.
    pub subject: String,
    /// Explicit claims.
    pub claims: Vec<ReportClaim>,
    /// Supporting evidence references.
    pub evidence: Vec<ReportReference>,
    /// Challenging evidence references.
    pub counterevidence: Vec<ReportReference>,
    /// Explicit gaps.
    pub gaps: Vec<ReportGap>,
    /// Named report owner.
    pub owner: String,
    /// Explicit next actions.
    pub actions: Vec<String>,
    /// Optional externally authoritative human-decision reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_ref: Option<String>,
}

impl ReportProjection {
    /// Validate the bounded report vocabulary without inferring a verdict.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for an unsupported protocol or malformed field.
    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.projection_type != REPORT_PROJECTION_PROTOCOL {
            return Err(SemanticError::new(
                SemanticErrorKind::UnsupportedProtocol,
                format!(
                    "unsupported report projection protocol {:?}",
                    self.projection_type
                ),
            ));
        }
        validate_identity(&self.report_id, "report_id")?;
        non_empty(&self.subject, "report subject")?;
        non_empty(&self.owner, "report owner")?;
        for claim in &self.claims {
            validate_identity(&claim.claim_id, "claim_id")?;
            non_empty(&claim.statement, "claim statement")?;
        }
        for reference in self.evidence.iter().chain(&self.counterevidence) {
            validate_identity(&reference.semantic_ref, "semantic_ref")?;
        }
        for gap in &self.gaps {
            validate_identity(&gap.gap_id, "gap_id")?;
            non_empty(&gap.summary, "gap summary")?;
            non_empty(&gap.owner, "gap owner")?;
            non_empty(&gap.action, "gap action")?;
        }
        for action in &self.actions {
            non_empty(action, "report action")?;
        }
        if let Some(reference) = &self.decision_ref {
            validate_identity(reference, "decision_ref")?;
        }
        Ok(())
    }

    /// Render canonical compact JSON followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for invalid input or serialization failure.
    pub fn render_json(&self) -> Result<String, SemanticError> {
        self.validate()?;
        let value = serde_json::to_value(self).map_err(|error| {
            SemanticError::new(
                SemanticErrorKind::Serialization,
                format!("report serialization failed: {error}"),
            )
        })?;
        let mut encoded = serde_json::to_string(&value).map_err(|error| {
            SemanticError::new(
                SemanticErrorKind::Serialization,
                format!("report serialization failed: {error}"),
            )
        })?;
        encoded.push('\n');
        Ok(encoded)
    }

    /// Render the retained bounded Markdown view.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for invalid input.
    pub fn render_markdown(&self) -> Result<String, SemanticError> {
        self.validate()?;
        let mut lines = vec![
            format!("# Assurance report: {}", markdown_text(&self.subject)),
            String::new(),
            "## Claims".to_owned(),
            String::new(),
        ];
        if self.claims.is_empty() {
            lines.push("- No claims declared.".to_owned());
        } else {
            lines.extend(self.claims.iter().map(|item| {
                format!(
                    "- `{}` [{}]: {}",
                    item.claim_id,
                    item.status,
                    markdown_text(&item.statement)
                )
            }));
        }
        for (heading, entries, empty) in [
            ("Evidence", &self.evidence, "- No evidence declared."),
            (
                "Counterevidence",
                &self.counterevidence,
                "- No counterevidence declared.",
            ),
        ] {
            lines.extend([String::new(), format!("## {heading}"), String::new()]);
            if entries.is_empty() {
                lines.push(empty.to_owned());
            } else {
                lines.extend(
                    entries
                        .iter()
                        .map(|entry| format!("- `{}` ({})", entry.semantic_ref, entry.relation)),
                );
            }
        }
        lines.extend([String::new(), "## Gaps".to_owned(), String::new()]);
        if self.gaps.is_empty() {
            lines.push("- No gaps declared.".to_owned());
        } else {
            lines.extend(self.gaps.iter().map(|gap| {
                format!(
                    "- `{}`: {} (owner: {}; action: {})",
                    gap.gap_id,
                    markdown_text(&gap.summary),
                    markdown_text(&gap.owner),
                    markdown_text(&gap.action)
                )
            }));
        }
        lines.extend([
            String::new(),
            "## Owner".to_owned(),
            String::new(),
            markdown_text(&self.owner),
            String::new(),
            "## Actions".to_owned(),
            String::new(),
        ]);
        if self.actions.is_empty() {
            lines.push("- No actions declared.".to_owned());
        } else {
            lines.extend(
                self.actions
                    .iter()
                    .map(|action| format!("- {}", markdown_text(action))),
            );
        }
        lines.extend([
            String::new(),
            "## Human decision reference".to_owned(),
            String::new(),
            markdown_text(
                self.decision_ref
                    .as_deref()
                    .unwrap_or("No decision recorded."),
            ),
            String::new(),
        ]);
        Ok(lines.join("\n"))
    }
}

/// Parse and validate an encoded bounded report.
///
/// # Errors
///
/// Returns [`SemanticError`] for malformed JSON or invalid report fields.
pub fn validate_report_bytes(raw: &[u8]) -> Result<ReportProjection, SemanticError> {
    let report: ReportProjection = serde_json::from_slice(raw).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::InvalidInputEncoding,
            format!("invalid report projection: {error}"),
        )
    })?;
    report.validate()?;
    Ok(report)
}

fn markdown_text(value: &str) -> String {
    value
        .lines()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

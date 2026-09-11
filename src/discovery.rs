// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure validation of the canonical Engineering Assurance discovery bundle.
//!
//! Four agent hosts are supposed to find one skill. Each declares a thin
//! manifest naming where that skill lives, and the failure this module exists
//! to prevent is a manifest that carries its own copy of the behaviour instead
//! of a reference to the canonical one, or that points somewhere outside the
//! bundle entirely. Both look like a working host until the copies disagree.
//!
//! This module reads no file. A caller supplies manifest bytes already parsed
//! into JSON and the inventory it observed on disk; the confined adapter that
//! obtains either belongs outside the reusable library, which FR-014 requires
//! to be free of filesystem access.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::workflow::WorkflowKind;

/// Bundle-relative directory that holds every canonical skill.
pub const CANONICAL_SKILLS_DIRECTORY: &str = "engineering_assurance/skills";

/// Directory name of the one canonical skill.
pub const CANONICAL_SKILL_NAME: &str = "assurance-onboarding";

/// Bundle-relative path of the one canonical skill file.
pub const CANONICAL_SKILL_FILE: &str = "engineering_assurance/skills/assurance-onboarding/SKILL.md";

/// Bundle-relative directory that holds the promoted workflow definitions.
pub const CANONICAL_WORKFLOWS_DIRECTORY: &str =
    "engineering_assurance/skills/assurance-onboarding/workflows";

/// File name that carries one workflow definition.
pub const WORKFLOW_DEFINITION_NAME: &str = "def.yaml";

/// The longest bundle-relative target this module will consider.
///
/// A manifest is repository-owned text, but it is still parsed input, and an
/// unbounded path would be carried all the way to a host adapter before
/// anything refused it.
pub const MAX_TARGET_BYTES: usize = 4_096;

/// One supported agent host and the manifest that exposes its skill source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HostSurface {
    /// Stable host identity.
    pub name: HostSurfaceName,
    /// Bundle-relative path of the host's discovery manifest.
    pub manifest: &'static str,
    /// The complete set of keys the manifest may carry.
    pub allowed_keys: &'static [&'static str],
    /// The shape this host's `skills` value must take.
    pub source_shape: SkillSourceShape,
}

/// The supported agent hosts, as a closed set.
///
/// The set is closed rather than configurable because a host nobody enumerated
/// would resolve whatever it liked and still pass a "every declared host
/// agrees" check, the declared hosts being the ones that agreed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostSurfaceName {
    /// Claude Code.
    ClaudeCode,
    /// Codex.
    Codex,
    /// opencode.
    Opencode,
    /// GitHub Copilot.
    GithubCopilot,
}

impl HostSurfaceName {
    /// Return the stable wire spelling of this host.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
            Self::GithubCopilot => "github-copilot",
        }
    }
}

impl std::fmt::Display for HostSurfaceName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// How one host spells the single canonical skill source it references.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SkillSourceShape {
    /// The `skills` value is one string.
    SingleString,
    /// The `skills` value is an array holding exactly one string.
    SingleElementArray,
}

/// Keys a plugin-style discovery manifest may carry.
const PLUGIN_KEYS: &[&str] = &["name", "version", "description", "skills"];

/// Keys the opencode discovery manifest may carry.
const OPENCODE_KEYS: &[&str] = &["$schema", "skills"];

/// Every supported host surface, in host-name order.
pub const HOST_SURFACES: [HostSurface; 4] = [
    HostSurface {
        name: HostSurfaceName::ClaudeCode,
        manifest: ".claude-plugin/plugin.json",
        allowed_keys: PLUGIN_KEYS,
        source_shape: SkillSourceShape::SingleString,
    },
    HostSurface {
        name: HostSurfaceName::Codex,
        manifest: ".codex-plugin/plugin.json",
        allowed_keys: PLUGIN_KEYS,
        source_shape: SkillSourceShape::SingleString,
    },
    HostSurface {
        name: HostSurfaceName::Opencode,
        manifest: "opencode.json",
        allowed_keys: OPENCODE_KEYS,
        source_shape: SkillSourceShape::SingleElementArray,
    },
    HostSurface {
        name: HostSurfaceName::GithubCopilot,
        manifest: ".github/plugin/plugin.json",
        allowed_keys: PLUGIN_KEYS,
        source_shape: SkillSourceShape::SingleString,
    },
];

/// One host surface resolved to the canonical skill it references.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ResolvedSkill {
    /// The host that resolved.
    pub surface: HostSurfaceName,
    /// Bundle-relative path of the manifest that resolved it.
    pub manifest: &'static str,
    /// Bundle-relative skill directory this manifest declared.
    pub skill_source: String,
    /// Bundle-relative skill file that directory implies.
    pub skill_file: String,
}

/// Stable refusals at the pure discovery boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DiscoveryError {
    /// The manifest is not a JSON object.
    #[error("{surface} manifest is not a JSON object")]
    ManifestNotAnObject {
        /// The host whose manifest was supplied.
        surface: HostSurfaceName,
    },
    /// The manifest carries a key outside its allowed set.
    #[error("{surface} manifest embeds unsupported content: {keys:?}")]
    UnsupportedManifestKeys {
        /// The host whose manifest was supplied.
        surface: HostSurfaceName,
        /// The offending keys, in sorted order.
        keys: Vec<String>,
    },
    /// The `skills` value does not have this host's required shape.
    #[error("{surface} does not declare exactly one canonical skill source: {detail}")]
    SkillSourceShapeInvalid {
        /// The host whose manifest was supplied.
        surface: HostSurfaceName,
        /// Stable shape detail.
        detail: &'static str,
    },
    /// The declared target leaves the bundle or cannot be a bundle path.
    #[error("{surface} discovery target escapes the bundle: {observed:?}")]
    TargetEscapesBundle {
        /// The host whose manifest was supplied.
        surface: HostSurfaceName,
        /// The target exactly as declared.
        observed: String,
    },
    /// The declared target resolves somewhere other than the canonical source.
    #[error("{surface} does not resolve the canonical skill source: {observed:?}")]
    TargetNotCanonical {
        /// The host whose manifest was supplied.
        surface: HostSurfaceName,
        /// The normalized target.
        observed: String,
    },
    /// The supplied host set is not exactly the four supported hosts.
    #[error("supported host set is not exact: {detail}")]
    HostSetNotExact {
        /// Stable set detail.
        detail: String,
    },
    /// The bundle holds other than exactly the one canonical skill file.
    #[error("bundle must contain exactly one canonical assurance-onboarding skill: {observed:?}")]
    SkillInventoryNotExact {
        /// The observed skill files, in sorted order.
        observed: Vec<String>,
    },
    /// The canonical skill exposes other than exactly the promoted workflows.
    #[error("canonical workflow set is not exact: {observed:?}")]
    WorkflowSetNotExact {
        /// The observed workflow names, in sorted order.
        observed: Vec<String>,
    },
    /// Two hosts resolved different skill files.
    #[error("supported hosts resolve different skill files: {observed:?}")]
    DivergentResolution {
        /// The distinct resolved skill files, in sorted order.
        observed: Vec<String>,
    },
}

impl DiscoveryError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ManifestNotAnObject { .. }
            | Self::UnsupportedManifestKeys { .. }
            | Self::SkillSourceShapeInvalid { .. } => "discovery_manifest_invalid",
            Self::TargetEscapesBundle { .. } => "discovery_target_escapes_bundle",
            Self::TargetNotCanonical { .. } => "discovery_target_not_canonical",
            Self::HostSetNotExact { .. } => "discovery_host_set_not_exact",
            Self::SkillInventoryNotExact { .. } => "discovery_skill_inventory_not_exact",
            Self::WorkflowSetNotExact { .. } => "discovery_workflow_set_not_exact",
            Self::DivergentResolution { .. } => "discovery_resolution_divergent",
        }
    }
}

/// Return the promoted workflow names the canonical skill must expose.
///
/// The names come from [`WorkflowKind`] rather than from a list of their own,
/// so a workflow cannot be discoverable without also being bindable.
#[must_use]
pub fn expected_workflows() -> BTreeSet<String> {
    WorkflowKind::ALL
        .into_iter()
        .map(|workflow| workflow.as_str().to_owned())
        .collect()
}

/// Validate one thin manifest and return the canonical skill it resolves.
///
/// # Errors
///
/// Returns a typed [`DiscoveryError`] when the document is not an object,
/// carries a key outside the host's allowed set, declares a `skills` value of
/// the wrong shape, or names a target that leaves the bundle or is not the
/// canonical skill source.
pub fn validate_manifest_document(
    surface: &HostSurface,
    document: &Value,
) -> Result<ResolvedSkill, DiscoveryError> {
    let object = document
        .as_object()
        .ok_or(DiscoveryError::ManifestNotAnObject {
            surface: surface.name,
        })?;
    let unsupported = object
        .keys()
        .filter(|key| !surface.allowed_keys.contains(&key.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    if !unsupported.is_empty() {
        return Err(DiscoveryError::UnsupportedManifestKeys {
            surface: surface.name,
            keys: unsupported.into_iter().collect(),
        });
    }
    let declared = skill_source(surface, object.get("skills"))?;
    let normalized =
        normalize_bundle_relative(declared).ok_or_else(|| DiscoveryError::TargetEscapesBundle {
            surface: surface.name,
            observed: declared.to_owned(),
        })?;
    Ok(ResolvedSkill {
        surface: surface.name,
        manifest: surface.manifest,
        skill_file: format!("{normalized}/{CANONICAL_SKILL_NAME}/SKILL.md"),
        skill_source: normalized,
    })
}

/// Validate that a supplied host set is exactly the four supported hosts.
///
/// # Errors
///
/// Returns [`DiscoveryError::HostSetNotExact`] when a supported host is
/// missing, repeated, or joined by one nobody declared.
pub fn validate_surface_set(surfaces: &[HostSurface]) -> Result<(), DiscoveryError> {
    let observed = surfaces
        .iter()
        .map(|surface| surface.name)
        .collect::<BTreeSet<_>>();
    let supported = HOST_SURFACES
        .iter()
        .map(|surface| surface.name)
        .collect::<BTreeSet<_>>();
    if observed.len() != surfaces.len() {
        return Err(DiscoveryError::HostSetNotExact {
            detail: format!("{} entries name {} hosts", surfaces.len(), observed.len()),
        });
    }
    if observed != supported {
        return Err(DiscoveryError::HostSetNotExact {
            detail: format!(
                "observed {:?}",
                observed
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
            ),
        });
    }
    // Two entries may name four distinct hosts and still disagree about where
    // one of them looks, which is the manifest-substitution case the exact-set
    // rule is really guarding, so identity is compared as well as membership.
    for surface in surfaces {
        let declared = HOST_SURFACES
            .iter()
            .find(|candidate| candidate.name == surface.name)
            .ok_or_else(|| DiscoveryError::HostSetNotExact {
                detail: format!("{} is not a supported host", surface.name),
            })?;
        if declared != surface {
            return Err(DiscoveryError::HostSetNotExact {
                detail: format!("{} does not match its declared surface", surface.name),
            });
        }
    }
    Ok(())
}

/// Validate that the bundle holds exactly the one canonical skill file.
///
/// # Errors
///
/// Returns [`DiscoveryError::SkillInventoryNotExact`] for an absent canonical
/// skill or for any second skill alongside it.
pub fn validate_skill_inventory(observed: &BTreeSet<String>) -> Result<(), DiscoveryError> {
    if observed.len() == 1 && observed.contains(CANONICAL_SKILL_FILE) {
        return Ok(());
    }
    Err(DiscoveryError::SkillInventoryNotExact {
        observed: observed.iter().cloned().collect(),
    })
}

/// Validate that the canonical skill exposes exactly the promoted workflows.
///
/// # Errors
///
/// Returns [`DiscoveryError::WorkflowSetNotExact`] when a promoted workflow is
/// absent or an unpromoted definition is exposed.
pub fn validate_workflow_set(observed: &BTreeSet<String>) -> Result<(), DiscoveryError> {
    if *observed == expected_workflows() {
        return Ok(());
    }
    Err(DiscoveryError::WorkflowSetNotExact {
        observed: observed.iter().cloned().collect(),
    })
}

/// Validate that every supported host resolved the one canonical skill file.
///
/// Agreement is decided here rather than per manifest on purpose. A manifest
/// validated in isolation against a hardcoded canonical answer can only ever
/// produce that answer, so two manifests pointing at different in-bundle skill
/// trees — the drift this module exists to catch — would be unrepresentable
/// before any check saw them. Each manifest reports the target it actually
/// named, and the disagreement is a failure the set can have.
///
/// # Errors
///
/// Returns [`DiscoveryError::HostSetNotExact`] when the resolutions do not
/// cover the supported hosts exactly once,
/// [`DiscoveryError::DivergentResolution`] when they name more than one file,
/// and [`DiscoveryError::TargetNotCanonical`] when they agree on a file that is
/// not the canonical one.
pub fn validate_resolution(resolved: &[ResolvedSkill]) -> Result<(), DiscoveryError> {
    let hosts = resolved
        .iter()
        .map(|entry| entry.surface)
        .collect::<BTreeSet<_>>();
    let supported = HOST_SURFACES
        .iter()
        .map(|surface| surface.name)
        .collect::<BTreeSet<_>>();
    if hosts.len() != resolved.len() || hosts != supported {
        return Err(DiscoveryError::HostSetNotExact {
            detail: format!("{} resolutions name {} hosts", resolved.len(), hosts.len()),
        });
    }
    let files = resolved
        .iter()
        .map(|entry| entry.skill_file.clone())
        .collect::<BTreeSet<_>>();
    let named = files.iter().cloned().collect::<Vec<_>>();
    let [agreed] = named.as_slice() else {
        return Err(DiscoveryError::DivergentResolution { observed: named });
    };
    if agreed != CANONICAL_SKILL_FILE {
        let surface = resolved
            .first()
            .map_or(HostSurfaceName::ClaudeCode, |entry| entry.surface);
        return Err(DiscoveryError::TargetNotCanonical {
            surface,
            observed: agreed.clone(),
        });
    }
    Ok(())
}

/// Extract the one declared skill source from a manifest's `skills` value.
fn skill_source<'a>(
    surface: &HostSurface,
    value: Option<&'a Value>,
) -> Result<&'a str, DiscoveryError> {
    let shape = |detail| DiscoveryError::SkillSourceShapeInvalid {
        surface: surface.name,
        detail,
    };
    let scalar = match surface.source_shape {
        SkillSourceShape::SingleString => value.ok_or_else(|| shape("skills is absent"))?,
        SkillSourceShape::SingleElementArray => {
            let entries = value
                .and_then(Value::as_array)
                .ok_or_else(|| shape("skills is not an array"))?;
            match entries.as_slice() {
                [only] => only,
                _ => return Err(shape("skills does not hold exactly one entry")),
            }
        }
    };
    let source = scalar
        .as_str()
        .ok_or_else(|| shape("skills source is not a string"))?;
    if source.trim().is_empty() {
        return Err(shape("skills source is blank"));
    }
    Ok(source)
}

/// Normalize a declared target to a bundle-relative path, or refuse it.
///
/// Refusal is deliberately lexical and total: an absolute path, a Windows drive
/// or backslash spelling, a parent-directory component, an embedded NUL, or an
/// overlong target never reaches a host adapter that would have to resolve it.
/// A `.` component and a repeated separator are normalized away rather than
/// refused, because they change nothing about where the path lands.
fn normalize_bundle_relative(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > MAX_TARGET_BYTES {
        return None;
    }
    if value.starts_with('/') || value.contains('\\') || value.contains('\0') {
        return None;
    }
    // A Windows drive prefix is one ASCII letter and a colon. Refusing every
    // second-position colon would reject an ordinary directory name.
    let mut prefix = value.chars();
    if prefix
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && prefix.next() == Some(':')
    {
        return None;
    }
    let mut components = Vec::new();
    for component in value.split('/') {
        match component {
            "" | "." => {}
            ".." => return None,
            other => components.push(other),
        }
    }
    if components.is_empty() {
        return None;
    }
    Some(components.join("/"))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::*;

    #[trace("TC-013", "FR-002-AC-5", "TC-041", "FR-002-CON-1")]
    #[test]
    fn tc_013_bundle_relative_targets_and_host_sets_refuse_lexically() {
        // Every spelling below reaches a host adapter as a path it would have to
        // open. The refusal has to happen before that, in a module that cannot
        // open anything, or the bundle boundary is enforced by whichever adapter
        // happens to look.
        for escaping in [
            "../../outside",
            "..",
            "engineering_assurance/../../outside",
            "/engineering_assurance/skills",
            "C:/engineering_assurance/skills",
            "engineering_assurance\\skills",
            "engineering_assurance/skills\0",
            "",
            "./",
        ] {
            assert_eq!(
                normalize_bundle_relative(escaping),
                None,
                "{escaping:?} must not normalize to a bundle-relative path"
            );
        }
        // A redundant component changes nothing about where the path lands, so
        // refusing it would reject a correct manifest for a cosmetic reason.
        assert_eq!(
            normalize_bundle_relative("./engineering_assurance//skills").as_deref(),
            Some(CANONICAL_SKILLS_DIRECTORY)
        );

        let supported = HOST_SURFACES.to_vec();
        assert!(validate_surface_set(&supported).is_ok());
        assert!(validate_surface_set(&supported[..3]).is_err());
        let mut duplicated = supported.clone();
        duplicated.push(HOST_SURFACES[0]);
        assert!(validate_surface_set(&duplicated).is_err());
        // A substituted surface keeps four distinct host names while changing
        // where one of them looks, which membership alone would accept.
        let mut substituted = supported;
        substituted[0].manifest = ".claude-plugin/other.json";
        assert!(validate_surface_set(&substituted).is_err());
    }

    #[trace("TC-012", "FR-002-AC-4", "TC-042", "FR-002-CON-2")]
    #[test]
    fn tc_012_thin_manifests_refuse_behaviour_and_second_targets() {
        let claude = HOST_SURFACES[0];
        let opencode = HOST_SURFACES[2];
        let valid = json!({"name": "a", "skills": CANONICAL_SKILLS_DIRECTORY});
        assert_eq!(
            validate_manifest_document(&claude, &valid)
                .expect("a thin manifest must resolve")
                .skill_file,
            CANONICAL_SKILL_FILE
        );

        // The failure this prevents is a host carrying its own copy of the
        // onboarding behaviour: the copy drifts, and every host still reports a
        // successful discovery while running different instructions.
        let mut behavioural = valid.clone();
        behavioural["instructions"] = json!("copied behavioral instructions");
        assert_eq!(
            validate_manifest_document(&claude, &behavioural)
                .expect_err("embedded behaviour must refuse")
                .code(),
            "discovery_manifest_invalid"
        );

        // Two declared sources are two answers to a question with one answer.
        for shape in [
            json!({"$schema": "s", "skills": []}),
            json!({"$schema": "s", "skills": [CANONICAL_SKILLS_DIRECTORY, "other"]}),
            json!({"$schema": "s", "skills": CANONICAL_SKILLS_DIRECTORY}),
            json!({"$schema": "s", "skills": [" "]}),
            json!({"$schema": "s"}),
        ] {
            assert_eq!(
                validate_manifest_document(&opencode, &shape)
                    .expect_err("a non-singular skill source must refuse")
                    .code(),
                "discovery_manifest_invalid",
                "shape unexpectedly accepted: {shape}"
            );
        }
        assert!(
            validate_manifest_document(
                &opencode,
                &json!({"$schema": "s", "skills": [CANONICAL_SKILLS_DIRECTORY]}),
            )
            .is_ok()
        );
        assert!(validate_manifest_document(&claude, &json!([])).is_err());
    }

    #[trace("TC-010", "FR-002-AC-2", "TC-038", "NFR-001-AC-1")]
    #[test]
    fn tc_010_two_hosts_pointing_at_different_skill_trees_refuse() {
        let resolved = |source: &str| {
            HOST_SURFACES
                .into_iter()
                .map(|surface| {
                    validate_manifest_document(
                        &surface,
                        &match surface.source_shape {
                            SkillSourceShape::SingleString => json!({"skills": source}),
                            SkillSourceShape::SingleElementArray => {
                                json!({"$schema": "s", "skills": [source]})
                            }
                        },
                    )
                    .expect("an in-bundle source must survive the document check")
                })
                .collect::<Vec<_>>()
        };

        let agreeing = resolved(CANONICAL_SKILLS_DIRECTORY);
        assert!(validate_resolution(&agreeing).is_ok());

        // This is the drift the four manifests exist to make impossible: two
        // hosts install different behaviour, each reports a successful
        // discovery, and nothing downstream records which one ran. It has to be
        // representable to be refusable, so the manifest check reports the
        // target named rather than the target required.
        let mut divergent = agreeing.clone();
        divergent[1] = resolved("engineering_assurance/other-skills")[1].clone();
        assert_eq!(
            validate_resolution(&divergent)
                .expect_err("hosts naming different trees must refuse")
                .code(),
            "discovery_resolution_divergent"
        );

        // Unanimity is not canonicality: all four agreeing on the wrong tree is
        // still the wrong tree.
        assert_eq!(
            validate_resolution(&resolved("engineering_assurance/other-skills"))
                .expect_err("an agreed non-canonical tree must refuse")
                .code(),
            "discovery_target_not_canonical"
        );

        // A resolution short of the supported set cannot be read as agreement
        // either, or a host that failed to resolve would improve the result.
        assert_eq!(
            validate_resolution(&agreeing[..3])
                .expect_err("a short resolution must refuse")
                .code(),
            "discovery_host_set_not_exact"
        );
    }

    #[trace("TC-041", "FR-002-CON-1")]
    #[test]
    fn tc_041_host_identity_has_one_wire_spelling() {
        // The wire spelling is declared twice — by the serde attribute and by
        // as_str — and a variant rename would silently split them, leaving a
        // recorded host name that matches nothing the set is checked against.
        for surface in HOST_SURFACES {
            let encoded = serde_json::to_string(&surface.name).expect("a host name must serialize");
            assert_eq!(encoded, format!("{:?}", surface.name.as_str()));
        }
    }

    #[trace("TC-009", "FR-002-AC-1", "TC-011", "FR-002-AC-3")]
    #[test]
    fn tc_009_skill_and_workflow_inventories_are_exact_sets() {
        // A second skill file is the drift this refuses: both validate, both are
        // discoverable, and nothing says which one a host should have run.
        assert!(validate_skill_inventory(&BTreeSet::new()).is_err());
        assert!(
            validate_skill_inventory(&BTreeSet::from([CANONICAL_SKILL_FILE.to_owned()])).is_ok()
        );
        assert!(
            validate_skill_inventory(&BTreeSet::from([
                CANONICAL_SKILL_FILE.to_owned(),
                "engineering_assurance/skills/other/SKILL.md".to_owned(),
            ]))
            .is_err()
        );

        // The promoted set is compared in both directions. A missing workflow is
        // an unusable bundle; an extra one is a workflow that can be discovered
        // and started but never bound, because binding uses the same list.
        assert!(validate_workflow_set(&expected_workflows()).is_ok());
        assert!(validate_workflow_set(&BTreeSet::new()).is_err());
        let mut short = expected_workflows();
        short.remove("change-assurance");
        assert!(validate_workflow_set(&short).is_err());
        let mut long = expected_workflows();
        long.insert("unpromoted".to_owned());
        assert!(validate_workflow_set(&long).is_err());
        assert_eq!(expected_workflows().len(), WorkflowKind::ALL.len());
    }
}

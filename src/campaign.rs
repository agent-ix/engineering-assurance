// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed, versioned campaign contracts and resolution into FR-019 requests.
//!
//! The wire records come exclusively from FCD's generated crate. This module
//! validates cross-record invariants and binds an authored procedure to exact
//! caller-selected executable, input, adapter, and source identities. It does
//! not execute producers or interpret their domain observations.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use thiserror::Error;

pub use agent_ix_engineering_assurance_campaign::{
    CampaignAttempt, CampaignAttemptStatus, CampaignCompletionRule, CampaignDefinition,
    CampaignMember, CampaignRawArtifact, CampaignRun, CampaignSource, CampaignVerdict,
    MeasurementProcedure, ProcedureArgument, ProcedureArgumentKind, ProcedureArtifact,
    ProcedureEnvironment, ProcedureEnvironmentKind, ProcedureInputOrigin, ProcedureInputOriginKind,
    ProcedureInputPrefix,
};

use crate::producer_execution::{
    ArgumentBinding, CancellationBinding, ContainmentBinding, ContentDigest, ContractBinding,
    ExecutionBudget, ExecutionProcedure, ExitCodeBinding, InputBinding, InvalidExecutionRequest,
    MAX_ARGUMENTS, MAX_ARTIFACTS, MAX_TIMEOUT_MILLIS, OutputBinding, OutputTreeBinding,
    PRODUCER_EXECUTION_REQUEST_PROTOCOL, ProducerDescriptor, ProducerExecutionRequest,
    RequestIdentity, ResponseBinding, StdinBinding, validate_source_tree,
};

/// The authored measurement-procedure discriminator.
pub const MEASUREMENT_PROCEDURE_VERSION: &str = "engineering-assurance.measurement-procedure/v1";
/// The authored campaign-definition discriminator.
pub const CAMPAIGN_DEFINITION_VERSION: &str = "engineering-assurance.campaign-definition/v1";
/// The retained campaign-run discriminator.
pub const CAMPAIGN_RUN_VERSION: &str = "engineering-assurance.campaign-run/v1";
/// Maximum number of named members in one campaign.
pub const MAX_CAMPAIGN_MEMBERS: usize = 4_096;
/// Maximum retained attempts in one run.
pub const MAX_CAMPAIGN_ATTEMPTS: usize = 65_536;

/// One available `MeasurementPlan` and its exact current procedure.
#[derive(Clone, Copy, Debug)]
pub struct PlanRegistration<'a> {
    /// Plan identity.
    pub id: &'a str,
    /// Exact `definition_version` in the plan frontmatter.
    pub definition_version: &'a str,
    /// Loaded typed procedure, absent for older non-runnable plans.
    pub procedure: Option<&'a MeasurementProcedure>,
}

/// Exact runtime bindings supplied by the caller for one producer invocation.
///
/// The authored procedure controls producer name/version, argument order,
/// artifact roles, response contract identity, repetition count, and timeout.
/// These bindings supply retained-byte identities and local capability paths.
#[derive(Clone, Debug)]
pub struct ProcedureBindings {
    /// Exact selected producer executable and provenance.
    pub producer: ProducerDescriptor,
    /// Caller domain contract bound to the request.
    pub caller: ContractBinding,
    /// Absolute capability root used by FR-019.
    pub capability_root: String,
    /// Complete explicit child environment.
    pub environment: BTreeMap<String, String>,
    /// Exact selected input files.
    pub inputs: Vec<InputBinding>,
    /// Claimed origins of selected explicit inputs, derived from the caller's
    /// source/dependency selectors. Quoin must independently verify the bytes.
    pub input_origins: Option<Vec<ProcedureInputOrigin>>,
    /// Optional complete tracked source tree staged as implicit input files.
    pub source_tree: Option<SourceTreeBinding>,
    /// Declared output paths.
    pub outputs: Vec<OutputBinding>,
    /// Declared bounded directories of dynamic outputs.
    pub output_trees: Vec<OutputTreeBinding>,
    /// Selected stdin source.
    pub stdin: StdinBinding,
    /// Cooperative confinement contract.
    pub containment: ContainmentBinding,
    /// Cancellation binding.
    pub cancellation: CancellationBinding,
    /// Execution limits; authored timeout replaces `timeout_millis`.
    pub budget: ExecutionBudget,
    /// Exact response protocol implementation.
    pub response_protocol: ContractBinding,
    /// Exact response adapter implementation.
    pub response_adapter: ContractBinding,
    /// Exit-code policy of the response adapter.
    pub exit_codes: ExitCodeBinding,
}

/// A Git source tree selected for sealed per-file staging by FR-019.
///
/// `manifest` is the exact NUL-delimited stdout of
/// `git ls-tree -r -z --full-tree <revision>`. Its SHA-256 must equal the
/// `CampaignSource` digest. Every regular tracked file is opened under the
/// capability root and checked against its Git blob OID before selection as
/// an [`InputBinding`] with `role = "source/" + path`. FR-019 later rechecks
/// its SHA-256 before staging. Tracked symlinks must be present at preflight:
/// their target bytes are verified against their Git blob OIDs at resolution
/// and recorded as omitted, but no symlink is staged or followed. Omitted-link
/// identity is resolution evidence; it is not checked again at launch because
/// the link cannot affect execution in the staged directory. Other modes
/// refuse the tree.
/// The caller and independent checker must establish that these manifest bytes
/// came from `CampaignSource.revision` in the retained Git repository; this
/// pure binding alone cannot derive a commit's tree from its self-reported ID.
#[derive(Clone, Debug)]
pub struct SourceTreeBinding {
    /// Repository identity in the campaign source graph.
    pub repository: String,
    /// Exact raw Git tree inventory bytes.
    pub manifest: Vec<u8>,
}

/// A source symlink bound by the Git manifest but omitted from the safe staging projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedSourceLink {
    /// Tracked relative path.
    pub path: String,
    /// SHA-256 of the link target bytes after Git blob OID verification.
    pub digest: ContentDigest,
}

/// A resolved, identity-bearing FR-019 request.
#[derive(Clone, Debug)]
pub struct ResolvedProcedure {
    /// Closed request handed to the bounded EA executor.
    pub request: ProducerExecutionRequest,
    /// SHA-256-JCS identity of the exact request.
    pub identity: RequestIdentity,
    /// Tracked symlinks verified at resolution and omitted from staging.
    pub omitted_source_links: Vec<OmittedSourceLink>,
}

/// Invalid campaign, procedure, or binding.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CampaignError {
    /// A generated record has the wrong explicit protocol version.
    #[error("unsupported {kind} protocol version `{actual}`")]
    Version {
        /// Record kind.
        kind: &'static str,
        /// Received version.
        actual: String,
    },
    /// A required string is empty or whitespace.
    #[error("{field} must be nonempty")]
    Empty {
        /// Field path.
        field: &'static str,
    },
    /// A collection required by the runnable profile is empty.
    #[error("{field} must contain at least one entry")]
    NoEntries {
        /// Field path.
        field: &'static str,
    },
    /// A role, source, member, or dependency repeats.
    #[error("duplicate {kind} `{name}`")]
    Duplicate {
        /// Entity kind.
        kind: &'static str,
        /// Duplicated identity.
        name: String,
    },
    /// A plan or dependency name cannot be resolved.
    #[error("unresolved {kind} `{name}`")]
    Unresolved {
        /// Entity kind.
        kind: &'static str,
        /// Missing identity.
        name: String,
    },
    /// The declared plan version differs from the selected plan.
    #[error("measurement plan `{plan}` definition version differs")]
    PlanVersion {
        /// Plan identity.
        plan: String,
    },
    /// A required plan lacks a typed runnable procedure.
    #[error("measurement plan `{plan}` has no execution_procedure")]
    MissingProcedure {
        /// Plan identity.
        plan: String,
    },
    /// A dependency cycle prevents a finite execution order.
    #[error("campaign dependency cycle")]
    DependencyCycle,
    /// The procedure's repetition count or timeout is outside bounds.
    #[error("invalid procedure {field}")]
    Limit {
        /// Bounded field.
        field: &'static str,
    },
    /// A resolved binding does not match the authored contract or source graph.
    #[error("procedure binding mismatch at {field}")]
    Binding {
        /// Binding field.
        field: &'static str,
    },
    /// An authored or retained digest is not lowercase SHA-256 hexadecimal.
    #[error("invalid SHA-256 digest at {field}")]
    Digest {
        /// Digest field.
        field: &'static str,
    },
    /// FR-019 rejected the constructed request before identity minting.
    #[error("invalid resolved producer request: {0}")]
    InvalidRequest(#[from] InvalidExecutionRequest),
    /// Canonical encoding of the generated contract failed.
    #[error("campaign contract cannot be encoded canonically")]
    Encoding,
    /// The source manifest and selected file population disagree.
    #[error("invalid source tree: {field}")]
    SourceTree {
        /// Invalid source-tree component.
        field: &'static str,
    },
}

fn nonempty(value: &str, field: &'static str) -> Result<(), CampaignError> {
    if value.trim().is_empty() {
        Err(CampaignError::Empty { field })
    } else {
        Ok(())
    }
}

fn valid_input_role_prefix(value: &str) -> bool {
    let Some(components) = value.strip_suffix('/') else {
        return false;
    };
    !components.is_empty()
        && value.len() <= 256
        && components.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

fn digest(value: &str, field: &'static str) -> Result<(), CampaignError> {
    ContentDigest::parse(value)
        .map(|_| ())
        .map_err(|_| CampaignError::Digest { field })
}

/// Validates one FCD-generated authored procedure before any request is minted.
///
/// # Errors
/// Returns [`CampaignError`] for an invalid version, role, bound, or argument.
pub fn validate_procedure(procedure: &MeasurementProcedure) -> Result<(), CampaignError> {
    if procedure.schema_version != MEASUREMENT_PROCEDURE_VERSION {
        return Err(CampaignError::Version {
            kind: "measurement procedure",
            actual: procedure.schema_version.clone(),
        });
    }
    nonempty(&procedure.producer_name, "producerName")?;
    nonempty(&procedure.producer_version, "producerVersion")?;
    nonempty(&procedure.source_repository, "sourceRepository")?;
    nonempty(&procedure.response_protocol, "responseProtocol")?;
    nonempty(&procedure.response_adapter, "responseAdapter")?;
    nonempty(
        &procedure.response_adapter_version,
        "responseAdapterVersion",
    )?;
    if procedure.repetitions < 1 || procedure.repetitions > i64::from(u16::MAX) {
        return Err(CampaignError::Limit {
            field: "repetitions",
        });
    }
    if procedure.timeout_millis < 1
        || u64::try_from(procedure.timeout_millis)
            .ok()
            .is_none_or(|n| n > MAX_TIMEOUT_MILLIS)
    {
        return Err(CampaignError::Limit {
            field: "timeoutMillis",
        });
    }
    let mut inputs = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for artifact in procedure.inputs.as_deref().unwrap_or(&[]) {
        nonempty(&artifact.role, "inputs.role")?;
        if !inputs.insert(artifact.role.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "input role",
                name: artifact.role.clone(),
            });
        }
    }
    let input_prefixes = validate_input_prefixes(procedure, &inputs)?;
    validate_input_origins(procedure, &inputs)?;
    for artifact in procedure.outputs.as_deref().unwrap_or(&[]) {
        nonempty(&artifact.role, "outputs.role")?;
        if !outputs.insert(artifact.role.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "output role",
                name: artifact.role.clone(),
            });
        }
    }
    for artifact in procedure.output_trees.as_deref().unwrap_or(&[]) {
        nonempty(&artifact.role, "outputTrees.role")?;
        if !outputs.insert(artifact.role.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "output role",
                name: artifact.role.clone(),
            });
        }
    }
    let mut environment = BTreeSet::new();
    for entry in procedure.environment.as_deref().unwrap_or(&[]) {
        nonempty(&entry.name, "environment.name")?;
        nonempty(&entry.value, "environment.value")?;
        if !environment.insert(entry.name.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "environment name",
                name: entry.name.clone(),
            });
        }
    }
    let arguments = procedure.arguments.as_deref().unwrap_or(&[]);
    if arguments.len() > MAX_ARGUMENTS
        || inputs.len() > MAX_ARTIFACTS
        || input_prefixes.len() > MAX_ARTIFACTS
        || outputs.len() > MAX_ARTIFACTS
    {
        return Err(CampaignError::Limit {
            field: "argument or artifact population",
        });
    }
    for argument in arguments {
        nonempty(&argument.value, "arguments.value")?;
        match argument.kind {
            ProcedureArgumentKind::Literal => {}
            ProcedureArgumentKind::InputArtifact if inputs.contains(argument.value.as_str()) => {}
            ProcedureArgumentKind::OutputArtifact if outputs.contains(argument.value.as_str()) => {}
            _ => {
                return Err(CampaignError::Binding {
                    field: "arguments.role",
                });
            }
        }
    }
    Ok(())
}

fn validate_input_prefixes<'a>(
    procedure: &'a MeasurementProcedure,
    inputs: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, CampaignError> {
    let mut prefixes = BTreeSet::new();
    for declaration in procedure.input_role_prefixes.as_deref().unwrap_or(&[]) {
        if !valid_input_role_prefix(&declaration.prefix) {
            return Err(CampaignError::Binding {
                field: "inputRolePrefixes.prefix",
            });
        }
        if !prefixes.insert(declaration.prefix.as_str())
            || inputs
                .iter()
                .any(|role| role.starts_with(&declaration.prefix))
            || prefixes.iter().any(|other| {
                *other != declaration.prefix.as_str()
                    && (other.starts_with(&declaration.prefix)
                        || declaration.prefix.starts_with(*other))
            })
        {
            return Err(CampaignError::Duplicate {
                kind: "input role prefix",
                name: declaration.prefix.clone(),
            });
        }
    }
    Ok(prefixes)
}

fn validate_input_origins(
    procedure: &MeasurementProcedure,
    inputs: &BTreeSet<&str>,
) -> Result<(), CampaignError> {
    // Absence preserves the pre-origin contract while current source-pinned
    // Campaigns migrate. A present declaration is always a complete inventory.
    let Some(origins) = &procedure.input_origins else {
        return Ok(());
    };
    let mut roles = BTreeSet::new();
    for origin in origins {
        if !inputs.contains(origin.role.as_str()) {
            return Err(CampaignError::Binding {
                field: "inputOrigins.role",
            });
        }
        if !roles.insert(origin.role.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "input origin role",
                name: origin.role.clone(),
            });
        }
        let source = origin
            .source_repository
            .as_deref()
            .zip(origin.source_path.as_deref());
        let dependency = origin
            .dependency_member
            .as_deref()
            .zip(origin.dependency_artifact_role.as_deref());
        match origin.kind {
            ProcedureInputOriginKind::SelectedBytes
                if origin.source_repository.is_none()
                    && origin.source_path.is_none()
                    && origin.dependency_member.is_none()
                    && origin.dependency_artifact_role.is_none() => {}
            ProcedureInputOriginKind::SourceFile
                if source.is_some_and(|(repository, path)| {
                    !repository.is_empty() && valid_source_path(path)
                }) && origin.dependency_member.is_none()
                    && origin.dependency_artifact_role.is_none() => {}
            ProcedureInputOriginKind::Dependency
                if dependency
                    .is_some_and(|(member, role)| !member.is_empty() && !role.is_empty())
                    && origin.source_repository.is_none()
                    && origin.source_path.is_none() => {}
            _ => {
                return Err(CampaignError::Binding {
                    field: "inputOrigins.kind",
                });
            }
        }
    }
    if roles.len() != inputs.len() {
        return Err(CampaignError::Binding {
            field: "inputOrigins.complete",
        });
    }
    Ok(())
}

fn valid_source_path(path: &str) -> bool {
    !path.is_empty()
        && !path.as_bytes().contains(&0)
        && !std::path::Path::new(path).is_absolute()
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

/// Validates a runnable campaign against the exact available plans.
///
/// # Errors
/// Returns [`CampaignError`] for unresolved or stale plans, duplicate IDs,
/// invalid procedures, unknown dependencies, or dependency cycles.
pub fn validate_definition(
    definition: &CampaignDefinition,
    plans: &[PlanRegistration<'_>],
) -> Result<(), CampaignError> {
    if definition.schema_version != CAMPAIGN_DEFINITION_VERSION {
        return Err(CampaignError::Version {
            kind: "campaign definition",
            actual: definition.schema_version.clone(),
        });
    }
    nonempty(&definition.id, "id")?;
    nonempty(&definition.subject_name, "subjectName")?;
    nonempty(&definition.subject_version, "subjectVersion")?;
    if definition.members.is_empty() {
        return Err(CampaignError::NoEntries { field: "members" });
    }
    if definition.members.len() > MAX_CAMPAIGN_MEMBERS {
        return Err(CampaignError::Limit { field: "members" });
    }
    if definition.source_graph.is_empty() {
        return Err(CampaignError::NoEntries {
            field: "sourceGraph",
        });
    }
    let mut sources = BTreeSet::new();
    for source in &definition.source_graph {
        nonempty(&source.repository, "sourceGraph.repository")?;
        nonempty(&source.revision, "sourceGraph.revision")?;
        digest(&source.digest, "sourceGraph.digest")?;
        if !sources.insert(source.repository.as_str()) {
            return Err(CampaignError::Duplicate {
                kind: "source repository",
                name: source.repository.clone(),
            });
        }
    }
    let mut registrations = BTreeMap::new();
    for plan in plans {
        if registrations.insert(plan.id, plan).is_some() {
            return Err(CampaignError::Duplicate {
                kind: "plan",
                name: plan.id.to_owned(),
            });
        }
    }
    let mut members = BTreeMap::new();
    for member in &definition.members {
        nonempty(&member.name, "members.name")?;
        if let Some(group) = &member.group {
            nonempty(group, "members.group")?;
        }
        if members.insert(member.name.as_str(), member).is_some() {
            return Err(CampaignError::Duplicate {
                kind: "member",
                name: member.name.clone(),
            });
        }
        let plan = registrations.get(member.plan_id.as_str()).ok_or_else(|| {
            CampaignError::Unresolved {
                kind: "measurement plan",
                name: member.plan_id.clone(),
            }
        })?;
        if member.definition_version != plan.definition_version {
            return Err(CampaignError::PlanVersion {
                plan: member.plan_id.clone(),
            });
        }
        let procedure = plan
            .procedure
            .ok_or_else(|| CampaignError::MissingProcedure {
                plan: member.plan_id.clone(),
            })?;
        validate_procedure(procedure)?;
        validate_origin_context(procedure, member, &sources)?;
        if !sources.contains(procedure.source_repository.as_str()) {
            return Err(CampaignError::Unresolved {
                kind: "procedure source repository",
                name: procedure.source_repository.clone(),
            });
        }
        if let Some(checker) = &member.checker_procedure {
            validate_procedure(checker)?;
            validate_origin_context(checker, member, &sources)?;
            if !sources.contains(checker.source_repository.as_str()) {
                return Err(CampaignError::Unresolved {
                    kind: "checker source repository",
                    name: checker.source_repository.clone(),
                });
            }
        }
    }
    validate_member_dependencies(&members)?;
    validate_origin_artifact_roles(&members, &registrations)?;
    Ok(())
}

fn validate_origin_artifact_roles(
    members: &BTreeMap<&str, &CampaignMember>,
    registrations: &BTreeMap<&str, &PlanRegistration<'_>>,
) -> Result<(), CampaignError> {
    for member in members.values() {
        let plan = registrations.get(member.plan_id.as_str()).ok_or_else(|| {
            CampaignError::Unresolved {
                kind: "measurement plan",
                name: member.plan_id.clone(),
            }
        })?;
        for procedure in [plan.procedure, member.checker_procedure.as_ref()]
            .into_iter()
            .flatten()
        {
            for origin in procedure.input_origins.as_deref().unwrap_or(&[]) {
                if !matches!(origin.kind, ProcedureInputOriginKind::Dependency) {
                    continue;
                }
                let dependency =
                    origin
                        .dependency_member
                        .as_deref()
                        .ok_or(CampaignError::Binding {
                            field: "inputOrigins.kind",
                        })?;
                let role =
                    origin
                        .dependency_artifact_role
                        .as_deref()
                        .ok_or(CampaignError::Binding {
                            field: "inputOrigins.kind",
                        })?;
                let upstream =
                    members
                        .get(dependency)
                        .ok_or_else(|| CampaignError::Unresolved {
                            kind: "dependency",
                            name: dependency.to_owned(),
                        })?;
                let upstream_plan =
                    registrations
                        .get(upstream.plan_id.as_str())
                        .ok_or_else(|| CampaignError::Unresolved {
                            kind: "measurement plan",
                            name: upstream.plan_id.clone(),
                        })?;
                let upstream_procedure =
                    upstream_plan
                        .procedure
                        .ok_or_else(|| CampaignError::MissingProcedure {
                            plan: upstream.plan_id.clone(),
                        })?;
                let fixed = upstream_procedure
                    .outputs
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .any(|output| output.role == role);
                let in_tree = upstream_procedure
                    .output_trees
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .any(|tree| {
                        role.strip_prefix(&tree.role)
                            .and_then(|suffix| suffix.strip_prefix('/'))
                            .is_some_and(valid_source_path)
                    });
                if !fixed && !in_tree {
                    return Err(CampaignError::Binding {
                        field: "inputOrigins.dependencyArtifactRole",
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_origin_context(
    procedure: &MeasurementProcedure,
    member: &CampaignMember,
    sources: &BTreeSet<&str>,
) -> Result<(), CampaignError> {
    for origin in procedure.input_origins.as_deref().unwrap_or(&[]) {
        match origin.kind {
            ProcedureInputOriginKind::SourceFile => {
                let repository =
                    origin
                        .source_repository
                        .as_deref()
                        .ok_or(CampaignError::Binding {
                            field: "inputOrigins.kind",
                        })?;
                if !sources.contains(repository) {
                    return Err(CampaignError::Unresolved {
                        kind: "input origin source repository",
                        name: repository.to_owned(),
                    });
                }
            }
            ProcedureInputOriginKind::Dependency => {
                let dependency =
                    origin
                        .dependency_member
                        .as_deref()
                        .ok_or(CampaignError::Binding {
                            field: "inputOrigins.kind",
                        })?;
                if !member
                    .depends_on
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .any(|declared| declared == dependency)
                {
                    return Err(CampaignError::Binding {
                        field: "inputOrigins.dependencyMember",
                    });
                }
            }
            ProcedureInputOriginKind::SelectedBytes => {}
        }
    }
    Ok(())
}

fn validate_member_dependencies(
    members: &BTreeMap<&str, &CampaignMember>,
) -> Result<(), CampaignError> {
    for member in members.values() {
        let mut seen = BTreeSet::new();
        for dependency in member.depends_on.as_deref().unwrap_or(&[]) {
            if !seen.insert(dependency.as_str()) {
                return Err(CampaignError::Duplicate {
                    kind: "dependency",
                    name: dependency.clone(),
                });
            }
            if !members.contains_key(dependency.as_str()) {
                return Err(CampaignError::Unresolved {
                    kind: "dependency",
                    name: dependency.clone(),
                });
            }
        }
    }
    // Kahn's algorithm avoids recursive stack growth from authored chains.
    let mut remaining: BTreeMap<&str, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, member) in members {
        let deps = member.depends_on.as_deref().unwrap_or(&[]);
        remaining.insert(name, deps.len());
        for dependency in deps {
            dependents.entry(dependency).or_default().push(name);
        }
    }
    let mut ready: BTreeSet<&str> = remaining
        .iter()
        .filter_map(|(name, count)| (*count == 0).then_some(*name))
        .collect();
    let mut visited = 0;
    while let Some(name) = ready.pop_first() {
        visited += 1;
        for dependent in dependents.get(name).into_iter().flatten() {
            let count = remaining
                .get_mut(dependent)
                .ok_or_else(|| CampaignError::Unresolved {
                    kind: "dependency",
                    name: (*dependent).to_owned(),
                })?;
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent);
            }
        }
    }
    if visited != members.len() {
        return Err(CampaignError::DependencyCycle);
    }
    Ok(())
}

/// Resolves a typed procedure into one exact FR-019 request and identity.
///
/// # Errors
/// Returns [`CampaignError`] if any selected binding differs from the authored
/// procedure or source graph, or if FR-019 refuses the completed request.
pub fn resolve_procedure(
    procedure: &MeasurementProcedure,
    source_graph: &[CampaignSource],
    bindings: ProcedureBindings,
) -> Result<ResolvedProcedure, CampaignError> {
    validate_procedure(procedure)?;
    let source = source_graph
        .iter()
        .find(|source| source.repository == procedure.source_repository)
        .ok_or_else(|| CampaignError::Unresolved {
            kind: "procedure source repository",
            name: procedure.source_repository.clone(),
        })?;
    digest(&source.digest, "sourceGraph.digest")?;
    if source.revision != bindings.producer.source_revision {
        return Err(CampaignError::Binding {
            field: "producer.sourceRevision",
        });
    }
    if procedure.producer_name != bindings.producer.name
        || procedure.producer_version != bindings.producer.version
    {
        return Err(CampaignError::Binding { field: "producer" });
    }
    if procedure.response_protocol != bindings.response_protocol.kind {
        return Err(CampaignError::Binding {
            field: "response.protocol.kind",
        });
    }
    if procedure.response_adapter != bindings.response_adapter.kind
        || procedure.response_adapter_version != bindings.response_adapter.version
    {
        return Err(CampaignError::Binding {
            field: "response.adapter",
        });
    }
    validate_declared_environment(procedure, &bindings.environment)?;
    validate_bound_origins(procedure, &bindings)?;
    let (inputs, omitted_source_links) = extend_source_inputs(&bindings, source)?;
    validate_declared_artifacts(procedure, &bindings, &inputs)?;
    let arguments = procedure
        .arguments
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|argument| match argument.kind {
            ProcedureArgumentKind::Literal => ArgumentBinding::Literal {
                value: argument.value.clone(),
            },
            ProcedureArgumentKind::InputArtifact => ArgumentBinding::InputArtifact {
                role: argument.value.clone(),
            },
            ProcedureArgumentKind::OutputArtifact => ArgumentBinding::OutputArtifact {
                role: argument.value.clone(),
            },
        })
        .collect();
    let mut budget = bindings.budget;
    budget.timeout_millis =
        u64::try_from(procedure.timeout_millis).map_err(|_| CampaignError::Limit {
            field: "timeoutMillis",
        })?;
    let request = ProducerExecutionRequest {
        protocol: PRODUCER_EXECUTION_REQUEST_PROTOCOL.to_owned(),
        producer: bindings.producer,
        caller: bindings.caller,
        procedure: ExecutionProcedure::Direct,
        capability_root: bindings.capability_root,
        arguments,
        environment: bindings.environment,
        inputs,
        stdin: bindings.stdin,
        outputs: bindings.outputs,
        output_trees: bindings.output_trees,
        containment: bindings.containment,
        cancellation: bindings.cancellation,
        budget,
        response: ResponseBinding {
            protocol: bindings.response_protocol,
            adapter: bindings.response_adapter,
            exit_codes: bindings.exit_codes,
        },
    };
    let identity = request.identity()?;
    Ok(ResolvedProcedure {
        request,
        identity,
        omitted_source_links,
    })
}

fn validate_bound_origins(
    procedure: &MeasurementProcedure,
    bindings: &ProcedureBindings,
) -> Result<(), CampaignError> {
    let (Some(declared), Some(selected)) = (
        procedure.input_origins.as_deref(),
        bindings.input_origins.as_deref(),
    ) else {
        if procedure.input_origins.is_some() || bindings.input_origins.is_some() {
            return Err(CampaignError::Binding {
                field: "inputOrigins.binding",
            });
        }
        return Ok(());
    };
    let by_role: BTreeMap<_, _> = declared
        .iter()
        .map(|origin| (origin.role.as_str(), origin))
        .collect();
    let selected_roles: BTreeSet<_> = bindings
        .inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect();
    if selected_roles.len() != bindings.inputs.len()
        || selected_roles.len() != selected.len()
        || !selected_roles.iter().all(|role| by_role.contains_key(role))
    {
        return Err(CampaignError::Binding {
            field: "inputOrigins.binding",
        });
    }
    let mut seen = BTreeSet::new();
    for origin in selected {
        if !seen.insert(origin.role.as_str()) || by_role.get(origin.role.as_str()) != Some(&origin)
        {
            return Err(CampaignError::Binding {
                field: "inputOrigins.binding",
            });
        }
    }
    Ok(())
}

fn extend_source_inputs(
    bindings: &ProcedureBindings,
    source: &CampaignSource,
) -> Result<(Vec<InputBinding>, Vec<OmittedSourceLink>), CampaignError> {
    let mut inputs = bindings.inputs.clone();
    let mut omitted = Vec::new();
    if let Some(source_tree) = &bindings.source_tree {
        let (source_inputs, omitted_links) =
            validate_source_tree(source_tree, source, &bindings.capability_root)?;
        omitted = omitted_links;
        let explicit_roles: BTreeSet<_> = inputs.iter().map(|input| input.role.as_str()).collect();
        let explicit_paths: BTreeSet<_> = inputs.iter().map(|input| input.path.as_str()).collect();
        if source_inputs.iter().any(|input| {
            explicit_roles.contains(input.role.as_str())
                || explicit_paths.contains(input.path.as_str())
        }) {
            return Err(CampaignError::Binding {
                field: "source input collision",
            });
        }
        inputs.extend(source_inputs);
    }
    if inputs.len() > MAX_ARTIFACTS {
        return Err(CampaignError::Limit {
            field: "total input population",
        });
    }
    Ok((inputs, omitted))
}

fn validate_declared_environment(
    procedure: &MeasurementProcedure,
    environment: &BTreeMap<String, String>,
) -> Result<(), CampaignError> {
    let declared_environment: BTreeMap<_, _> = procedure
        .environment
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|entry| (entry.name.as_str(), entry))
        .collect();
    if environment.len() != declared_environment.len() {
        return Err(CampaignError::Binding {
            field: "environment",
        });
    }
    for (name, actual) in environment {
        let declaration =
            declared_environment
                .get(name.as_str())
                .ok_or(CampaignError::Binding {
                    field: "environment",
                })?;
        if matches!(declaration.kind, ProcedureEnvironmentKind::Literal)
            && actual != &declaration.value
        {
            return Err(CampaignError::Binding {
                field: "environment.literal",
            });
        }
    }
    Ok(())
}

fn validate_declared_artifacts(
    procedure: &MeasurementProcedure,
    bindings: &ProcedureBindings,
    inputs: &[InputBinding],
) -> Result<(), CampaignError> {
    let input_roles: BTreeMap<_, _> = inputs
        .iter()
        .map(|input| (input.role.as_str(), input))
        .collect();
    let output_roles: BTreeMap<_, _> = bindings
        .outputs
        .iter()
        .map(|output| (output.role.as_str(), output))
        .collect();
    let output_tree_roles: BTreeMap<_, _> = bindings
        .output_trees
        .iter()
        .map(|output| (output.role.as_str(), output))
        .collect();
    validate_required_inputs(procedure, &input_roles)?;
    for role in procedure.outputs.as_deref().unwrap_or(&[]) {
        if role.required && !output_roles.contains_key(role.role.as_str()) {
            return Err(CampaignError::Binding {
                field: "outputs.required",
            });
        }
        if output_roles
            .get(role.role.as_str())
            .is_some_and(|bound| bound.required != role.required)
        {
            return Err(CampaignError::Binding {
                field: "outputs.required mismatch",
            });
        }
    }
    for role in procedure.output_trees.as_deref().unwrap_or(&[]) {
        if role.required && !output_tree_roles.contains_key(role.role.as_str()) {
            return Err(CampaignError::Binding {
                field: "outputTrees.required",
            });
        }
        if output_tree_roles
            .get(role.role.as_str())
            .is_some_and(|bound| bound.required != role.required)
        {
            return Err(CampaignError::Binding {
                field: "outputTrees.required mismatch",
            });
        }
    }
    if input_roles.len() != inputs.len()
        || output_roles.len() != bindings.outputs.len()
        || output_tree_roles.len() != bindings.output_trees.len()
    {
        return Err(CampaignError::Binding {
            field: "artifact roles duplicated",
        });
    }
    let declared_inputs: BTreeSet<_> = procedure
        .inputs
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|artifact| artifact.role.as_str())
        .collect();
    let declared_input_prefixes: Vec<_> = procedure
        .input_role_prefixes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|declaration| declaration.prefix.as_str())
        .collect();
    let declared_outputs: BTreeSet<_> = procedure
        .outputs
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|artifact| artifact.role.as_str())
        .collect();
    let declared_output_trees: BTreeSet<_> = procedure
        .output_trees
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|artifact| artifact.role.as_str())
        .collect();
    if bindings.inputs.iter().any(|input| {
        let role = input.role.as_str();
        !declared_inputs.contains(role)
            && !declared_input_prefixes
                .iter()
                .any(|prefix| role.starts_with(prefix))
    }) || output_roles
        .keys()
        .any(|role| !declared_outputs.contains(role))
        || output_tree_roles
            .keys()
            .any(|role| !declared_output_trees.contains(role))
    {
        return Err(CampaignError::Binding {
            field: "artifact role undeclared",
        });
    }
    Ok(())
}

fn validate_required_inputs(
    procedure: &MeasurementProcedure,
    input_roles: &BTreeMap<&str, &InputBinding>,
) -> Result<(), CampaignError> {
    for role in procedure.inputs.as_deref().unwrap_or(&[]) {
        if role.required && !input_roles.contains_key(role.role.as_str()) {
            return Err(CampaignError::Binding {
                field: "inputs.required",
            });
        }
    }
    for declaration in procedure.input_role_prefixes.as_deref().unwrap_or(&[]) {
        if declaration.required
            && !input_roles
                .keys()
                .any(|role| role.starts_with(&declaration.prefix))
        {
            return Err(CampaignError::Binding {
                field: "inputRolePrefixes.required",
            });
        }
    }
    Ok(())
}

/// RFC 8785 SHA-256 identity of a generated `CampaignDefinition` or source graph.
///
/// # Errors
/// Returns [`CampaignError::Encoding`] if the value cannot be represented in
/// canonical JSON.
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<ContentDigest, CampaignError> {
    let bytes = serde_json_canonicalizer::to_vec(value).map_err(|_| CampaignError::Encoding)?;
    Ok(ContentDigest::of_bytes(&bytes))
}

/// Checks a retained run's identity bindings and attempt inventory shape.
/// Domain verdicts and raw bytes must be checked by Quoin and the domain
/// checker; this structural check never promotes evidence on its own.
///
/// # Errors
/// Returns [`CampaignError`] for a wrong version, stale identities, duplicate
/// attempts or unknown members, or malformed digest references.
pub fn validate_run(
    run: &CampaignRun,
    definition: &CampaignDefinition,
) -> Result<(), CampaignError> {
    if run.schema_version != CAMPAIGN_RUN_VERSION {
        return Err(CampaignError::Version {
            kind: "campaign run",
            actual: run.schema_version.clone(),
        });
    }
    nonempty(&run.id, "run.id")?;
    if run.definition_digest != canonical_digest(definition)?.as_str() {
        return Err(CampaignError::Binding {
            field: "definitionDigest",
        });
    }
    if run.source_graph_digest != canonical_digest(&definition.source_graph)?.as_str() {
        return Err(CampaignError::Binding {
            field: "sourceGraphDigest",
        });
    }
    let members: BTreeSet<_> = definition
        .members
        .iter()
        .map(|member| member.name.as_str())
        .collect();
    let mut attempts = BTreeSet::new();
    if run
        .attempts
        .as_ref()
        .is_some_and(|items| items.len() > MAX_CAMPAIGN_ATTEMPTS)
    {
        return Err(CampaignError::Limit { field: "attempts" });
    }
    for attempt in run.attempts.as_deref().unwrap_or(&[]) {
        if !members.contains(attempt.member.as_str()) {
            return Err(CampaignError::Unresolved {
                kind: "attempt member",
                name: attempt.member.clone(),
            });
        }
        if attempt.index < 1 {
            return Err(CampaignError::Limit {
                field: "attempt.index",
            });
        }
        if !attempts.insert((attempt.member.as_str(), attempt.index)) {
            return Err(CampaignError::Duplicate {
                kind: "attempt",
                name: format!("{}#{}", attempt.member, attempt.index),
            });
        }
        for value in [
            &attempt.request_digest,
            &attempt.result_digest,
            &attempt.collection_digest,
            &attempt.verdict_digest,
            &attempt.domain_verdict_digest,
            &attempt.checker_request_digest,
            &attempt.checker_result_digest,
        ]
        .into_iter()
        .flatten()
        {
            digest(value, "attempt digest")?;
        }
        if attempt.collection_id.is_some() != attempt.collection_digest.is_some() {
            return Err(CampaignError::Binding {
                field: "collectionId/collectionDigest",
            });
        }
        let mut roles = BTreeSet::new();
        for artifact in attempt.raw_artifacts.as_deref().unwrap_or(&[]) {
            nonempty(&artifact.role, "rawArtifacts.role")?;
            digest(&artifact.digest, "rawArtifacts.digest")?;
            if !roles.insert(artifact.role.as_str()) {
                return Err(CampaignError::Duplicate {
                    kind: "raw artifact role",
                    name: artifact.role.clone(),
                });
            }
        }
    }
    Ok(())
}

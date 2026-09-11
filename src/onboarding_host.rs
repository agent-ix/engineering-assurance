// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Filesystem and Quire adapter for the pure onboarding library boundary.

use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use engineering_assurance::onboarding::{
    ArtifactType, ArtifactValidation, Inventory, OnboardingPlan, OnboardingRequest,
    OnboardingResult, PublicationPlan, authored_result, frontmatter_type, plan,
    render_from_skeleton,
};
use thiserror::Error;

use crate::process_host::{self, ProcessLimits};

static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const QUIRE_VALIDATION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_QUIRE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub(crate) enum OnboardingHostError {
    #[error("invalid onboarding repository root: {detail}")]
    Root { detail: String },
    #[error("invalid installed Engineering Assurance module root: {detail}")]
    ModuleRoot { detail: String },
    #[error("onboarding inventory failed: {detail}")]
    Inventory { detail: String },
    #[error("invalid onboarding publication target: {detail}")]
    Target { detail: String },
    #[error("onboarding publication failed: {detail}")]
    Publication { detail: String },
}

impl OnboardingHostError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Root { .. } => "onboarding_root_invalid",
            Self::ModuleRoot { .. } => "onboarding_module_root_invalid",
            Self::Inventory { .. } => "onboarding_inventory_failed",
            Self::Target { .. } => "onboarding_target_invalid",
            Self::Publication { .. } => "onboarding_publication_failed",
        }
    }
}

pub(crate) fn execute(
    request: &OnboardingRequest,
) -> Result<OnboardingResult, OnboardingHostError> {
    let root = selected_directory(&request.repository_root, RootKind::Repository)?;
    let module_root = selected_directory(&request.module_root, RootKind::Module)?;
    let inventory =
        inventory_repository(&root, &module_root, OsStr::new(&request.quire_executable))?;
    match plan(request, inventory) {
        OnboardingPlan::Complete(result) => Ok(result),
        OnboardingPlan::Publish(publication) => {
            publish(&root, &module_root, &request.quire_executable, &publication)?;
            Ok(authored_result(publication))
        }
    }
}

#[derive(Clone, Copy)]
enum RootKind {
    Repository,
    Module,
}

fn selected_directory(value: &str, kind: RootKind) -> Result<PathBuf, OnboardingHostError> {
    let path = Path::new(value);
    let error = |detail: String| match kind {
        RootKind::Repository => OnboardingHostError::Root { detail },
        RootKind::Module => OnboardingHostError::ModuleRoot { detail },
    };
    if !path.is_absolute() {
        return Err(error("path must be absolute".to_owned()));
    }
    let resolved = fs::canonicalize(path).map_err(|source| error(source.to_string()))?;
    if !resolved.is_dir() {
        return Err(error("path is not a directory".to_owned()));
    }
    if matches!(kind, RootKind::Module)
        && (!resolved.join("manifest.yaml").is_file() || !resolved.join("skeletons").is_dir())
    {
        return Err(error(
            "path is not an installed Engineering Assurance module".to_owned(),
        ));
    }
    Ok(resolved)
}

fn inventory_repository(
    root: &Path,
    module_root: &Path,
    quire_executable: &OsStr,
) -> Result<Inventory, OnboardingHostError> {
    let mut inventory = Inventory::default();
    let mut pending_directories = vec![root.to_owned()];
    while let Some(directory) = pending_directories.pop() {
        visit_directory(
            root,
            &directory,
            module_root,
            quire_executable,
            &mut inventory,
            &mut pending_directories,
        )?;
    }
    inventory.sort();
    Ok(inventory)
}

fn visit_directory(
    root: &Path,
    directory: &Path,
    module_root: &Path,
    quire_executable: &OsStr,
    inventory: &mut Inventory,
    pending_directories: &mut Vec<PathBuf>,
) -> Result<(), OnboardingHostError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| OnboardingHostError::Inventory {
            detail: format!("cannot read {}: {source}", directory.display()),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| OnboardingHostError::Inventory {
            detail: format!("cannot enumerate {}: {source}", directory.display()),
        })?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let relative = relative_path(root, &path)?;
        let file_type = entry
            .file_type()
            .map_err(|source| OnboardingHostError::Inventory {
                detail: format!("cannot classify {relative}: {source}"),
            })?;
        if file_type.is_symlink() {
            if path.is_dir() {
                continue;
            }
            inventory
                .unresolved_inputs
                .push(format!("symlink-not-inspected:{relative}"));
            continue;
        }
        if file_type.is_dir() {
            if is_excluded_directory(&entry.file_name()) {
                continue;
            }
            pending_directories.push(path);
            continue;
        }
        if file_type.is_file() {
            inspect_file(
                root,
                &path,
                &relative,
                module_root,
                quire_executable,
                inventory,
            );
        }
    }
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> Result<String, OnboardingHostError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|source| OnboardingHostError::Inventory {
            detail: source.to_string(),
        })?;
    relative
        .to_str()
        .map(|value| value.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| OnboardingHostError::Inventory {
            detail: format!("non-UTF-8 repository path: {}", relative.display()),
        })
}

fn is_excluded_directory(name: &OsStr) -> bool {
    matches!(name.to_str(), Some(".git" | "node_modules" | "__pycache__"))
}

fn inspect_file(
    root: &Path,
    path: &Path,
    relative: &str,
    module_root: &Path,
    quire_executable: &OsStr,
    inventory: &mut Inventory,
) {
    let suffix = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if !matches!(suffix.as_deref(), Some("md" | "json" | "yaml" | "yml")) {
        return;
    }
    let relative_path = Path::new(relative);
    let lowered_parts = relative_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let mut raw_parts = relative_path
        .components()
        .filter_map(|component| component.as_os_str().to_str());
    let producer = matches!(
        (raw_parts.next(), raw_parts.next()),
        (Some(".github"), Some("workflows"))
    ) || path
        .file_stem()
        .and_then(OsStr::to_str)
        .is_some_and(|stem| stem.to_lowercase().contains("producer"))
        || lowered_parts.iter().any(|part| part == "producers");
    if producer {
        inventory.producer_configurations.push(relative.to_owned());
    } else if lowered_parts.iter().any(|part| part == "evidence") {
        inventory.evidence_references.push(relative.to_owned());
    }
    if suffix.as_deref() != Some("md") {
        return;
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) => {
            inventory.unresolved_inputs.push(format!(
                "unreadable:{}:{relative}",
                io_error_name(source.kind())
            ));
            return;
        }
    };
    let Ok(artifact_type) = frontmatter_type(&text) else {
        inventory
            .unresolved_inputs
            .push(format!("malformed-frontmatter:{relative}"));
        return;
    };
    let Some(type_name) = artifact_type else {
        return;
    };
    if let Ok(artifact_type) = ArtifactType::from_str(&type_name) {
        let validation = validate_artifact(
            root,
            path,
            relative,
            module_root,
            quire_executable,
            artifact_type,
        );
        if artifact_type == ArtifactType::MeasurementPlan {
            inventory.measurements.push(validation);
        } else {
            inventory.assurance_artifacts.push(validation);
        }
    } else if type_name.to_lowercase().contains("decision")
        || lowered_parts.iter().any(|part| part == "decisions")
    {
        inventory.decisions.push(relative.to_owned());
    }
}

fn io_error_name(kind: io::ErrorKind) -> &'static str {
    match kind {
        io::ErrorKind::InvalidData => "UnicodeDecodeError",
        _ => "OSError",
    }
}

fn validate_artifact(
    root: &Path,
    path: &Path,
    relative: &str,
    module_root: &Path,
    quire_executable: &OsStr,
    artifact_type: ArtifactType,
) -> ArtifactValidation {
    let arguments = [
        OsStr::new("validate"),
        OsStr::new("--module"),
        module_root.as_os_str(),
        OsStr::new("--strict"),
        path.as_os_str(),
    ];
    let output = process_host::run_configured(
        quire_executable,
        &arguments,
        Some(root),
        &[],
        &[],
        ProcessLimits {
            timeout: QUIRE_VALIDATION_TIMEOUT,
            max_output_bytes: MAX_QUIRE_OUTPUT_BYTES,
        },
    );
    let (valid, diagnostics) = match output {
        Ok(output) => (
            output.status.success(),
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect(),
        ),
        Err(_) => (false, vec!["quire-unavailable".to_owned()]),
    };
    ArtifactValidation {
        path: relative.to_owned(),
        artifact_type,
        valid,
        diagnostics,
    }
}

fn publish(
    root: &Path,
    module_root: &Path,
    quire_executable: &str,
    plan: &PublicationPlan,
) -> Result<(), OnboardingHostError> {
    let target = confined_target(&plan.target)?;
    let root_dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|source| publication_error(&source))?;
    match root_dir.symlink_metadata(&target) {
        Ok(_) => {
            return Err(OnboardingHostError::Target {
                detail: format!("artifact already exists: {}", plan.target),
            });
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(target_error(&source)),
    }
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    root_dir
        .create_dir_all(parent)
        .map_err(|source| target_error(&source))?;
    let skeleton_path = module_root
        .join("skeletons")
        .join(format!("{}.md", plan.artifact_type));
    let skeleton =
        fs::read_to_string(&skeleton_path).map_err(|source| OnboardingHostError::Publication {
            detail: format!("cannot read {}: {source}", skeleton_path.display()),
        })?;
    let content = render_from_skeleton(plan.artifact_type, &skeleton, &plan.frontmatter).map_err(
        |source| OnboardingHostError::Publication {
            detail: source.to_string(),
        },
    )?;
    let stage = stage_path(&target)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut staged = root_dir
        .open_with(&stage, &options)
        .map_err(|source| publication_error(&source))?;
    if let Err(source) = staged
        .write_all(content.as_bytes())
        .and_then(|()| staged.sync_all())
    {
        drop(staged);
        let _ = root_dir.remove_file(&stage);
        return Err(publication_error(&source));
    }
    drop(staged);
    let absolute_stage = root.join(&stage);
    let validation = validate_artifact(
        root,
        &absolute_stage,
        &stage.to_string_lossy(),
        module_root,
        OsStr::new(quire_executable),
        plan.artifact_type,
    );
    if !validation.valid {
        let _ = root_dir.remove_file(&stage);
        return Err(OnboardingHostError::Publication {
            detail: format!(
                "Quire rejected staged artifact: {}",
                validation.diagnostics.join("; ")
            ),
        });
    }
    if let Err(source) = root_dir.hard_link(&stage, &root_dir, &target) {
        let _ = root_dir.remove_file(&stage);
        return Err(target_error(&source));
    }
    let _ = root_dir.remove_file(&stage);
    Ok(())
}

fn confined_target(value: &str) -> Result<PathBuf, OnboardingHostError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        || path.file_name().is_none()
    {
        return Err(OnboardingHostError::Target {
            detail: format!("target is not a confined relative path: {value}"),
        });
    }
    Ok(path.to_owned())
}

fn stage_path(target: &Path) -> Result<PathBuf, OnboardingHostError> {
    let filename =
        target
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| OnboardingHostError::Target {
                detail: format!("target has no UTF-8 filename: {}", target.display()),
            })?;
    let sequence = STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let stage_name = format!(".{filename}.{}.{sequence}.staged", std::process::id());
    Ok(target
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(stage_name))
}

fn target_error(source: &io::Error) -> OnboardingHostError {
    OnboardingHostError::Target {
        detail: source.to_string(),
    }
}

fn publication_error(source: &io::Error) -> OnboardingHostError {
    OnboardingHostError::Publication {
        detail: source.to_string(),
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native launcher for explicit, manually invoked agent-evaluation runs.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use thiserror::Error;

use crate::{
    integration_evidence_host,
    process_host::{self, ProcessLimits},
};

const TIMEOUT: Duration = Duration::from_hours(2);
const MAX_OUTPUT: usize = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub(crate) enum AgentEvalsHostError {
    #[error("agent-evals repository root is invalid")]
    RootInvalid,
    #[error("agent-evals selected executable is unavailable")]
    ExecutableUnavailable,
    #[error("agent-evals governing snapshot is unavailable")]
    SnapshotUnavailable,
    #[error("agent-evals runner could not be observed")]
    RunnerUnavailable,
}

pub(crate) struct RunRequest<'a> {
    pub(crate) root: &'a Path,
    pub(crate) agent: &'a str,
    pub(crate) selector: &'a str,
    pub(crate) filter: Option<&'a str>,
    pub(crate) model: Option<&'a str>,
    pub(crate) keep: bool,
    pub(crate) report: &'a Path,
}

pub(crate) fn run(
    request: &RunRequest<'_>,
) -> Result<(ExitCode, Vec<u8>, Vec<u8>), AgentEvalsHostError> {
    let root = fs::canonicalize(request.root).map_err(|_| AgentEvalsHostError::RootInvalid)?;
    if !root.is_dir() {
        return Err(AgentEvalsHostError::RootInvalid);
    }
    let path = selected_path(&root).ok_or(AgentEvalsHostError::ExecutableUnavailable)?;
    let runner =
        executable(&path, "cli-evals").ok_or(AgentEvalsHostError::ExecutableUnavailable)?;
    let ix_flow = executable(&path, "ix-flow").ok_or(AgentEvalsHostError::ExecutableUnavailable)?;
    let host =
        executable(&path, request.agent).ok_or(AgentEvalsHostError::ExecutableUnavailable)?;
    let snapshot = integration_evidence_host::agent_evals_snapshot(&root, request.agent)
        .map_err(|_| AgentEvalsHostError::SnapshotUnavailable)?;
    let snapshot_path = root
        .join(".agent-evals")
        .join(format!("governing-{}.json", request.agent));
    fs::create_dir_all(
        snapshot_path
            .parent()
            .ok_or(AgentEvalsHostError::RootInvalid)?,
    )
    .map_err(|_| AgentEvalsHostError::SnapshotUnavailable)?;
    fs::write(
        &snapshot_path,
        serde_json::to_vec_pretty(&snapshot)
            .map_err(|_| AgentEvalsHostError::SnapshotUnavailable)?,
    )
    .map_err(|_| AgentEvalsHostError::SnapshotUnavailable)?;
    let mut args = vec![
        OsString::from("run"),
        OsString::from("--suite"),
        root.join("evals/cli-agent-evals.config.mjs")
            .into_os_string(),
    ];
    if let Some(filter) = request.filter {
        args.extend([OsString::from("--filter"), OsString::from(filter)]);
    } else {
        args.push(OsString::from(format!("--{}", request.selector)));
    }
    args.extend([OsString::from("--agent"), OsString::from(request.agent)]);
    if let Some(model) = request.model {
        args.extend([OsString::from("--model"), OsString::from(model)]);
    }
    if request.keep {
        args.push(OsString::from("--keep"));
    }
    args.extend([
        OsString::from("--report"),
        absolute(&root, request.report).into_os_string(),
    ]);
    let current = env::current_exe().map_err(|_| AgentEvalsHostError::ExecutableUnavailable)?;
    let environment = [
        (OsStr::new("PATH"), path.as_os_str()),
        (OsStr::new("IX_FLOW_BIN"), ix_flow.as_os_str()),
        (
            OsStr::new("EA_EVAL_GOVERNING_PATH"),
            snapshot_path.as_os_str(),
        ),
        (OsStr::new("ENGINEERING_ASSURANCE_BIN"), current.as_os_str()),
        (OsStr::new("EA_EVAL_AGENT_BIN"), host.as_os_str()),
    ];
    let refs = args.iter().map(OsString::as_os_str).collect::<Vec<_>>();
    let result = process_host::run_configured(
        runner.as_os_str(),
        &refs,
        Some(&root),
        &environment,
        &[],
        ProcessLimits {
            timeout: TIMEOUT,
            max_output_bytes: MAX_OUTPUT,
        },
    )
    .map_err(|_| AgentEvalsHostError::RunnerUnavailable)?;
    Ok((
        ExitCode::from(u8::try_from(result.status.code().unwrap_or(1)).unwrap_or(1)),
        result.stdout,
        result.stderr,
    ))
}

fn selected_path(root: &Path) -> Option<OsString> {
    env::join_paths(
        std::iter::once(root.join(".agent-evals/bin")).chain(
            env::var_os("PATH")
                .as_deref()
                .map(env::split_paths)
                .into_iter()
                .flatten(),
        ),
    )
    .ok()
}
fn executable(path: &OsStr, name: &str) -> Option<PathBuf> {
    env::split_paths(path)
        .map(|entry| entry.join(name))
        .find(|candidate| candidate.is_file())
        .and_then(|path| fs::canonicalize(path).ok())
}
fn absolute(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

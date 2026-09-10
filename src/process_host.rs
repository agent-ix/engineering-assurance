// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded child-process mechanics for binary-only host adapters.
//!
//! This module carries no producer, workflow, evaluation, or assurance
//! semantics. Callers remain responsible for their own command identity,
//! response protocol, and mutation uncertainty.

use std::{
    ffi::OsStr,
    io::{self, Read},
    path::Path,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;

#[derive(Clone, Copy)]
pub(crate) struct ProcessLimits {
    pub(crate) timeout: Duration,
    pub(crate) max_output_bytes: usize,
}

pub(crate) struct CompletedProcess {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

#[derive(Debug, Error)]
pub(crate) enum ProcessError {
    #[error("child process is unavailable: {detail}")]
    Unavailable { detail: String },
    #[error("child process {stream} pipe is unavailable")]
    PipeUnavailable { stream: &'static str },
    #[error("cannot observe child process: {detail}")]
    Observation { detail: String },
    #[error("child process exceeded the {timeout:?} time limit")]
    TimedOut { timeout: Duration },
    #[error("cannot read child process {stream}: {detail}")]
    OutputUnreadable {
        stream: &'static str,
        detail: String,
    },
    #[error("child process {stream} exceeded the {limit}-byte limit")]
    OutputTooLarge { stream: &'static str, limit: usize },
}

pub(crate) fn run(
    executable: &OsStr,
    arguments: &[&OsStr],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    run_configured(executable, arguments, None, &[], limits)
}

pub(crate) fn run_configured(
    executable: &OsStr,
    arguments: &[&OsStr],
    current_directory: Option<&Path>,
    environment: &[(&OsStr, &OsStr)],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(directory) = current_directory {
        command.current_dir(directory);
    }
    command.envs(environment.iter().copied());
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| ProcessError::Unavailable {
            detail: source.to_string(),
        })?;
    let Some(stdout) = child.stdout.take() else {
        terminate(&mut child);
        return Err(ProcessError::PipeUnavailable { stream: "stdout" });
    };
    let Some(stderr) = child.stderr.take() else {
        terminate(&mut child);
        return Err(ProcessError::PipeUnavailable { stream: "stderr" });
    };
    let stdout_reader = bounded_reader(stdout, limits.max_output_bytes);
    let stderr_reader = bounded_reader(stderr, limits.max_output_bytes);

    let started = Instant::now();
    let process_result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok((status, false)),
            Ok(None) if started.elapsed() < limits.timeout => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None) => {
                let _ = child.kill();
                break child.wait().map(|status| (status, true)).map_err(|source| {
                    ProcessError::Observation {
                        detail: format!("cannot reap timed-out child: {source}"),
                    }
                });
            }
            Err(source) => {
                terminate(&mut child);
                break Err(ProcessError::Observation {
                    detail: source.to_string(),
                });
            }
        }
    };

    let stdout = join_reader(stdout_reader, "stdout")?;
    let stderr = join_reader(stderr_reader, "stderr")?;
    let (status, timed_out) = process_result?;
    if timed_out {
        return Err(ProcessError::TimedOut {
            timeout: limits.timeout,
        });
    }
    if stdout.overflow {
        return Err(ProcessError::OutputTooLarge {
            stream: "stdout",
            limit: limits.max_output_bytes,
        });
    }
    if stderr.overflow {
        return Err(ProcessError::OutputTooLarge {
            stream: "stderr",
            limit: limits.max_output_bytes,
        });
    }
    Ok(CompletedProcess {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    })
}

fn terminate(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn bounded_reader<R>(reader: R, maximum: usize) -> thread::JoinHandle<io::Result<CapturedBytes>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
        let mut bytes = Vec::new();
        reader.take(limit).read_to_end(&mut bytes)?;
        Ok(CapturedBytes {
            overflow: bytes.len() > maximum,
            bytes,
        })
    })
}

fn join_reader(
    handle: thread::JoinHandle<io::Result<CapturedBytes>>,
    stream: &'static str,
) -> Result<CapturedBytes, ProcessError> {
    handle
        .join()
        .map_err(|_| ProcessError::OutputUnreadable {
            stream,
            detail: "reader thread panicked".to_owned(),
        })?
        .map_err(|source| ProcessError::OutputUnreadable {
            stream,
            detail: source.to_string(),
        })
}

struct CapturedBytes {
    bytes: Vec<u8>,
    overflow: bool,
}

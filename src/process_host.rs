// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded child-process mechanics for binary-only host adapters.
//!
//! This module carries no producer, workflow, evaluation, or assurance
//! semantics. Callers remain responsible for their own command identity,
//! response protocol, and mutation uncertainty.

use std::{ffi::OsStr, path::Path, process::ExitStatus, time::Duration};

#[cfg(unix)]
use std::{
    io::{self, Read},
    process::{Command, Stdio},
    thread,
    time::Instant,
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

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
    run_configured(executable, arguments, None, &[], &[], limits)
}

pub(crate) fn run_configured(
    executable: &OsStr,
    arguments: &[&OsStr],
    current_directory: Option<&Path>,
    environment: &[(&OsStr, &OsStr)],
    removed_environment: &[&OsStr],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    #[cfg(unix)]
    {
        run_configured_contained(
            executable,
            arguments,
            current_directory,
            environment,
            removed_environment,
            limits,
        )
    }
    #[cfg(not(unix))]
    {
        let _ = (
            executable,
            arguments,
            current_directory,
            environment,
            removed_environment,
            limits,
        );
        Err(ProcessError::Unavailable {
            detail: "descendant process-group containment is unavailable on this host".to_owned(),
        })
    }
}

#[cfg(unix)]
fn run_configured_contained(
    executable: &OsStr,
    arguments: &[&OsStr],
    current_directory: Option<&Path>,
    environment: &[(&OsStr, &OsStr)],
    removed_environment: &[&OsStr],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(directory) = current_directory {
        command.current_dir(directory);
    }
    command.envs(environment.iter().copied());
    for name in removed_environment {
        command.env_remove(name);
    }
    command.process_group(0);
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
            Ok(Some(status)) => {
                terminate_process_group(child.id());
                break Ok((status, false));
            }
            Ok(None) if started.elapsed() < limits.timeout => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None) => {
                terminate_process_group(child.id());
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

#[cfg(unix)]
fn terminate(child: &mut std::process::Child) {
    terminate_process_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn terminate_process_group(id: u32) {
    let Ok(raw) = i32::try_from(id) else {
        return;
    };
    let Some(group) = rustix::process::Pid::from_raw(raw) else {
        return;
    };
    let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
struct CapturedBytes {
    bytes: Vec<u8>,
    overflow: bool,
}

#[cfg(all(test, unix))]
mod tests {
    use std::{ffi::OsString, fs, thread, time::Duration};

    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn timeout_terminates_descendants_before_joining_output() {
        let directory = tempfile::tempdir().expect("temporary directory must be available");
        let pid_path = directory.path().join("descendant.pid");
        let completion_path = directory.path().join("descendant.completed");
        let pid_argument = pid_path.as_os_str().to_owned();
        let completion_argument = completion_path.as_os_str().to_owned();
        let arguments = [
            OsStr::new("-c"),
            OsStr::new("(sleep 2; touch \"$2\") & echo $! > \"$1\"; wait"),
            OsStr::new("process-host-test"),
            pid_argument.as_os_str(),
            completion_argument.as_os_str(),
        ];
        let result = run_configured(
            OsStr::new("sh"),
            &arguments,
            None,
            &[],
            &[],
            ProcessLimits {
                timeout: Duration::from_secs(1),
                max_output_bytes: 1024,
            },
        );
        assert!(matches!(result, Err(ProcessError::TimedOut { .. })));

        let raw = fs::read_to_string(pid_path).expect("fixture must publish its descendant pid");
        let pid = raw
            .trim()
            .parse::<i32>()
            .ok()
            .and_then(rustix::process::Pid::from_raw)
            .expect("fixture descendant pid must be valid");
        let deadline = Instant::now() + Duration::from_secs(2);
        while rustix::process::test_kill_process(pid).is_ok() {
            assert!(
                Instant::now() < deadline,
                "descendant remained alive after process-host timeout"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !completion_path.exists(),
            "terminated descendant must not complete after the adapter returns"
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn configured_environment_can_remove_protected_input() {
        let variable = OsString::from("ASSURANCE_PROTECTED_TOKENS");
        let protected = OsString::from("private-marker");
        // The child exits successfully only when the protected variable is absent.
        let arguments = [
            OsStr::new("-c"),
            OsStr::new("test -z \"${ASSURANCE_PROTECTED_TOKENS+x}\""),
        ];
        let result = run_configured(
            OsStr::new("sh"),
            &arguments,
            None,
            &[(variable.as_os_str(), protected.as_os_str())],
            &[variable.as_os_str()],
            ProcessLimits {
                timeout: Duration::from_secs(1),
                max_output_bytes: 1024,
            },
        )
        .expect("environment probe must terminate");
        assert!(result.status.success());
    }
}

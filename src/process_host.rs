// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Compatibility facade for binary-only host adapters.
//!
//! Process creation, supervision, containment, timeout handling, and bounded
//! stream capture are owned by the library's producer-execution kernel.

use std::{ffi::OsStr, path::Path};

pub(crate) use engineering_assurance::producer_execution::__private::{
    CompletedProcess, ProcessError, ProcessLimits,
};

pub(crate) fn run(
    executable: &OsStr,
    arguments: &[&OsStr],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    engineering_assurance::producer_execution::__private::run(executable, arguments, limits)
}

pub(crate) fn run_configured(
    executable: &OsStr,
    arguments: &[&OsStr],
    current_directory: Option<&Path>,
    environment: &[(&OsStr, &OsStr)],
    removed_environment: &[&OsStr],
    limits: ProcessLimits,
) -> Result<CompletedProcess, ProcessError> {
    engineering_assurance::producer_execution::__private::run_configured(
        executable,
        arguments,
        current_directory,
        environment,
        removed_environment,
        limits,
    )
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        ffi::OsString,
        fs, thread,
        time::{Duration, Instant},
    };

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

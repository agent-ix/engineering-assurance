// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The interactive process host (FR-085) under a pseudo-terminal, at the
//! deadline. The test re-runs itself as the session leader of a pseudo-terminal
//! made by `script`, because only a process with a controlling terminal can take
//! and give back its foreground group.

use std::{
    ffi::OsStr,
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use engineering_assurance::producer_execution::__private::{
    ProcessError, ProcessLimits, run_configured_foreground,
};
use ix_trace_rs::trace;
use nix::unistd::{Pid, tcgetpgrp};

const CHILD_MARKER: &str = "EA113_PTY_CHILD";

fn foreground_group() -> Pid {
    let tty = fs::File::open("/dev/tty").expect("a controlling terminal");
    tcgetpgrp(&tty).expect("foreground group")
}

/// The scenario, run inside the pseudo-terminal: a child that never finishes is
/// stopped at the deadline, git's group no longer exists and the foreground
/// group is the one from before the run.
fn deadline_scenario(directory: &std::path::Path) {
    let before = foreground_group();
    let pid_file = directory.join("child.pid");
    let script = format!("echo $$ > '{}'; sleep 30", pid_file.display());
    let started = Instant::now();
    let result = run_configured_foreground(
        OsStr::new("/bin/sh"),
        &[OsStr::new("-c"), OsStr::new(&script)],
        None,
        &[],
        &[],
        ProcessLimits {
            timeout: Duration::from_secs(2),
            max_output_bytes: 1024,
        },
    );
    assert!(
        matches!(result, Err(ProcessError::TimedOut { .. })),
        "the run must reach its deadline"
    );
    assert!(started.elapsed() >= Duration::from_secs(2));
    assert_eq!(foreground_group(), before, "the previous group is restored");
    let child: i32 = fs::read_to_string(&pid_file)
        .expect("child pid")
        .trim()
        .parse()
        .expect("pid");
    let group = rustix::process::Pid::from_raw(child).expect("pid");
    let gone = (0..100).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        rustix::process::test_kill_process_group(group).is_err()
    });
    assert!(gone, "the child's group must not outlive the run");
    fs::write(directory.join("done"), "ok").expect("done marker");
}

#[test]
#[trace("TC-1158", "FR-085-AC-11")]
fn tc_1158_the_foreground_group_is_restored_when_the_run_reaches_its_deadline() {
    if let Some(directory) = std::env::var_os(CHILD_MARKER) {
        deadline_scenario(std::path::Path::new(&directory));
        return;
    }
    let work = tempfile::tempdir().expect("work dir");
    let child = Command::new("/usr/bin/script")
        .args(["-q", "-e", "-c"])
        .arg(format!(
            "'{}' --exact tc_1158_the_foreground_group_is_restored_when_the_run_reaches_its_deadline --nocapture --test-threads=1",
            std::env::current_exe().expect("test binary").display()
        ))
        .arg("/dev/null")
        .env(CHILD_MARKER, work.path())
        .env("SHELL", "/bin/sh")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("script must start");
    let report = String::from_utf8_lossy(&child.stdout).into_owned();
    assert!(child.status.success(), "{report}");
    assert!(work.path().join("done").exists(), "{report}");
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust-owned executable fixture for the bounded producer contract tests.

#![forbid(unsafe_code)]

use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    path::Path,
    process::{self, Command},
    thread,
    time::Duration,
};

fn argument(arguments: &mut impl Iterator<Item = OsString>) -> OsString {
    arguments
        .next()
        .unwrap_or_else(|| fail("missing fixture argument"))
}

fn text(value: OsString) -> String {
    value
        .into_string()
        .unwrap_or_else(|_| fail("fixture argument must be UTF-8"))
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(125);
}

fn wait_for(path: &Path) {
    while !path.exists() {
        thread::sleep(Duration::from_millis(1));
    }
}

fn fixture_executable() -> OsString {
    env::var_os("EA_FIXTURE_EXECUTABLE")
        .unwrap_or_else(|| fail("missing retained fixture executable binding"))
}

fn write_stdout(bytes: &[u8]) {
    io::stdout()
        .write_all(bytes)
        .unwrap_or_else(|_| fail("cannot write fixture stdout"));
}

fn copy_stdin(arguments: &mut impl Iterator<Item = OsString>) {
    let output = argument(arguments);
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .unwrap_or_else(|_| fail("cannot read fixture stdin"));
    fs::write(output, &bytes).unwrap_or_else(|_| fail("cannot write fixture output"));
    write_stdout(&bytes);
}

fn observation(path: &Path) -> &'static [u8] {
    if path.exists() { b"present" } else { b"absent" }
}

fn wait_read(arguments: &mut impl Iterator<Item = OsString>) {
    let ready = argument(arguments);
    let release = argument(arguments);
    let input = argument(arguments);
    fs::write(&ready, b"ready").unwrap_or_else(|_| fail("cannot write ready marker"));
    wait_for(Path::new(&release));
    let bytes = fs::read(input).unwrap_or_else(|_| fail("cannot read retained input"));
    write_stdout(&bytes);
}

fn wait_exists(arguments: &mut impl Iterator<Item = OsString>) {
    let ready = argument(arguments);
    let release = argument(arguments);
    let path = argument(arguments);
    fs::write(&ready, b"ready").unwrap_or_else(|_| fail("cannot write ready marker"));
    wait_for(Path::new(&release));
    write_stdout(observation(Path::new(&path)));
}

fn exit_with_output(arguments: &mut impl Iterator<Item = OsString>) -> ! {
    let code = text(argument(arguments))
        .parse::<i32>()
        .unwrap_or_else(|_| fail("invalid exit code"));
    write_stdout(argument(arguments).as_encoded_bytes());
    process::exit(code);
}

fn wait(arguments: &mut impl Iterator<Item = OsString>) {
    let millis = text(argument(arguments))
        .parse::<u64>()
        .unwrap_or_else(|_| fail("invalid wait duration"));
    thread::sleep(Duration::from_millis(millis));
}

fn spawn_child(arguments: &mut impl Iterator<Item = OsString>) {
    let ready = argument(arguments);
    let completed = argument(arguments);
    let mut child = Command::new(fixture_executable())
        .args(["wait", "10000"])
        .spawn()
        .unwrap_or_else(|_| fail("cannot spawn fixture child"));
    fs::write(ready, b"ready").unwrap_or_else(|_| fail("cannot write ready marker"));
    let _ = child.wait();
    fs::write(completed, b"completed").unwrap_or_else(|_| fail("cannot write completion marker"));
}

fn escape(arguments: &mut impl Iterator<Item = OsString>) {
    let pid_path = argument(arguments);
    let mut child = Command::new(fixture_executable())
        .arg("escape-child")
        .arg(pid_path)
        .spawn()
        .unwrap_or_else(|_| fail("cannot spawn escaping fixture"));
    let _ = child.wait();
}

fn escape_child(arguments: &mut impl Iterator<Item = OsString>) {
    rustix::process::setsid().unwrap_or_else(|_| fail("cannot create fixture session"));
    fs::write(argument(arguments), process::id().to_string())
        .unwrap_or_else(|_| fail("cannot write escaped pid"));
    thread::sleep(Duration::from_secs(10));
}

fn main() {
    let mut arguments = env::args_os().skip(1);
    match text(argument(&mut arguments)).as_str() {
        "emit" => write_stdout(argument(&mut arguments).as_encoded_bytes()),
        "copy-stdin" => copy_stdin(&mut arguments),
        "environment" => println!(
            "ONLY={}",
            env::var("ONLY").unwrap_or_else(|_| fail("missing ONLY environment"))
        ),
        "touch" => fs::write(argument(&mut arguments), b"launched")
            .unwrap_or_else(|_| fail("cannot write launch marker")),
        "wait-read" => wait_read(&mut arguments),
        "exists" => write_stdout(observation(Path::new(&argument(&mut arguments)))),
        "wait-exists" => wait_exists(&mut arguments),
        "exit" => exit_with_output(&mut arguments),
        "wait" => wait(&mut arguments),
        "spawn-child" => spawn_child(&mut arguments),
        "escape" => escape(&mut arguments),
        "escape-child" => escape_child(&mut arguments),
        _ => fail("unknown fixture mode"),
    }
}

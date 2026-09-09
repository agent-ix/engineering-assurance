// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Engineering Assurance command-line boundary.

#![forbid(unsafe_code)]

use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

use clap::Command;
use engineering_assurance::compatibility::{CompatibilityOutcome, evaluate_request_bytes};
use serde::Serialize;

const ERROR_PROTOCOL: &str = "engineering-assurance.error/v1";
const COMPATIBILITY_CAPABILITY: &str = "compatibility";

#[derive(Serialize)]
struct MachineError<'a> {
    protocol: &'static str,
    capability: &'static str,
    code: &'a str,
    message: &'a str,
}

fn command() -> Command {
    Command::new(engineering_assurance::PACKAGE_NAME)
        .version(engineering_assurance::PACKAGE_VERSION)
        .about("Engineering Assurance native command boundary")
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .subcommand(
            Command::new(COMPATIBILITY_CAPABILITY)
                .about("Classify explicit observations against the reviewed compatibility matrix"),
        )
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    match matches.subcommand_name() {
        Some(COMPATIBILITY_CAPABILITY) => run_compatibility(),
        Some(_) | None => ExitCode::from(2),
    }
}

fn run_compatibility() -> ExitCode {
    let mut bytes = Vec::new();
    let read_result = io::stdin().lock().read_to_end(&mut bytes);
    if let Err(error) = read_result {
        return emit_error(
            "compatibility_input_unreadable",
            &format!("failed to read compatibility request: {error}"),
        );
    }
    match evaluate_request_bytes(&bytes) {
        Ok(result) => match result.to_json_line() {
            Ok(encoded) => {
                if let Err(error) = write_stdout(&encoded) {
                    eprintln!("failed to write compatibility result: {error}");
                    return ExitCode::from(2);
                }
                match result.outcome {
                    CompatibilityOutcome::Compatible => ExitCode::SUCCESS,
                    CompatibilityOutcome::Withheld => ExitCode::from(1),
                }
            }
            Err(error) => emit_error(error.code(), &error.to_string()),
        },
        Err(error) => emit_error(error.code(), &error.to_string()),
    }
}

fn emit_error(code: &str, message: &str) -> ExitCode {
    let result = MachineError {
        protocol: ERROR_PROTOCOL,
        capability: COMPATIBILITY_CAPABILITY,
        code,
        message,
    };
    let encoded = match serde_json::to_vec(&result) {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            bytes
        }
        Err(_) => b"{\"protocol\":\"engineering-assurance.error/v1\",\"capability\":\"compatibility\",\"code\":\"error_result_serialization_failed\",\"message\":\"error result serialization failed\"}\n".to_vec(),
    };
    if let Err(error) = write_stdout(&encoded) {
        eprintln!("{message}; failed to write error result: {error}");
    } else {
        eprintln!("{message}");
    }
    ExitCode::from(2)
}

fn write_stdout(bytes: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()
}

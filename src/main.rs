// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Engineering Assurance command-line boundary.

#![forbid(unsafe_code)]

use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

use clap::Command;
use engineering_assurance::compatibility::{
    CompatibilityOutcome, MAX_REQUEST_BYTES, evaluate_request_bytes,
};
use serde::Serialize;

const ERROR_PROTOCOL: &str = "engineering-assurance.error/v1";
const COMPATIBILITY_CAPABILITY: &str = "compatibility";
const MAX_DIAGNOSTIC_BYTES: usize = 4_096;

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
    let read_limit = u64::try_from(MAX_REQUEST_BYTES).map_or(u64::MAX, |limit| limit + 1);
    let read_result = io::stdin().lock().take(read_limit).read_to_end(&mut bytes);
    if let Err(error) = read_result {
        return emit_error(
            "compatibility_input_unreadable",
            &format!("failed to read compatibility request: {error}"),
        );
    }
    if bytes.len() > MAX_REQUEST_BYTES {
        let message = format!(
            "compatibility request is at least {} bytes; maximum is {MAX_REQUEST_BYTES}",
            bytes.len()
        );
        return emit_error("compatibility_request_too_large", &message);
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
    let message = bounded_message(message);
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

fn bounded_message(message: &str) -> &str {
    if message.len() <= MAX_DIAGNOSTIC_BYTES {
        return message;
    }
    let mut end = MAX_DIAGNOSTIC_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    &message[..end]
}

fn write_stdout(bytes: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Engineering Assurance command-line boundary.

#![forbid(unsafe_code)]

mod onboarding_host;

use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

use clap::Command;
use engineering_assurance::compatibility::{CompatibilityOutcome, evaluate_request_bytes};
use engineering_assurance::onboarding;
use engineering_assurance::workflow_invariants;
use serde::Serialize;

const ERROR_PROTOCOL: &str = "engineering-assurance.error/v1";
const COMPATIBILITY_CAPABILITY: &str = "compatibility";
const ONBOARDING_CAPABILITY: &str = "onboarding";
const WORKFLOW_INVARIANTS_CAPABILITY: &str = "workflow-invariants";
const MAX_STDIN_BYTES: usize = 8 * 1024 * 1024;

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
        .subcommand(
            Command::new(ONBOARDING_CAPABILITY)
                .about("Inventory a selected repository and produce a bounded onboarding result"),
        )
        .subcommand(
            Command::new(WORKFLOW_INVARIANTS_CAPABILITY)
                .about("Evaluate a closed workflow projection against named invariants"),
        )
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    match matches.subcommand_name() {
        Some(COMPATIBILITY_CAPABILITY) => run_compatibility(),
        Some(ONBOARDING_CAPABILITY) => run_onboarding(),
        Some(WORKFLOW_INVARIANTS_CAPABILITY) => run_workflow_invariants(),
        Some(_) | None => ExitCode::from(2),
    }
}

fn run_onboarding() -> ExitCode {
    let bytes = match read_stdin(ONBOARDING_CAPABILITY, "onboarding_request_invalid") {
        Ok(bytes) => bytes,
        Err(exit_code) => return exit_code,
    };
    let request = match onboarding::parse_request_bytes(&bytes) {
        Ok(request) => request,
        Err(error) => {
            return emit_error(ONBOARDING_CAPABILITY, error.code(), &error.to_string());
        }
    };
    let result = match onboarding_host::execute(&request) {
        Ok(result) => result,
        Err(error) => {
            return emit_error(ONBOARDING_CAPABILITY, error.code(), &error.to_string());
        }
    };
    match result.to_json_line() {
        Ok(encoded) => match write_stdout(&encoded) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("failed to write onboarding result: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => emit_error(ONBOARDING_CAPABILITY, error.code(), &error.to_string()),
    }
}

fn run_compatibility() -> ExitCode {
    let bytes = match read_stdin(COMPATIBILITY_CAPABILITY, "compatibility_input_unreadable") {
        Ok(bytes) => bytes,
        Err(exit_code) => return exit_code,
    };
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
            Err(error) => emit_error(COMPATIBILITY_CAPABILITY, error.code(), &error.to_string()),
        },
        Err(error) => emit_error(COMPATIBILITY_CAPABILITY, error.code(), &error.to_string()),
    }
}

fn run_workflow_invariants() -> ExitCode {
    let bytes = match read_stdin(
        WORKFLOW_INVARIANTS_CAPABILITY,
        "workflow_invariant_request_invalid",
    ) {
        Ok(bytes) => bytes,
        Err(exit_code) => return exit_code,
    };
    match workflow_invariants::evaluate_request_bytes(&bytes) {
        Ok(result) => match result.to_json_line() {
            Ok(encoded) => match write_stdout(&encoded) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("failed to write workflow-invariant result: {error}");
                    ExitCode::from(2)
                }
            },
            Err(error) => emit_error(
                WORKFLOW_INVARIANTS_CAPABILITY,
                error.code(),
                &error.to_string(),
            ),
        },
        Err(error) => emit_error(
            WORKFLOW_INVARIANTS_CAPABILITY,
            error.code(),
            &error.to_string(),
        ),
    }
}

fn read_stdin(capability: &'static str, error_code: &str) -> Result<Vec<u8>, ExitCode> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_STDIN_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if let Err(error) = io::stdin().lock().take(limit).read_to_end(&mut bytes) {
        return Err(emit_error(
            capability,
            error_code,
            &format!("failed to read {capability} request: {error}"),
        ));
    }
    if bytes.len() > MAX_STDIN_BYTES {
        return Err(emit_error(
            capability,
            error_code,
            &format!("{capability} request exceeds the {MAX_STDIN_BYTES}-byte input limit"),
        ));
    }
    Ok(bytes)
}

fn emit_error(capability: &'static str, code: &str, message: &str) -> ExitCode {
    let result = MachineError {
        protocol: ERROR_PROTOCOL,
        capability,
        code,
        message,
    };
    let encoded = match serde_json::to_vec(&result) {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            bytes
        }
        Err(_) => format!(
            "{{\"protocol\":\"{ERROR_PROTOCOL}\",\"capability\":\"{capability}\",\"code\":\"error_result_serialization_failed\",\"message\":\"error result serialization failed\"}}\n"
        )
        .into_bytes(),
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

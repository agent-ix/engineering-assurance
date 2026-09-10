// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Engineering Assurance command-line boundary.

#![forbid(unsafe_code)]

mod onboarding_host;
mod package_host;
mod workflow_host;

use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::{Arg, ArgMatches, Command};
use engineering_assurance::compatibility::{CompatibilityOutcome, evaluate_request_bytes};
use engineering_assurance::onboarding;
use engineering_assurance::package_lifecycle::PackageLifecycleResult;
use engineering_assurance::workflow;
use engineering_assurance::workflow_invariants;
use serde::Serialize;

const ERROR_PROTOCOL: &str = "engineering-assurance.error/v1";
const COMPATIBILITY_CAPABILITY: &str = "compatibility";
const ONBOARDING_CAPABILITY: &str = "onboarding";
const PACKAGE_LIFECYCLE_CAPABILITY: &str = "package-lifecycle";
const WORKFLOW_INVARIANTS_CAPABILITY: &str = "workflow-invariants";
const WORKFLOW_HOST_CAPABILITY: &str = "workflow-host";
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
        .subcommand(
            Command::new(WORKFLOW_HOST_CAPABILITY)
                .about("Coordinate a bound workflow lifecycle through ix-flow"),
        )
        .subcommand(
            Command::new(PACKAGE_LIFECYCLE_CAPABILITY)
                .about("Stage, clean, or refuse publication of the npm artifact bundle")
                .subcommand_required(true)
                .subcommand(package_root_command(
                    "stage",
                    "Stage the fixed npm module payload",
                ))
                .subcommand(package_root_command(
                    "clean",
                    "Remove an exactly corresponding staged npm payload",
                ))
                .subcommand(
                    Command::new("refuse-publication")
                        .about("Unconditionally refuse public npm publication")
                        .arg(npm_hook_arg()),
                ),
        )
}

fn package_root_command(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(
            Arg::new("root")
                .long("root")
                .value_name("PATH")
                .value_parser(clap::value_parser!(PathBuf))
                .required(true),
        )
        .arg(npm_hook_arg())
}

fn npm_hook_arg() -> Arg {
    Arg::new("npm-hook")
        .long("npm-hook")
        .action(clap::ArgAction::SetTrue)
        .help("Leave stdout to the invoking npm lifecycle command")
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    match matches.subcommand() {
        Some((COMPATIBILITY_CAPABILITY, _)) => run_compatibility(),
        Some((ONBOARDING_CAPABILITY, _)) => run_onboarding(),
        Some((WORKFLOW_INVARIANTS_CAPABILITY, _)) => run_workflow_invariants(),
        Some((WORKFLOW_HOST_CAPABILITY, _)) => run_workflow_host(),
        Some((PACKAGE_LIFECYCLE_CAPABILITY, arguments)) => run_package_lifecycle(arguments),
        Some(_) | None => ExitCode::from(2),
    }
}

fn run_package_lifecycle(arguments: &ArgMatches) -> ExitCode {
    let (result, npm_hook) = match arguments.subcommand() {
        Some(("stage", values)) => (
            package_root(values).and_then(package_host::stage),
            values.get_flag("npm-hook"),
        ),
        Some(("clean", values)) => (
            package_root(values).and_then(package_host::clean),
            values.get_flag("npm-hook"),
        ),
        Some(("refuse-publication", values)) => (
            Ok(PackageLifecycleResult::publication_refused()),
            values.get_flag("npm-hook"),
        ),
        Some(_) | None => {
            return emit_error(
                PACKAGE_LIFECYCLE_CAPABILITY,
                "package_operation_invalid",
                "package-lifecycle operation is invalid",
            );
        }
    };
    let result = match result {
        Ok(result) => result,
        Err(error) if npm_hook => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
        Err(error) => {
            return emit_error(
                PACKAGE_LIFECYCLE_CAPABILITY,
                error.code(),
                &error.to_string(),
            );
        }
    };
    let refused = result.operation()
        == engineering_assurance::package_lifecycle::PackageLifecycleOperation::RefusePublication;
    if npm_hook {
        if refused {
            eprintln!("public package publication is disabled for engineering-assurance");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }
    match result.to_json_line() {
        Ok(encoded) => match write_stdout(&encoded) {
            Ok(()) if refused => {
                eprintln!("public package publication is disabled for engineering-assurance");
                ExitCode::from(1)
            }
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("failed to write package-lifecycle result: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => emit_error(
            PACKAGE_LIFECYCLE_CAPABILITY,
            error.code(),
            &error.to_string(),
        ),
    }
}

fn package_root(
    arguments: &ArgMatches,
) -> Result<&std::path::Path, package_host::PackageHostError> {
    arguments
        .get_one::<PathBuf>("root")
        .map(PathBuf::as_path)
        .ok_or(package_host::PackageHostError::RootInvalid)
}

fn run_workflow_host() -> ExitCode {
    let bytes = match read_stdin(WORKFLOW_HOST_CAPABILITY, "workflow_host_request_invalid") {
        Ok(bytes) => bytes,
        Err(exit_code) => return exit_code,
    };
    let request = match workflow::parse_request_bytes(&bytes) {
        Ok(request) => request,
        Err(error) => {
            return emit_error(WORKFLOW_HOST_CAPABILITY, error.code(), &error.to_string());
        }
    };
    let result = match workflow_host::execute(&request) {
        Ok(result) => result,
        Err(error) => {
            return emit_error(WORKFLOW_HOST_CAPABILITY, error.code(), &error.to_string());
        }
    };
    match result.to_json_line() {
        Ok(encoded) => match write_stdout(&encoded) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("failed to write workflow-host result: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => emit_error(WORKFLOW_HOST_CAPABILITY, error.code(), &error.to_string()),
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

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Engineering Assurance command-line boundary.

#![forbid(unsafe_code)]

use clap::Command;

fn command() -> Command {
    Command::new(engineering_assurance::PACKAGE_NAME)
        .version(engineering_assurance::PACKAGE_VERSION)
        .about("Engineering Assurance native command boundary")
        .disable_help_subcommand(true)
}

fn main() {
    command().get_matches();
}

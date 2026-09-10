// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Engineering Assurance domain types and deterministic behavior.
//!
//! This crate is the reusable, I/O-free boundary governed by ADR-002 and
//! FR-014. Filesystem, process, environment, and host interaction belong to the
//! `engineering-assurance` binary rather than this library.

#![forbid(unsafe_code)]

pub mod compatibility;
pub mod evaluation;
pub mod evidence;
pub mod onboarding;
pub mod semantics;
pub mod workflow;
pub mod workflow_invariants;

/// The Cargo package name shared by the library and CLI targets.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

/// The package version embedded in the library at build time.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

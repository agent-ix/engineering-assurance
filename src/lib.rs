// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Engineering Assurance domain types and deterministic behavior.
//!
//! This crate is the reusable boundary governed by ADR-002 and FR-014. Domain
//! modules are I/O-free. FR-019 admits process, filesystem, and environment
//! access only inside the bounded [`producer_execution`] capability.

#![forbid(unsafe_code)]

pub mod compatibility;
pub mod compatibility_corpus;
pub mod content_rights;
pub mod discovery;
pub mod evaluation;
pub mod evaluation_reports;
pub mod evidence;
pub mod manifest;
pub mod onboarding;
pub mod package_audit;
pub mod package_lifecycle;
pub mod package_membership;
pub mod producer_execution;
pub mod semantics;
pub mod source_audit;
pub mod structured_yaml;
pub mod workflow;
pub mod workflow_invariants;

/// The Cargo package name shared by the library and CLI targets.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

/// The package version embedded in the library at build time.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

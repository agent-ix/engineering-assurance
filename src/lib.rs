// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Engineering Assurance domain types and deterministic behavior.
//!
//! This crate is the reusable boundary governed by ADR-002 and FR-014. Domain
//! modules are I/O-free. FR-019 admits process, filesystem, and environment
//! access only inside the bounded [`producer_execution`] capability.

#![forbid(unsafe_code)]

#[cfg(feature = "full")]
pub mod compatibility;
#[cfg(feature = "full")]
pub mod compatibility_corpus;
#[cfg(feature = "full")]
pub mod content_rights;
#[cfg(feature = "full")]
pub mod discovery;
pub mod evaluation;
#[cfg(feature = "full")]
pub mod evaluation_reports;
#[cfg(feature = "full")]
pub mod evidence;
#[cfg(feature = "full")]
pub mod manifest;
#[cfg(feature = "full")]
pub mod onboarding;
#[cfg(feature = "full")]
pub mod package_audit;
#[cfg(feature = "full")]
pub mod package_lifecycle;
#[cfg(feature = "full")]
pub mod package_membership;
#[cfg(feature = "producer-execution")]
pub mod producer_execution;
#[cfg(feature = "full")]
pub mod semantics;
#[cfg(feature = "full")]
pub mod source_audit;
#[cfg(feature = "full")]
pub mod structured_yaml;
#[cfg(feature = "full")]
pub mod workflow;
#[cfg(feature = "full")]
pub mod workflow_invariants;

/// The Cargo package name shared by the library and CLI targets.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

/// The package version embedded in the library at build time.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

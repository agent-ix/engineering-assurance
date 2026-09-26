// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Engineering Assurance domain types and deterministic behavior.
//!
//! This crate is the reusable boundary governed by ADR-002 and FR-014. Domain
//! modules are I/O-free. FR-019 admits process, filesystem, and environment
//! access only inside the bounded [`producer_execution`] capability.
//!
//! # Cargo features
//!
//! Every library module has its own capability feature. Enable only what a
//! consumer needs; each feature compiles alone (`make rust-features`).
//! `full` (the current `default`) enables every row below plus the
//! `engineering-assurance` binary's own crates (`clap`, `cap-std`, `tar`,
//! `zip`, `flate2`, `tempfile`) and `serde_json/arbitrary_precision`, which
//! no per-capability feature enables.
//!
//! | Feature | Module(s) | Other features it enables | Extra dependencies |
//! |---|---|---|---|
//! | `claim-strength` | `claim_strength` | | `serde` |
//! | `compatibility` | `compatibility` | | `serde`, `serde_json`, `thiserror` |
//! | `compatibility-corpus` | `compatibility_corpus` | | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `content-rights` | `content_rights` | | `regex`, `serde`, `serde_json`, `thiserror`, `unicode-casefold` |
//! | `discovery` | `discovery` | `workflow` | `serde`, `serde_json`, `thiserror` |
//! | `evaluation` | `evaluation` | `evidence`, `workflow` | `serde`, `thiserror`, `time` |
//! | `evaluation-reports` | `evaluation_reports` | `evaluation` | `serde`, `serde_json`, `thiserror` |
//! | `evidence` | `evidence` | | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `manifest` | `manifest` | `structured-yaml` | `jsonschema`, `serde`, `serde_json`, `thiserror` |
//! | `measurement` | `measurement` | | `serde`, `thiserror` |
//! | `onboarding` | `onboarding` | | `serde`, `serde_json`, `thiserror`, `yaml_serde` |
//! | `package-audit` | `package_audit` | | `serde`, `serde_json`, `thiserror` |
//! | `package-lifecycle` | `package_lifecycle` | | `serde`, `serde_json`, `thiserror` |
//! | `package-membership` | `package_membership` | | `serde`, `thiserror` |
//! | `producer-execution` | `producer_execution` | | `rustix`, `serde`, `serde_json`, `serde_json_canonicalizer`, `sha2`, `thiserror` |
//! | `semantics` | `semantics` | `claim-strength`, `compatibility-corpus` | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `source-audit` | `source_audit` | | `serde`, `syn`, `thiserror` |
//! | `structured-yaml` | `structured_yaml` | | `serde_json`, `yaml_serde` |
//! | `workflow` | `workflow` | | `serde`, `serde_json`, `thiserror` |
//! | `workflow-invariants` | `workflow_invariants` | | `serde`, `serde_json`, `thiserror`, `time` |
//! | `campaign` | `campaign` | `measurement`, `producer-execution` | the generated campaign crate, `sha1` |
//! | `full` | all of the above, plus the binary | every feature above | `clap`, `cap-std`, `tar`, `zip`, `flate2`, `tempfile`, `serde_json/arbitrary_precision` |

#![forbid(unsafe_code)]

#[cfg(feature = "campaign")]
pub mod campaign;
#[cfg(feature = "claim-strength")]
pub mod claim_strength;
#[cfg(feature = "compatibility")]
pub mod compatibility;
#[cfg(feature = "compatibility-corpus")]
pub mod compatibility_corpus;
#[cfg(feature = "content-rights")]
pub mod content_rights;
#[cfg(feature = "discovery")]
pub mod discovery;
#[cfg(feature = "evaluation")]
pub mod evaluation;
#[cfg(feature = "evaluation-reports")]
pub mod evaluation_reports;
#[cfg(feature = "evidence")]
pub mod evidence;
#[cfg(feature = "manifest")]
pub mod manifest;
#[cfg(feature = "measurement")]
pub mod measurement;
#[cfg(feature = "onboarding")]
pub mod onboarding;
#[cfg(feature = "package-audit")]
pub mod package_audit;
#[cfg(feature = "package-lifecycle")]
pub mod package_lifecycle;
#[cfg(feature = "package-membership")]
pub mod package_membership;
#[cfg(feature = "producer-execution")]
pub mod producer_execution;
#[cfg(feature = "semantics")]
pub mod semantics;
#[cfg(feature = "source-audit")]
pub mod source_audit;
#[cfg(feature = "structured-yaml")]
pub mod structured_yaml;
#[cfg(feature = "workflow")]
pub mod workflow;
#[cfg(feature = "workflow-invariants")]
pub mod workflow_invariants;

/// The Cargo package name shared by the library and CLI targets.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

/// The package version embedded in the library at build time.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

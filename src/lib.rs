// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Engineering Assurance domain types and deterministic behavior.
//!
//! This crate is the reusable boundary governed by ADR-002 and FR-014. Domain
//! modules are I/O-free. FR-019 admits process, filesystem, and environment
//! access only inside the bounded `producer_execution` capability.
//!
//! # Cargo features
//!
//! **Default features are empty.** Enable the capability you need, for example
//! `features = ["source-audit"]`; the `engineering-assurance` binary needs
//! `full`. A bare dependency on this crate therefore exposes no module and
//! pulls no optional dependency. That is deliberate: Cargo unifies features
//! across the whole build, so a default that reached
//! `serde_json/arbitrary_precision` would silently turn it on for every crate
//! in a consumer's workspace.
//!
//! Every library module has its own capability feature. Enable only what a
//! consumer needs; each feature compiles alone (`make rust-features`). `full`
//! enables every row below plus the `engineering-assurance` binary's own crates
//! (`clap`, `cap-std`, `tar`, `zip`, `flate2`, `tempfile`) and `exact-numbers`.
//!
//! `exact-numbers` turns on `serde_json/arbitrary_precision`. It is workspace-
//! global, and it changes how a `serde_json::Value` bridges to non-JSON
//! serializers: yaml, toml and ciborium re-serialise a number as a private
//! marker map instead of a number. It is the explicit opt-in, and only the
//! features marked "implies `exact-numbers`" below enable it: their identity
//! digests and refusals (integers beyond `u64`, `1e309`) change without it.
//!
//! | Feature | Module(s) | Other features it enables | Extra dependencies |
//! |---|---|---|---|
//! | `claim-strength` | `claim_strength` | | `serde` |
//! | `compatibility` | `compatibility` | | `serde`, `serde_json`, `thiserror` |
//! | `compatibility-corpus` | `compatibility_corpus` | | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `content-rights` | `content_rights` | | `regex`, `serde`, `serde_json`, `thiserror`, `unicode-casefold` |
//! | `discovery` | `discovery` | `workflow` | `serde`, `serde_json`, `thiserror` |
//! | `evaluation` | `evaluation` | `evidence`, `workflow` (implies `exact-numbers`) | `serde`, `serde_json`, `thiserror`, `time` |
//! | `evaluation-reports` | `evaluation_reports` | `evaluation`, `evidence`, `workflow` (implies `exact-numbers`) | `serde`, `serde_json`, `thiserror` |
//! | `evidence` | `evidence` | `exact-numbers` | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `manifest` | `manifest` | `structured-yaml` | `jsonschema`, `serde`, `serde_json`, `thiserror` |
//! | `measurement` | `measurement` | | `serde`, `thiserror` |
//! | `onboarding` | `onboarding` | | `serde`, `serde_json`, `thiserror`, `yaml_serde` |
//! | `package-audit` | `package_audit` | | `serde`, `serde_json`, `thiserror` |
//! | `package-lifecycle` | `package_lifecycle` | | `serde`, `serde_json`, `thiserror` |
//! | `package-membership` | `package_membership` | | `serde`, `thiserror` |
//! | `producer-execution` | `producer_execution` | | `rustix`, `serde`, `serde_json`, `serde_json_canonicalizer`, `sha2`, `thiserror` |
//! | `semantics` | `semantics` | `claim-strength`, `compatibility-corpus`, `exact-numbers` | `serde`, `serde_json`, `sha2`, `thiserror` |
//! | `source-audit` | `source_audit` | | `serde`, `syn`, `thiserror` |
//! | `structured-yaml` | `structured_yaml` | | `serde_json`, `yaml_serde` |
//! | `workflow` | `workflow` | | `serde`, `serde_json`, `thiserror` |
//! | `workflow-invariants` | `workflow_invariants` | | `serde`, `serde_json`, `thiserror`, `time` |
//! | `campaign` | `campaign` | `measurement`, `producer-execution` | the generated campaign crate, `sha1` |
//! | `exact-numbers` | none (opt-in) | | `serde_json/arbitrary_precision` |
//! | `full` | all of the above, plus the binary | every feature above | `clap`, `cap-std`, `tar`, `zip`, `flate2`, `tempfile` |

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

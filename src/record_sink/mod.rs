// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pluggable record sinks with a fail-closed public-log check (FR-054 to
//! FR-057, NFR-009, NFR-010).
//!
//! A record exists as a public form of closed value grammars and a private
//! form that holds everything a public log must never see ([`form`]). One sink
//! interface has three implementations: the public log, the private store and
//! `none` ([`sink`]). The repository's configuration selects exactly one
//! ([`config`]). Nothing reaches or signs for the public log unless the leak
//! check of [`leak`] passed for the record being published.
//!
//! The module performs no I/O and implements no cryptography. The public log,
//! the private store, the visibility lookup and the signing stack are traits
//! the caller supplies; the signing stack stays the delegated Sigstore library
//! (NFR-010).

pub mod config;
pub mod form;
pub mod leak;
pub mod sink;

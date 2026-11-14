// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Change record predicates and producer-reported CycloneDX agent BOMs
//! (FR-072, FR-073).

pub mod bom;
pub mod predicate;

pub use bom::{AgentSession, BomError, CycloneDxBom, McpServer, Plugin, build_bom};
pub use predicate::{
    ApprovalDecision, ApprovalEntry, ChangePredicate, Failure, PredicateError, PredicateInput,
    Review, ReviewKind, ReviewVerdict, Status, Ticket, TicketKind, build_predicate,
    canonical_digest, verify_private_form_digest,
};

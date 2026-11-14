// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Change record predicates and producer-reported `CycloneDX` agent BOMs
//! (FR-072, FR-073).

pub mod bom;
pub mod predicate;

pub use bom::{
    build_bom, validate_bom_value, AgentSession, BomError, CycloneDxBom, McpServer, Plugin,
};
pub use predicate::{
    build_predicate, canonical_digest, verify_private_form_digest, ApprovalDecision, ApprovalEntry,
    ApprovalVerdict, ChangePredicate, Failure, PredicateError, PredicateInput, Review, ReviewKind,
    ReviewVerdict, Status, Ticket, TicketKind, PREDICATE_TYPE,
};

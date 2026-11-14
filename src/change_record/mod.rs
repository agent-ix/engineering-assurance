// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Change record predicates and producer-reported `CycloneDX` agent BOMs
//! (FR-072, FR-073).

pub mod bom;
pub mod predicate;

pub use bom::{
    AgentSession, BomError, CycloneDxBom, McpServer, Plugin, build_bom, validate_bom_value,
};
pub use predicate::{
    ApprovalDecision, ApprovalEntry, ApprovalVerdict, BuiltChangeRecord, ChangePredicate,
    ChangeStatement, ChangeSubjectInput, Failure, PREDICATE_TYPE, PredicateError, PredicateInput,
    Review, ReviewInput, ReviewKind, ReviewVerdict, STATEMENT_TYPE, StatementSubject, Status,
    Ticket, TicketKind, build_change_record, canonical_digest, verify_private_form_digest,
};

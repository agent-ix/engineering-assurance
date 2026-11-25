// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Identity of the executable producing a command result.

use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Build identity and host-clock observation for one command invocation.
#[derive(Clone, Serialize)]
pub(crate) struct ProducerIdentity {
    ea_version: &'static str,
    ea_source_revision: Option<&'static str>,
    generated_at: String,
    generated_at_source: &'static str,
}

impl ProducerIdentity {
    /// Samples the producer build identity and UTC host clock once.
    pub(crate) fn sample() -> Option<Self> {
        let now = OffsetDateTime::now_utc();
        let generated_at = OffsetDateTime::from_unix_timestamp(now.unix_timestamp())
            .ok()?
            .format(&Rfc3339)
            .ok()?;
        Some(Self {
            ea_version: env!("CARGO_PKG_VERSION"),
            ea_source_revision: option_env!("EA_SOURCE_REVISION").filter(|value| !value.is_empty()),
            generated_at,
            generated_at_source: "host_clock",
        })
    }
}

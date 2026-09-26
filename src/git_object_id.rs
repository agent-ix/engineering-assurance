// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git blob object ids for the campaign source projection.
//!
//! This is not a content identity. A Git object id must equal what `git`
//! reports for the blob, in Git's SHA-1 or SHA-256 object format, whatever
//! algorithm [`crate::content_digest::ContentDigest`] uses; that is why this
//! module, and only this module besides `content_digest`, names a hash crate
//! (see `tests/digest_identity_audit.rs`).

use std::fmt::Write as _;

use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest, Sha256};

/// Incremental Git blob object id in either object format.
pub(crate) struct GitBlobHasher {
    sha1: Sha1,
    sha256: Sha256,
    hex_len: usize,
}

impl GitBlobHasher {
    /// Starts a blob of `length` bytes whose expected id has `hex_len`
    /// characters (40 selects SHA-1, anything else SHA-256).
    pub(crate) fn new(length: u64, hex_len: usize) -> Self {
        let header = format!("blob {length}\0");
        let mut sha1 = Sha1::new();
        let mut sha256 = Sha256::new();
        sha1.update(header.as_bytes());
        sha256.update(header.as_bytes());
        Self {
            sha1,
            sha256,
            hex_len,
        }
    }

    pub(crate) fn update(&mut self, bytes: &[u8]) {
        self.sha1.update(bytes);
        self.sha256.update(bytes);
    }

    /// Lowercase hexadecimal object id.
    pub(crate) fn finalize(self) -> String {
        let bytes = if self.hex_len == 40 {
            self.sha1.finalize().to_vec()
        } else {
            self.sha256.finalize().to_vec()
        };
        let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
        for byte in bytes {
            let _ = write!(encoded, "{byte:02x}");
        }
        encoded
    }
}

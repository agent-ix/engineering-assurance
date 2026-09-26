// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The single content-digest identity of evidence and artifact bytes.
//!
//! Every retained-byte identity this crate computes goes through
//! [`ContentDigest`]. The hash algorithm is a property of the value
//! ([`ContentDigest::algorithm`]) and is chosen in exactly one place
//! ([`DigestAlgorithm::CURRENT`]), so replacing SHA-256 is a change to this
//! module and to the golden tests that pin its output.
//!
//! The stored and serialized form is the bare lowercase hexadecimal digest,
//! byte-for-byte what earlier releases emitted. The algorithm-prefixed form
//! (`sha256:<hex>`) is additive: see [`ContentDigest::prefixed`] and
//! [`ContentDigest::parse_prefixed`]; it never appears in an existing wire
//! form.

use std::{fmt::Write as _, io::Read};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Closed set of digest algorithms.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DigestAlgorithm {
    /// SHA-256, rendered as 64 lowercase hexadecimal characters.
    Sha256,
}

impl DigestAlgorithm {
    /// The algorithm every newly computed identity uses.
    pub const CURRENT: Self = Self::Sha256;

    /// Stable lowercase name used as the prefix in the prefixed rendering.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }

    /// Number of lowercase hexadecimal characters in a digest.
    #[must_use]
    pub const fn hex_len(self) -> usize {
        match self {
            Self::Sha256 => 64,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "sha256" => Some(Self::Sha256),
            _ => None,
        }
    }
}

/// Failure while parsing or computing a content digest.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum DigestError {
    /// The supplied text is not a lowercase hexadecimal digest of the expected length.
    #[error("digest is not lowercase SHA-256 hexadecimal")]
    Invalid,
    /// The prefixed form names an algorithm this crate does not support.
    #[error("digest names an unsupported algorithm")]
    UnknownAlgorithm,
    /// The selected file does not exist or cannot be resolved.
    #[error("selected file is unavailable")]
    Unavailable,
    /// The selected path is linked or is not a regular file.
    #[error("selected path is not a regular file")]
    NotRegular,
    /// The input exceeds the supported identity ceiling.
    #[error("input exceeds the identity ceiling")]
    TooLarge,
    /// The input could not be read completely.
    #[error("input is unreadable")]
    Unreadable,
    /// The caller cancelled the read.
    #[error("digest computation was cancelled")]
    Cancelled,
}

/// A validated content identity: an algorithm and its lowercase hexadecimal digest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentDigest {
    algorithm: DigestAlgorithm,
    hex: Box<str>,
}

impl ContentDigest {
    /// Parses a bare lowercase hexadecimal digest of the current algorithm.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError::Invalid`] unless `value` is exactly the current
    /// algorithm's hex length of lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, DigestError> {
        Self::parse_hex(DigestAlgorithm::CURRENT, value)
    }

    /// Parses the prefixed rendering `<algorithm>:<lowercase hex>`.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError::UnknownAlgorithm`] for an unsupported algorithm
    /// name and [`DigestError::Invalid`] for a missing prefix or a malformed
    /// digest.
    pub fn parse_prefixed(value: &str) -> Result<Self, DigestError> {
        let (name, hex) = value.split_once(':').ok_or(DigestError::Invalid)?;
        let algorithm = DigestAlgorithm::from_name(name).ok_or(DigestError::UnknownAlgorithm)?;
        Self::parse_hex(algorithm, hex)
    }

    fn parse_hex(algorithm: DigestAlgorithm, value: &str) -> Result<Self, DigestError> {
        if value.len() != algorithm.hex_len()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DigestError::Invalid);
        }
        Ok(Self {
            algorithm,
            hex: value.into(),
        })
    }

    /// Starts an incremental computation with the current algorithm.
    #[must_use]
    pub fn hasher() -> DigestHasher {
        DigestHasher::new(DigestAlgorithm::CURRENT)
    }

    /// Computes the identity of an in-memory byte sequence.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Self::hasher();
        hasher.update(bytes);
        hasher.finalize()
    }

    /// Computes the identity of everything `reader` yields, streaming.
    ///
    /// This module is I/O-free (FR-014-AC-4); the hardened file form,
    /// `ContentDigest::of_file`, is defined beside the filesystem-owning
    /// producer-execution module.
    ///
    /// `cancelled` is polled before every read.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError::Cancelled`], [`DigestError::TooLarge`] when more
    /// than `maximum` bytes are produced, or [`DigestError::Unreadable`].
    pub fn of_reader(
        reader: &mut impl Read,
        maximum: u64,
        cancelled: impl Fn() -> bool,
    ) -> Result<Self, DigestError> {
        hash_reader(reader, maximum, cancelled).map(|(digest, _)| digest)
    }

    /// The algorithm that produced this identity.
    #[must_use]
    pub const fn algorithm(&self) -> DigestAlgorithm {
        self.algorithm
    }

    /// The bare lowercase hexadecimal digest, the form every existing record uses.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.hex
    }

    /// Consumes the value, returning the bare lowercase hexadecimal digest.
    #[must_use]
    pub fn into_hex(self) -> String {
        self.hex.into()
    }

    /// The algorithm-prefixed rendering, for example `sha256:<64 lowercase hex>`.
    #[must_use]
    pub fn prefixed(&self) -> String {
        format!("{}:{}", self.algorithm.name(), self.hex)
    }
}

impl Serialize for ContentDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex)
    }
}

impl<'de> Deserialize<'de> for ContentDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// Incremental [`ContentDigest`] computation.
pub struct DigestHasher {
    state: HasherState,
}

impl std::fmt::Debug for DigestHasher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately opaque: the running state is not an identity.
        formatter
            .debug_struct("DigestHasher")
            .finish_non_exhaustive()
    }
}

enum HasherState {
    Sha256(Sha256),
}

impl DigestHasher {
    /// Starts a computation with `algorithm`.
    #[must_use]
    pub fn new(algorithm: DigestAlgorithm) -> Self {
        Self {
            state: match algorithm {
                DigestAlgorithm::Sha256 => HasherState::Sha256(Sha256::new()),
            },
        }
    }

    /// Feeds more bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        match &mut self.state {
            HasherState::Sha256(hasher) => hasher.update(bytes),
        }
    }

    /// Finishes the computation.
    #[must_use]
    pub fn finalize(self) -> ContentDigest {
        match self.state {
            HasherState::Sha256(hasher) => {
                let mut hex = String::with_capacity(DigestAlgorithm::Sha256.hex_len());
                for byte in hasher.finalize() {
                    let _ = write!(hex, "{byte:02x}");
                }
                ContentDigest {
                    algorithm: DigestAlgorithm::Sha256,
                    hex: hex.into(),
                }
            }
        }
    }
}

pub(crate) fn hash_reader(
    reader: &mut impl Read,
    maximum: u64,
    cancelled: impl Fn() -> bool,
) -> Result<(ContentDigest, u64), DigestError> {
    let mut hasher = ContentDigest::hasher();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancelled() {
            return Err(DigestError::Cancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|_| DigestError::Unreadable)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(read).map_err(|_| DigestError::TooLarge)?)
            .ok_or(DigestError::TooLarge)?;
        if total > maximum {
            return Err(DigestError::TooLarge);
        }
        hasher.update(&buffer[..read]);
    }
    Ok((hasher.finalize(), total))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use ix_trace_rs::trace;

    use super::{ContentDigest, DigestAlgorithm, DigestError};

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    // 65_537 bytes `i % 251`: one byte past 64 KiB, so it spans five 16 KiB reads.
    const PAST_64_KIB: &str = "237356e18b503616912abb8ffaed3a72591e397d4ac294c4637917d48a3f529d";

    fn past_64_kib() -> Vec<u8> {
        (0..65_537_u32)
            .map(|index| u8::try_from(index % 251).expect("remainder fits a byte"))
            .collect()
    }

    /// A reader that yields at most `chunk` bytes per call.
    struct Chunked<'a> {
        bytes: &'a [u8],
        chunk: usize,
    }

    impl std::io::Read for Chunked<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.chunk.min(buffer.len()).min(self.bytes.len());
            buffer[..count].copy_from_slice(&self.bytes[..count]);
            self.bytes = &self.bytes[count..];
            Ok(count)
        }
    }

    #[trace("TC-199", "FR-014-AC-11")]
    #[test]
    fn tc_199_golden_hex_is_pinned_for_bytes_reader_and_hasher() {
        let long = past_64_kib();
        for (input, expected) in [
            (&b""[..], EMPTY),
            (&b"abc"[..], ABC),
            (&long[..], PAST_64_KIB),
        ] {
            assert_eq!(ContentDigest::of_bytes(input).as_str(), expected);
            let streamed = ContentDigest::of_reader(
                &mut Chunked {
                    bytes: input,
                    chunk: 7,
                },
                u64::MAX,
                || false,
            )
            .expect("streaming succeeds");
            assert_eq!(streamed.as_str(), expected);
            let mut hasher = ContentDigest::hasher();
            for piece in input.chunks(1000) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finalize().as_str(), expected);
        }
    }

    #[trace("TC-199", "FR-014-AC-11")]
    #[test]
    fn tc_199_reader_hashing_keeps_its_ceiling_and_cancellation() {
        assert_eq!(
            ContentDigest::of_reader(&mut Cursor::new(b"abcd"), 3, || false),
            Err(DigestError::TooLarge)
        );
        assert_eq!(
            ContentDigest::of_reader(&mut Cursor::new(b"abc"), 3, || false)
                .expect("exactly at the ceiling")
                .as_str(),
            ABC
        );
        assert_eq!(
            ContentDigest::of_reader(&mut Cursor::new(b"abc"), 3, || true),
            Err(DigestError::Cancelled)
        );
    }

    #[trace("TC-199", "FR-014-AC-11")]
    #[test]
    fn tc_199_algorithm_is_carried_and_prefixed_form_is_strict_and_additive() {
        let digest = ContentDigest::of_bytes(b"abc");
        assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
        assert_eq!(DigestAlgorithm::CURRENT.name(), "sha256");
        assert_eq!(digest.as_str(), ABC);
        assert_eq!(digest.prefixed(), format!("sha256:{ABC}"));
        assert_eq!(
            ContentDigest::parse_prefixed(&digest.prefixed()),
            Ok(digest.clone())
        );
        assert_eq!(ContentDigest::parse(ABC), Ok(digest));

        let upper = ABC.to_uppercase();
        for (value, expected) in [
            (format!("blake3:{ABC}"), DigestError::UnknownAlgorithm),
            (format!("SHA256:{ABC}"), DigestError::UnknownAlgorithm),
            (format!(":{ABC}"), DigestError::UnknownAlgorithm),
            (ABC.to_owned(), DigestError::Invalid),
            (format!("sha256:{upper}"), DigestError::Invalid),
            (format!("sha256:{}", &ABC[1..]), DigestError::Invalid),
            (format!("sha256:{ABC}0"), DigestError::Invalid),
            (format!("sha256:{}g", &ABC[1..]), DigestError::Invalid),
            (String::new(), DigestError::Invalid),
        ] {
            assert_eq!(
                ContentDigest::parse_prefixed(&value),
                Err(expected),
                "{value}"
            );
        }
        // The bare parser stays the bare form: a prefixed value is not accepted.
        assert_eq!(
            ContentDigest::parse(&format!("sha256:{ABC}")),
            Err(DigestError::Invalid)
        );
        assert_eq!(ContentDigest::parse(&upper), Err(DigestError::Invalid));
    }
}

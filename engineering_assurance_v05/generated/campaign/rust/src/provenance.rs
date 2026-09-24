//! Provenance constants, each taken verbatim from the compiler request.
//!
//! The correspondence between a constant and the request member it comes
//! from is published in FR-056 rather than inferred here, because the
//! request's member set carries no `SOURCE_DIGEST`, `MANIFEST_DIGEST` or
//! `LOCK_DIGEST` of its own and an implementer would otherwise have to
//! invent the mapping — and the test comparing them would have to invent it
//! a second time.
//!
//! No timestamp, hostname, working directory, user name or absolute path is
//! written here or anywhere else in the crate.

/// The semantic identity of the source the contract was read from.
pub const SOURCE_IDENTITY: &str = "ix://agent-ix/engineering-assurance-campaign/spec";

/// The version of the source the contract was read from.
pub const SOURCE_VERSION: &str = "0.0.0";

/// The digest of the source the contract was read from.
pub const SOURCE_DIGEST: &str =
    "sha256:f763b4ff0ef7f2c888c88bc9b23be1e11b1b5952f035287973c60a10b98fcfd7";

/// The semantic contract package this crate was generated from.
pub const PACKAGE_IDENTITY: &str = "agent-ix/engineering-assurance-campaign";

/// The version of the semantic contract package.
pub const PACKAGE_VERSION: &str = "0.0.0";

/// The digest of the package manifest.
pub const MANIFEST_DIGEST: &str =
    "sha256:b5d75acfbf808c15378bc19aafb6a37c7c6dd06a261acffaa8d0b8294effccd9";

/// The digest of the package lock.
pub const LOCK_DIGEST: &str =
    "sha256:f744926df404e8096f1eb2ddbcf6c6d143cfd721c754aa7d964156f11613850c";

/// The lock fingerprint the compiler request carried.
pub const LOCK_FINGERPRINT: &str =
    "sha256:46795a65600fd62af402b0407db266b7a413dfe6c2246ba3ddd2d13cf92be4ab";

/// The IR contract version the document declared.
pub const CONTRACT_VERSION: &str = "2.0.0";

/// The semantic identity of the backend that generated this crate.
pub const GENERATOR_IDENTITY: &str = "ix://agent-ix/filament-core-data/rust-backend";

/// The version of the backend that generated this crate.
pub const GENERATOR_VERSION: &str = "0.1.0";

/// The Cargo package name derived from the contract package identity.
pub const CRATE_NAME: &str = "agent-ix-engineering-assurance-campaign";

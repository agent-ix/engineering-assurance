// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Qualification of the accepted compatibility corpus through a confined reader.
//!
//! The corpus is retained by `agent-ix/qa-corpus`, pinned here as a submodule
//! and read in place. The reader below opens one explicit corpus root and reads
//! bytes out of it. It writes nothing, executes nothing from the corpus, and
//! refuses every path that would leave the root.
//!
//! The reader lives in the qualification tree rather than the library because
//! the library is the I/O-free boundary FR-014 governs, and rather than the
//! binary because no production command reads the corpus: it is read to
//! qualify the corpus, and nowhere else.

use std::{
    io::Read,
    path::{Component, Path, PathBuf},
};

use std::{collections::BTreeSet, process::Command};

use cap_std::{ambient_authority, fs::Dir};
use engineering_assurance::{
    compatibility_corpus::{
        CorpusError, CorpusIndex, MAX_INDEX_BYTES, REQUIRED_KINDS, RetainedArtifact, Retention,
        SharedConcept, sha256_hex,
    },
    semantics::{Pgm01Outcome, Pgm01View, map_pgm01_bytes},
};
use ix_trace_rs::trace;
use serde_json::Value;
use thiserror::Error;

/// Repository-relative location of the pinned corpus submodule.
const CORPUS_SUBMODULE: &str = "corpus";

/// Submodule-relative location of the compatibility corpus.
const CORPUS_RELATIVE_ROOT: &str = "corpus/compatibility";

/// Corpus-relative name of the accepted index.
const CORPUS_INDEX_NAME: &str = "corpus.json";

/// The largest retained artifact this adapter will read.
const MAX_RETAINED_BYTES: usize = 8_388_608;

/// The most files this adapter will enumerate under one corpus root.
const MAX_CORPUS_ENTRIES: usize = 4_096;

/// The deepest directory nesting this adapter will descend into.
///
/// The accepted corpus groups its records two or three directories below the
/// root, so this leaves an order of magnitude of headroom while keeping the
/// recursive walk bounded by a typed refusal instead of the stack.
const MAX_CORPUS_DEPTH: usize = 32;

/// Stable failures at the confined corpus-filesystem boundary.
#[derive(Debug, Error)]
enum CorpusHostError {
    /// The selected corpus root is absent, a symlink, or not a directory.
    ///
    /// An uninitialized submodule reaches this arm: it is an empty directory
    /// with no index, and a gate that passed quietly over one would prove
    /// nothing.
    #[error(
        "the selected compatibility corpus root is unavailable; run `git submodule update --init {CORPUS_SUBMODULE}`"
    )]
    RootUnavailable,
    /// A retained entry is missing, a symlink, or not a regular file.
    #[error("retained corpus entry {path:?} is missing, linked, or not a regular file")]
    EntryInvalid {
        /// Corpus-relative path of the refused entry.
        path: String,
    },
    /// A retained entry could not be read.
    #[error("retained corpus entry {path:?} is unreadable")]
    EntryUnreadable {
        /// Corpus-relative path of the unreadable entry.
        path: String,
    },
    /// A retained entry exceeds the accepted size bound.
    #[error("retained corpus entry {path:?} exceeds {MAX_RETAINED_BYTES} bytes")]
    EntryTooLarge {
        /// Corpus-relative path of the oversized entry.
        path: String,
    },
    /// The corpus holds more files than this adapter will enumerate.
    #[error("the compatibility corpus holds more than {MAX_CORPUS_ENTRIES} files")]
    PopulationTooLarge,
    /// The corpus nests directories deeper than this adapter will descend.
    ///
    /// The file count alone does not bound a recursive walk: a tree that is
    /// deep rather than wide would exhaust the stack, which is a crash and not
    /// a refusal, so the depth carries a refusal of its own.
    #[error("the compatibility corpus nests deeper than {MAX_CORPUS_DEPTH} directories")]
    TreeTooDeep,
    /// The corpus index or a retained artifact violates its own contract.
    #[error(transparent)]
    Corpus(#[from] CorpusError),
}

impl CorpusHostError {
    /// Stable machine-readable diagnostic code.
    fn code(&self) -> &'static str {
        match self {
            Self::RootUnavailable => "compatibility_corpus_root_unavailable",
            Self::EntryInvalid { .. } => "compatibility_corpus_entry_invalid",
            Self::EntryUnreadable { .. } => "compatibility_corpus_entry_unreadable",
            Self::EntryTooLarge { .. } => "compatibility_corpus_entry_too_large",
            Self::PopulationTooLarge => "compatibility_corpus_population_too_large",
            Self::TreeTooDeep => "compatibility_corpus_tree_too_deep",
            Self::Corpus(error) => error.code(),
        }
    }
}

/// One opened, confined compatibility-corpus root.
#[derive(Debug)]
struct CorpusRoot {
    directory: Dir,
}

impl CorpusRoot {
    /// Open the compatibility corpus under one explicit repository root.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusHostError::RootUnavailable`] when the pinned submodule
    /// is not checked out, when the corpus root is a symlink, or when it is not
    /// a directory.
    fn open(repository_root: &Path) -> Result<Self, CorpusHostError> {
        let path = repository_root.join(CORPUS_RELATIVE_ROOT);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| CorpusHostError::RootUnavailable)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(CorpusHostError::RootUnavailable);
        }
        let canonical =
            std::fs::canonicalize(&path).map_err(|_| CorpusHostError::RootUnavailable)?;
        let directory = Dir::open_ambient_dir(&canonical, ambient_authority())
            .map_err(|_| CorpusHostError::RootUnavailable)?;
        if !directory.exists(CORPUS_INDEX_NAME) {
            return Err(CorpusHostError::RootUnavailable);
        }
        Ok(Self { directory })
    }

    /// Read and validate the accepted corpus index.
    ///
    /// # Errors
    ///
    /// Returns whichever refusal the read itself produced — an unreadable or
    /// oversized index is not the same failure as an absent corpus root — and a
    /// [`CorpusError`] when the index does not describe itself correctly.
    fn load_index(&self) -> Result<CorpusIndex, CorpusHostError> {
        self.load_index_within(MAX_INDEX_BYTES)
    }

    /// Read and validate the accepted corpus index under an explicit bound.
    ///
    /// The bound is a parameter only so a qualification case can drive the
    /// oversized-index path against the real corpus without weakening
    /// [`MAX_INDEX_BYTES`], which is the bound every production read uses.
    ///
    /// # Errors
    ///
    /// Returns the read's own refusal, and a [`CorpusError`] when the index
    /// parses but does not describe itself correctly.
    fn load_index_within(&self, maximum: usize) -> Result<CorpusIndex, CorpusHostError> {
        let bytes = self.read_relative(Path::new(CORPUS_INDEX_NAME), maximum)?;
        Ok(CorpusIndex::parse(&bytes)?)
    }

    /// Read one retained artifact and prove it is the artifact recorded.
    ///
    /// The digest check is the whole contract of this function. A corpus whose
    /// bytes drifted from its index would otherwise still pass every
    /// behavioural assertion below it, against different bytes than the ones
    /// reviewed.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::NotRetained`] for a referenced artifact,
    /// [`CorpusHostError::EntryInvalid`] for a missing, linked, or non-regular
    /// entry, [`CorpusHostError::EntryTooLarge`] beyond the size bound, and
    /// [`CorpusError::DigestMismatch`] when the bytes are not the recorded ones.
    fn retained_bytes(&self, artifact: RetainedArtifact<'_>) -> Result<Vec<u8>, CorpusHostError> {
        let relative = artifact.require_retained_path()?;
        let bytes = self.read_relative(Path::new(relative), MAX_RETAINED_BYTES)?;
        artifact.verify_bytes(&bytes)?;
        Ok(bytes)
    }

    /// Every file the corpus retains, in stable order, under the shipping bounds.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusHostError::PopulationTooLarge`] beyond
    /// [`MAX_CORPUS_ENTRIES`], [`CorpusHostError::TreeTooDeep`] beyond
    /// [`MAX_CORPUS_DEPTH`], and [`CorpusHostError::EntryInvalid`] for an entry
    /// that is neither a regular file nor a directory.
    fn corpus_paths(&self) -> Result<Vec<PathBuf>, CorpusHostError> {
        self.corpus_paths_within(MAX_CORPUS_ENTRIES, MAX_CORPUS_DEPTH)
    }

    /// Every file the corpus retains, under explicit population and depth bounds.
    ///
    /// The two bounds are parameters for the same reason [`MAX_INDEX_BYTES`] is
    /// one on `load_index_within`: a qualification case has to drive the
    /// refusal each bound exists for, and the only other ways to reach them are
    /// to lower the real constants — which weakens the shipping bound to test it
    /// — or to build a corpus of four thousand files, which nothing would be
    /// learned from. With the bounds as parameters a case can put a tree of
    /// known shape exactly at each limit and exactly one past it, which is what
    /// makes a flipped comparison visible; a case that only ever refuses a wildly
    /// oversized tree cannot tell `>` from `>=`.
    ///
    /// Production reads go through [`Self::corpus_paths`], which supplies
    /// [`MAX_CORPUS_ENTRIES`] and [`MAX_CORPUS_DEPTH`] and nothing else.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusHostError::PopulationTooLarge`] beyond `entries`,
    /// [`CorpusHostError::TreeTooDeep`] beyond `depth`, and
    /// [`CorpusHostError::EntryInvalid`] for an entry that is neither a regular
    /// file nor a directory.
    fn corpus_paths_within(
        &self,
        entries: usize,
        depth: usize,
    ) -> Result<Vec<PathBuf>, CorpusHostError> {
        let mut found = Vec::new();
        self.walk(Path::new(""), 0, entries, depth, &mut found)?;
        found.sort();
        Ok(found)
    }

    fn walk(
        &self,
        relative: &Path,
        depth: usize,
        max_entries: usize,
        max_depth: usize,
        found: &mut Vec<PathBuf>,
    ) -> Result<(), CorpusHostError> {
        // The depth is checked before the directory is opened, so a tree deeper
        // than the bound is refused rather than descended one more level. A
        // file-count bound alone would not stop it: the stack runs out first,
        // and a crash is not a refusal an operator can act on.
        if depth > max_depth {
            return Err(CorpusHostError::TreeTooDeep);
        }
        let listing = if relative.as_os_str().is_empty() {
            self.directory.entries()
        } else {
            self.directory.read_dir(relative)
        }
        .map_err(|_| CorpusHostError::EntryInvalid {
            path: relative.display().to_string(),
        })?;
        for entry in listing {
            let entry = entry.map_err(|_| CorpusHostError::EntryInvalid {
                path: relative.display().to_string(),
            })?;
            let child = relative.join(entry.file_name());
            let display = child.display().to_string();
            let kind = self
                .directory
                .symlink_metadata(&child)
                .map_err(|_| CorpusHostError::EntryInvalid {
                    path: display.clone(),
                })?
                .file_type();
            if kind.is_dir() {
                self.walk(&child, depth + 1, max_entries, max_depth, found)?;
            } else if kind.is_file() {
                if found.len() >= max_entries {
                    return Err(CorpusHostError::PopulationTooLarge);
                }
                found.push(child);
            } else {
                // A symlink or special file in a retained evidence tree is not
                // a record; it is a way to read something that is not one.
                return Err(CorpusHostError::EntryInvalid { path: display });
            }
        }
        Ok(())
    }

    /// Read one corpus-relative regular file without leaving the root.
    fn read_relative(&self, relative: &Path, maximum: usize) -> Result<Vec<u8>, CorpusHostError> {
        let display = relative.display().to_string();
        let invalid = || CorpusHostError::EntryInvalid {
            path: display.clone(),
        };
        let components: Vec<_> = relative
            .components()
            .map(|component| match component {
                Component::Normal(value) => Some(value),
                Component::RootDir
                | Component::Prefix(_)
                | Component::CurDir
                | Component::ParentDir => None,
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(invalid)?;
        if components.is_empty() {
            return Err(invalid());
        }

        // Walk the path one component at a time and refuse a symlink anywhere
        // along it. `Dir` already confines resolution to the root; this also
        // refuses a link that stays inside it, because a retained record that
        // is a link is not the record.
        let mut current = PathBuf::new();
        for (index, component) in components.iter().enumerate() {
            current.push(component);
            let metadata = self
                .directory
                .symlink_metadata(&current)
                .map_err(|_| invalid())?;
            if metadata.file_type().is_symlink() {
                return Err(invalid());
            }
            let last = index + 1 == components.len();
            if last && !metadata.is_file() || !last && !metadata.is_dir() {
                return Err(invalid());
            }
        }

        let file = self
            .directory
            .open(relative)
            .map_err(|_| CorpusHostError::EntryUnreadable {
                path: display.clone(),
            })?;
        let limit = u64::try_from(maximum).unwrap_or(u64::MAX);
        let metadata = file
            .metadata()
            .map_err(|_| CorpusHostError::EntryUnreadable {
                path: display.clone(),
            })?;
        if metadata.len() > limit {
            return Err(CorpusHostError::EntryTooLarge {
                path: display.clone(),
            });
        }
        let mut bytes = Vec::new();
        file.take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| CorpusHostError::EntryUnreadable {
                path: display.clone(),
            })?;
        if bytes.len() > maximum {
            return Err(CorpusHostError::EntryTooLarge { path: display });
        }
        Ok(bytes)
    }

    /// Whether one corpus-relative file carries an owner-execute bit.
    ///
    /// These are records, and a record that can be run is a record that will be.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusHostError::EntryInvalid`] when the entry cannot be
    /// inspected.
    fn is_executable(&self, relative: &Path) -> Result<bool, CorpusHostError> {
        let metadata = self.directory.symlink_metadata(relative).map_err(|_| {
            CorpusHostError::EntryInvalid {
                path: relative.display().to_string(),
            }
        })?;
        #[cfg(unix)]
        {
            use cap_std::fs::MetadataExt;
            Ok(metadata.mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            Ok(false)
        }
    }
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn corpus() -> (CorpusRoot, CorpusIndex) {
    let root = CorpusRoot::open(repository_root())
        .expect("the pinned qa-corpus submodule must be checked out");
    let index = root.load_index().expect("the accepted corpus must parse");
    (root, index)
}

fn view(root: &CorpusRoot, index: &CorpusIndex, case_id: &str) -> Pgm01View {
    let case = index.case(case_id).expect("case must exist");
    let raw = root
        .retained_bytes(case.into())
        .expect("retained bytes must verify");
    let expected = case.expected.verify_against_digest_of.as_deref().map(|id| {
        let reference = index.case(id).expect("the referenced case must exist");
        sha256_hex(
            &root
                .retained_bytes(reference.into())
                .expect("the referenced bytes must verify"),
        )
    });
    map_pgm01_bytes(&raw, expected.as_deref()).expect("the mapper must accept a valid digest")
}

fn mapping_values(view: &Pgm01View, source_path: &str) -> Vec<Value> {
    view.mappings
        .iter()
        .filter(|mapping| mapping.source_path == source_path)
        .map(|mapping| mapping.value.clone())
        .collect()
}

#[trace("TC-069", "FR-011-AC-1", "FR-011-CON-2", "FR-015-AC-5")]
#[test]
fn tc_069_every_retained_artifact_is_the_artifact_recorded() {
    let (root, index) = corpus();
    for case in &index.cases {
        root.retained_bytes(case.into())
            .expect("every retained case must reproduce its recorded identity");
    }
    for producer in &index.producer_cases {
        if producer.retention == Retention::Retained {
            root.retained_bytes(producer.into())
                .expect("every retained producer case must reproduce its identity");
        }
    }
    for artifact in &index.chain.artifacts {
        root.retained_bytes(artifact.into())
            .expect("every chain artifact must reproduce its identity");
    }

    // A real legacy case must still match the digest the SOURCE repository
    // recorded for it. That is what proves the retained copy is the
    // immutable record and not an edited lookalike, and it is the assertion
    // that would fail first if anyone rewrote the source evidence tree.
    let mut compared = 0_usize;
    for case in &index.cases {
        if !case.family.starts_with("pgm01") || case.constructed() {
            continue;
        }
        let Some(origin) = case.origin.as_ref() else {
            continue;
        };
        let Some(recorded) = origin.recorded_sha256.as_deref() else {
            continue;
        };
        let raw = root
            .retained_bytes(case.into())
            .expect("retained bytes must verify");
        assert_eq!(
            sha256_hex(&raw),
            recorded,
            "{} no longer matches the digest {} recorded for it",
            case.id,
            origin.repository
        );
        compared += 1;
    }
    assert!(
        compared > 0,
        "no real legacy case was compared to its source"
    );
}

#[trace("TC-070", "FR-011-AC-2", "FR-015-AC-5", "FR-011-CON-3")]
#[test]
fn tc_070_the_accepted_corpus_covers_every_required_state() {
    let (_root, index) = corpus();
    index
        .require_all_kinds()
        .expect("the accepted corpus must carry every required state");
    for kind in REQUIRED_KINDS {
        assert!(
            !index.cases_by_kind(kind).is_empty(),
            "the accepted corpus carries no {kind} case"
        );
    }

    // Every constructed case says what was changed and why, so a reader can
    // tell a real record from one built to fill a hole in the corpus.
    for case in &index.cases {
        let Some(derivation) = case.derivation.as_ref() else {
            continue;
        };
        assert!(
            !derivation.operation.trim().is_empty(),
            "{} names no edit",
            case.id
        );
        assert!(
            !derivation.reason.trim().is_empty(),
            "{} gives no reason",
            case.id
        );
    }

    // And the corpus states the limitation that made those constructions
    // necessary, rather than presenting them as found history.
    let limitations = index.limitations.join(" ");
    assert!(limitations.contains("derived"));
    assert!(limitations.contains("not-computed"));

    // FR-011-CON-3: the corpus makes no claim about the live state of the
    // source repositories, and says so. Retained bytes prove fixture
    // integrity offline; they are not a re-observation.
    assert!(
        limitations.contains("does not re-observe the source repositories"),
        "the corpus index must disclaim any claim about live source state: {limitations}"
    );
}

#[trace("TC-071", "FR-011-AC-3", "FR-015-AC-5")]
#[test]
fn tc_071_every_legacy_case_maps_to_its_recorded_outcome() {
    let (root, index) = corpus();
    let legacy: Vec<_> = index
        .cases
        .iter()
        .filter(|case| case.family.starts_with("pgm01"))
        .collect();
    assert!(!legacy.is_empty(), "the corpus carries no PGM-01 case");

    for case in legacy {
        let view = view(&root, &index, &case.id);
        let outcome = serde_json::to_value(view.outcome)
            .expect("an outcome must serialize")
            .as_str()
            .expect("an outcome must be a string")
            .to_owned();
        assert_eq!(
            outcome, case.expected.outcome,
            "{} changed outcome",
            case.id
        );

        for required in &case.expected.required_mappings {
            assert!(
                mapping_values(&view, &required.source_path)
                    .iter()
                    .any(|value| value.as_str() == Some(required.value.as_str())),
                "{} lost {} -> {}",
                case.id,
                required.source_path,
                required.value
            );
        }
        if case.expected.require_no_mappings {
            assert!(view.mappings.is_empty(), "{} kept a mapping", case.id);
        }
        assert!(
            !view.limitations.is_empty(),
            "{} states no limitation",
            case.id
        );
    }
}

#[trace("TC-072", "FR-011-AC-4", "FR-015-AC-5")]
#[test]
fn tc_072_no_non_success_case_is_read_as_a_success() {
    let (root, index) = corpus();
    let mut examined = 0_usize;
    for kind in [
        "failed",
        "unavailable",
        "not_computed",
        "malformed",
        "tampered",
    ] {
        for case in index.cases_by_kind(kind) {
            let view = view(&root, &index, &case.id);

            // `compatible` is the only outcome that would mean "read
            // cleanly as a current record". None of these may reach it, and
            // none of them may report a passed check either.
            assert_ne!(
                view.outcome,
                Pgm01Outcome::Compatible,
                "{} read as clean",
                case.id
            );
            assert!(
                !mapping_values(&view, "/checks/0/status")
                    .iter()
                    .any(|value| value.as_str() == Some("passed")),
                "{} turned a non-success check into a pass",
                case.id
            );
            examined += 1;
        }
    }
    assert!(examined >= 5, "too few non-success cases were examined");
}

#[trace("TC-073", "FR-011-AC-5", "FR-015-AC-5")]
#[test]
fn tc_073_real_legacy_records_preserve_identity_producer_and_limits() {
    let (root, index) = corpus();
    let case = index
        .case("legacy-v1-passing")
        .expect("the real legacy case must exist");
    let raw = root
        .retained_bytes(case.into())
        .expect("retained bytes must verify");
    let source: Value = serde_json::from_slice(&raw).expect("the record must be JSON");
    let passing = map_pgm01_bytes(&raw, None).expect("the mapper must read a real record");

    for path in [
        "/subjectRevision",
        "/repository",
        "/collector/implementation",
        "/collector/implementationRevision",
        "/environment",
    ] {
        let expected = source
            .pointer(path)
            .unwrap_or_else(|| panic!("the record must carry {path}"));
        assert_eq!(
            mapping_values(&passing, path),
            vec![expected.clone()],
            "{path} drifted"
        );
    }

    // What the legacy record could not carry stays named, not filled in.
    let unmapped: BTreeSet<&str> = passing
        .unmapped_fields
        .iter()
        .map(|field| field.source_path.as_str())
        .collect();
    for required in ["/collector/version", "/configurationDigest", "/decision"] {
        assert!(
            unmapped.contains(required),
            "{required} is not named as unmapped"
        );
    }

    // An inconclusive check survives as inconclusive rather than rounding.
    let inconclusive = view(&root, &index, "legacy-v1-inconclusive");
    let states: BTreeSet<String> = inconclusive
        .mappings
        .iter()
        .filter(|mapping| mapping.target_field == "state")
        .filter_map(|mapping| mapping.value.as_str().map(str::to_owned))
        .collect();
    assert!(states.contains("inconclusive"));
    assert!(states.contains("passed"));
}

#[trace("TC-074", "FR-011-AC-6", "FR-015-AC-5")]
#[test]
fn tc_074_the_current_receipt_validates_against_the_packaged_schema() {
    let (root, index) = corpus();
    let case = index
        .case("current-verification-receipt")
        .expect("the current receipt case must exist");
    let receipt: Value = serde_json::from_slice(
        &root
            .retained_bytes(case.into())
            .expect("retained bytes must verify"),
    )
    .expect("the receipt must be JSON");
    let schema: Value = serde_json::from_slice(
        &root
            .retained_bytes(
                index
                    .chain_artifact("receipt_schema")
                    .expect("the packaged schema must be retained")
                    .into(),
            )
            .expect("the schema must verify"),
    )
    .expect("the schema must be JSON");
    let validator = jsonschema::options()
        .offline()
        .should_validate_formats(true)
        .build(&schema)
        .expect("Quoin's packaged receipt schema must compile");
    assert!(
        validator.validate(&receipt).is_ok(),
        "the retained receipt does not satisfy Quoin's packaged schema"
    );
    assert_eq!(receipt["outcome"].as_str(), Some("valid"));
    assert_eq!(case.expected.outcome, "valid");

    // The receipt binds the record, the attestation, and the retained
    // output that the chain actually carried, not a summary of them.
    let read_chain = |role: &str| -> Value {
        serde_json::from_slice(
            &root
                .retained_bytes(
                    index
                        .chain_artifact(role)
                        .expect("the chain artifact must exist")
                        .into(),
                )
                .expect("the chain artifact must verify"),
        )
        .expect("the chain artifact must be JSON")
    };
    let record = read_chain("change_assurance_record");
    let attestation = read_chain("proof_attestation");
    assert_eq!(receipt["record_digest"], record["digest"]);
    assert_eq!(
        receipt["proofs"][0]["attestation_digest"],
        attestation["digest"]
    );
    assert_eq!(
        receipt["proofs"][0]["retained_output_digest"],
        attestation["retained_output"]["digest"]
    );

    // The Quire export is referenced, not republished. Its identity is read
    // back from the retained attestation rather than asserted separately,
    // so the reference cannot drift from the evidence that binds it.
    let export = index
        .referenced_input("quire_export")
        .expect("the referenced export must exist");
    assert_eq!(export.retention, Retention::Referenced);
    assert_eq!(
        Value::from(export.blake3.clone()),
        attestation["retained_output"]["digest"]
    );
    assert_eq!(
        Value::from(export.size_bytes),
        attestation["retained_output"]["size_bytes"]
    );
    assert_eq!(export.bound_by, "chain/attestation-sealed.json");
    assert!(!export.reason.trim().is_empty());
    assert_eq!(
        receipt["candidate_revision"].as_str(),
        Some(index.chain.subject.revision.as_str())
    );

    // Both sides of the chain are released artifacts, named by their
    // release and pinned to the source revision that produced them.
    let quire = index
        .chain
        .tools
        .get("quire")
        .expect("quire must be pinned");
    let quoin = index
        .chain
        .tools
        .get("quoin")
        .expect("quoin must be pinned");
    assert_eq!(quire.version, "0.31.0");
    assert_eq!(quoin.version, "0.23.1");
    assert_eq!(quoin.release.as_deref(), Some("npm @agent-ix/quoin@0.23.1"));
    assert_eq!(quoin.source_revision.as_deref().map(str::len), Some(40));
    assert!(
        quoin
            .note
            .as_deref()
            .is_some_and(|note| note.contains("released artifact"))
    );
}

#[trace("TC-075", "FR-011-AC-7", "FR-015-AC-5")]
#[test]
fn tc_075_every_producer_case_names_a_real_producer_and_a_shared_concept() {
    let (root, index) = corpus();
    let mut concepts = BTreeSet::new();
    let mut languages = BTreeSet::new();
    for producer in &index.producer_cases {
        // A producer case that names no producer or no path within it records
        // nothing a reader could go back to, so it is coverage on paper only.
        assert!(
            !producer.producer.trim().is_empty(),
            "{} names no producer",
            producer.id
        );
        assert!(
            !producer.path.trim().is_empty(),
            "{} names no source path",
            producer.id
        );
        match producer.retention {
            // Retained bytes have to reproduce the identity the corpus records,
            // or the case is describing output nobody here actually holds.
            Retention::Retained => {
                root.retained_bytes(producer.into())
                    .expect("every retained producer case must reproduce its identity");
            }
            // A referenced case is pinned by digest and deliberately not copied
            // here. It still has to name what it is and why it is not retained,
            // so "referenced" can never become a quiet way to list nothing.
            Retention::Referenced => {
                assert_eq!(
                    producer.source_sha256.len(),
                    64,
                    "{} is unpinned",
                    producer.id
                );
                assert!(
                    producer.retained_path.is_none(),
                    "{} is referenced yet names retained bytes",
                    producer.id
                );
                assert!(
                    producer.note.contains("NOT"),
                    "{} does not say why it is not retained",
                    producer.id
                );
            }
        }
        concepts.insert(producer.feeds);
        languages.insert(producer.language.as_str());
    }

    // Cross-language is the point: a contract that only ever reads its own
    // language's output has not been tested against the campaign.
    for language in ["rust", "typescript"] {
        assert!(
            languages.contains(language),
            "no producer case was read from a {language} producer"
        );
    }

    // The concepts each case feeds are already a closed vocabulary — an
    // unrecognised one would have refused the index above — so what is left to
    // prove is that the cases span the model rather than crowding one corner.
    assert!(
        concepts.len() >= 4,
        "the producer cases exercise too few concepts"
    );
    for concept in [SharedConcept::CheckResult, SharedConcept::Measurement] {
        assert!(
            concepts.contains(&concept),
            "no producer case feeds {}",
            concept.as_str()
        );
    }

    // A real governed producer case, named in the ticket that accepted this
    // corpus, is present and pinned to an exact revision rather than a branch.
    let code_graph = index
        .producer_cases
        .iter()
        .find(|producer| producer.producer == "agent-ix/quire-code-rs")
        .expect("the governed code-graph producer case must be retained");
    assert_eq!(
        code_graph.revision.len(),
        40,
        "{} is not pinned to an exact revision",
        code_graph.id
    );
}

#[trace(
    "TC-076",
    "FR-011-AC-8",
    "FR-011-CON-1",
    "FR-011-CON-4",
    "FR-015-AC-5",
    "FR-015-CON-1"
)]
#[test]
fn tc_076_the_corpus_is_read_only_and_executes_nothing() {
    let (root, index) = corpus();
    let paths = root.corpus_paths().expect("the corpus must enumerate");
    assert!(!paths.is_empty(), "the corpus retains no file");

    // The snapshot re-enumerates the corpus every time rather than reading a
    // population captured once. A file created during mapping is a change to
    // the corpus too, and a snapshot keyed on the paths seen beforehand could
    // never see one: it would compare the same file list to itself and pass.
    let snapshot = |root: &CorpusRoot| -> Vec<(PathBuf, Vec<u8>)> {
        root.corpus_paths()
            .expect("the corpus must enumerate")
            .into_iter()
            .map(|path| {
                let bytes = root
                    .read_relative(&path, MAX_RETAINED_BYTES)
                    .expect("every retained file must be readable");
                (path, bytes)
            })
            .collect()
    };
    let before = snapshot(&root);
    for case in &index.cases {
        if case.family.starts_with("pgm01") {
            let _ = view(&root, &index, &case.id);
        }
    }
    let after = snapshot(&root);
    assert_eq!(
        after.iter().map(|(path, _)| path).collect::<Vec<_>>(),
        before.iter().map(|(path, _)| path).collect::<Vec<_>>(),
        "mapping the corpus added or removed a file"
    );
    assert_eq!(after, before, "mapping the corpus rewrote a retained file");

    // No retained artifact is executable: these are records, and a record
    // that can be run is a record that will be.
    for path in &paths {
        assert!(
            !root
                .is_executable(path)
                .expect("every entry must be inspectable"),
            "{} is executable",
            path.display()
        );
    }

    // Every path the index names is one of the files actually retained, so
    // the read-only assertion covers the population the gate reads.
    let retained: BTreeSet<PathBuf> = paths.iter().cloned().collect();
    for case in &index.cases {
        assert!(
            retained.contains(Path::new(&case.retained_path)),
            "{} names a path the corpus does not retain",
            case.id
        );
    }
}

#[trace("TC-078", "FR-011-AC-10", "FR-011-CON-5", "FR-015-AC-5")]
#[test]
fn tc_078_the_pinned_corpus_is_the_reviewed_corpus() {
    let git = |arguments: &[&str], directory: &Path| -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(arguments)
            .output()
            .expect("git must be available for the pinned-corpus check");
        assert!(output.status.success(), "git {arguments:?} failed");
        String::from_utf8(output.stdout)
            .expect("git output must be UTF-8")
            .trim()
            .to_owned()
    };

    let submodule = repository_root().join(CORPUS_SUBMODULE);
    let recorded = git(&["rev-parse", "HEAD"], &submodule);

    // Read from the index, not HEAD: the index is what a reviewer sees in
    // the diff and what the next commit will carry, so the check holds
    // while the pin is being changed as well as after.
    let gitlink = git(&["ls-files", "-s", CORPUS_SUBMODULE], repository_root());
    let fields: Vec<&str> = gitlink.split_whitespace().collect();
    assert!(!fields.is_empty(), "corpus is not tracked as a gitlink");
    assert_eq!(
        fields[0], "160000",
        "corpus is tracked as files, not a submodule"
    );
    assert_eq!(
        fields[1], recorded,
        "the checked-out corpus is {recorded}, the pinned commit is {}",
        fields[1]
    );

    // An uninitialized corpus fails rather than passing quietly.
    let absent = CorpusRoot::open(Path::new("/nonexistent-engineering-assurance-root"))
        .expect_err("an absent corpus root must refuse");
    assert_eq!(absent.code(), "compatibility_corpus_root_unavailable");

    // A corpus root that is present but whose index cannot be read is a
    // different failure, and it must say so. Collapsing it into the refusal
    // above would tell an operator to initialize a submodule that is already
    // checked out, and would hide every size and read refusal behind one code.
    let root = CorpusRoot::open(repository_root()).expect("the pinned corpus must be readable");
    assert_eq!(
        root.load_index_within(1)
            .expect_err("an index beyond the read bound must refuse")
            .code(),
        "compatibility_corpus_entry_too_large"
    );
}

#[trace("TC-103", "FR-015-AC-3", "FR-015-AC-5", "FR-015-CON-1")]
#[test]
fn tc_103_the_confined_reader_refuses_every_escaping_or_invalid_path() {
    let (root, index) = corpus();
    for escaping in [
        Path::new(""),
        Path::new("/etc/passwd"),
        Path::new("../corpus.json"),
        Path::new("records/../../compatibility/corpus.json"),
        Path::new("./corpus.json"),
    ] {
        let error = root
            .read_relative(escaping, MAX_RETAINED_BYTES)
            .expect_err("an escaping or non-normal path must refuse");
        assert_eq!(
            error.code(),
            "compatibility_corpus_entry_invalid",
            "{} was not refused",
            escaping.display()
        );
    }

    // A directory is not a record, and a missing entry is not an empty one.
    for invalid in [Path::new("records"), Path::new("records/not-a-record.json")] {
        assert_eq!(
            root.read_relative(invalid, MAX_RETAINED_BYTES)
                .expect_err("a non-regular or absent entry must refuse")
                .code(),
            "compatibility_corpus_entry_invalid"
        );
    }

    // A retained artifact larger than the accepted bound refuses before it
    // is read into memory.
    let case = index.case("legacy-v1-passing").expect("case must exist");
    assert_eq!(
        root.read_relative(Path::new(&case.retained_path), 1)
            .expect_err("an oversized entry must refuse")
            .code(),
        "compatibility_corpus_entry_too_large"
    );

    // The accepted corpus nests far inside the depth bound, so the bound that
    // keeps a deep tree from exhausting the stack instead of refusing is not
    // also quietly refusing the corpus this gate is meant to read.
    let deepest = root
        .corpus_paths()
        .expect("the corpus must enumerate")
        .iter()
        .map(|path| path.components().count())
        .max()
        .expect("the corpus retains at least one file");
    assert!(
        deepest < MAX_CORPUS_DEPTH,
        "the corpus nests {deepest} components deep, against a bound of {MAX_CORPUS_DEPTH}"
    );

    // A referenced producer case is refused before any path is resolved.
    let referenced = index
        .producer_cases
        .iter()
        .find(|producer| producer.retention == Retention::Referenced)
        .expect("the corpus must carry a referenced producer case");
    assert_eq!(
        root.retained_bytes(referenced.into())
            .expect_err("a referenced artifact must refuse")
            .code(),
        "compatibility_corpus_artifact_not_retained"
    );

    // And tampered bytes refuse against the identity the corpus records.
    let tampered = RetainedArtifact {
        retained_sha256: Some(&"0".repeat(64)),
        ..RetainedArtifact::from(case)
    };
    assert_eq!(
        root.retained_bytes(tampered)
            .expect_err("tampered bytes must refuse")
            .code(),
        "compatibility_corpus_digest_mismatch"
    );
}

#[trace("TC-103", "FR-015-AC-5", "FR-015-CON-1")]
#[test]
fn tc_103_the_walk_refuses_exactly_one_file_past_its_population_bound() {
    let (root, _index) = corpus();
    let all = root
        .corpus_paths()
        .expect("the pinned corpus must enumerate under the shipping bounds");
    let population = all.len();
    assert!(
        population > 1,
        "a one-file corpus cannot distinguish a bound from its neighbour"
    );

    // Exactly at the bound the walk succeeds. This half is what catches a
    // comparison shifted the other way: a case that only ever refuses a wildly
    // oversized population passes whether the source reads `>=` or `>`, which
    // is how a bound comes to look defended while nothing holds it in place.
    assert_eq!(
        root.corpus_paths_within(population, MAX_CORPUS_DEPTH)
            .expect("a population exactly at the bound must be enumerated")
            .len(),
        population
    );

    // One file past it refuses, with its own code rather than collapsing into
    // the generic invalid-entry answer.
    assert_eq!(
        root.corpus_paths_within(population - 1, MAX_CORPUS_DEPTH)
            .expect_err("a population past the bound must refuse")
            .code(),
        "compatibility_corpus_population_too_large"
    );
}

#[trace("TC-103", "FR-015-AC-5", "FR-015-CON-1")]
#[test]
fn tc_103_the_walk_refuses_exactly_one_directory_past_its_depth_bound() {
    let (root, _index) = corpus();
    let all = root
        .corpus_paths()
        .expect("the pinned corpus must enumerate under the shipping bounds");

    // The walk descends one level per directory, so the deepest recursion a
    // path forces is one less than its component count.
    let deepest = all
        .iter()
        .map(|path| path.components().count() - 1)
        .max()
        .expect("the corpus retains at least one file");
    assert!(
        deepest > 0,
        "a flat corpus cannot distinguish a depth bound from its neighbour"
    );

    // The depth bound exists because the file count does not bound a recursive
    // walk: a tree that is deep rather than wide exhausts the stack, and a
    // crash is not a refusal an operator can act on. Both halves are asserted
    // for the same reason as the population bound above — at the bound it
    // descends, one past it refuses — so neither removing the check nor
    // shifting its comparison leaves this suite green.
    assert_eq!(
        root.corpus_paths_within(MAX_CORPUS_ENTRIES, deepest)
            .expect("a tree exactly at the depth bound must be enumerated")
            .len(),
        all.len()
    );

    assert_eq!(
        root.corpus_paths_within(MAX_CORPUS_ENTRIES, deepest - 1)
            .expect_err("a tree past the depth bound must refuse")
            .code(),
        "compatibility_corpus_tree_too_deep"
    );
}

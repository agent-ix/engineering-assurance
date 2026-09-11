// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Qualification of this repository's canonical discovery bundle.
//!
//! The reader below opens this repository root once through a confined
//! directory handle and reads the four host manifests, the skill inventory, and
//! the promoted workflow definitions out of it. It writes nothing, executes
//! nothing, and refuses a symlink at the entries it opens.
//!
//! The reader lives in the qualification tree rather than in the library
//! because the library is the filesystem-free boundary FR-014 governs, and
//! rather than in the binary because no production command discovers the
//! bundle: the bundle is read to qualify it, and nowhere else. Agent hosts
//! resolve it themselves, through their own installers.

use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
};

use cap_std::{ambient_authority, fs::Dir};
use engineering_assurance::discovery::{
    CANONICAL_SKILL_FILE, CANONICAL_SKILL_NAME, CANONICAL_SKILLS_DIRECTORY,
    CANONICAL_WORKFLOWS_DIRECTORY, DiscoveryError, HOST_SURFACES, HostSurface, ResolvedSkill,
    WORKFLOW_DEFINITION_NAME, expected_workflows, validate_manifest_document, validate_resolution,
    validate_skill_inventory, validate_surface_set, validate_workflow_set,
};
use ix_trace_rs::trace;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The largest bundle file this adapter will read.
///
/// A thin manifest is a few hundred bytes and the canonical skill a few
/// thousand. The bound exists so an unexpectedly large file is a refusal with a
/// name rather than an allocation.
const MAX_BUNDLE_FILE_BYTES: usize = 1_048_576;

/// The most skills or workflow definitions this adapter will enumerate.
const MAX_BUNDLE_ENTRIES: usize = 256;

/// One opened, confined repository bundle root.
struct BundleRoot {
    directory: Dir,
}

impl BundleRoot {
    /// Open this repository's root through a confined directory handle.
    fn open() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let directory = Dir::open_ambient_dir(&root, ambient_authority())
            .expect("the repository root must be openable as a confined directory");
        Self { directory }
    }

    /// Read one bundle-relative regular file, refusing a symlink at its leaf.
    ///
    /// A symlinked manifest or skill would let the bundle appear canonical
    /// while the bytes a host actually installs come from outside it, which is
    /// the escape the bundle boundary exists to prevent.
    fn read(&self, relative: &str) -> Vec<u8> {
        let metadata = self
            .directory
            .symlink_metadata(relative)
            .unwrap_or_else(|error| panic!("cannot classify {relative}: {error}"));
        assert!(
            !metadata.file_type().is_symlink(),
            "{relative} must be a regular file, not a symlink"
        );
        assert!(metadata.is_file(), "{relative} must be a regular file");
        let mut file = self
            .directory
            .open(relative)
            .unwrap_or_else(|error| panic!("cannot open {relative}: {error}"));
        let mut bytes = Vec::new();
        file.by_ref()
            .take(
                u64::try_from(MAX_BUNDLE_FILE_BYTES)
                    .expect("the read bound must fit a file offset")
                    + 1,
            )
            .read_to_end(&mut bytes)
            .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
        assert!(
            bytes.len() <= MAX_BUNDLE_FILE_BYTES,
            "{relative} is larger than this adapter will read"
        );
        bytes
    }

    /// Parse one host manifest out of the bundle.
    fn manifest(&self, surface: &HostSurface) -> Value {
        serde_json::from_slice(&self.read(surface.manifest))
            .unwrap_or_else(|error| panic!("{} must be JSON: {error}", surface.manifest))
    }

    /// Names of the immediate subdirectories of one bundle-relative directory.
    fn subdirectories(&self, relative: &str) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let listing = self
            .directory
            .read_dir(relative)
            .unwrap_or_else(|error| panic!("cannot enumerate {relative}: {error}"));
        for entry in listing {
            let entry = entry.unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
            let kind = entry
                .file_type()
                .unwrap_or_else(|error| panic!("cannot classify an entry of {relative}: {error}"));
            assert!(
                !kind.is_symlink(),
                "{relative} must not contain a symlinked entry"
            );
            if kind.is_dir() {
                names.insert(
                    entry
                        .file_name()
                        .into_string()
                        .expect("bundle directory names must be UTF-8"),
                );
            }
            assert!(
                names.len() <= MAX_BUNDLE_ENTRIES,
                "{relative} holds more entries than this adapter will enumerate"
            );
        }
        names
    }

    /// Bundle-relative paths of every skill file the bundle exposes.
    fn skill_inventory(&self) -> BTreeSet<String> {
        self.subdirectories(CANONICAL_SKILLS_DIRECTORY)
            .into_iter()
            .map(|name| format!("{CANONICAL_SKILLS_DIRECTORY}/{name}/SKILL.md"))
            .filter(|relative| self.directory.is_file(relative))
            .collect()
    }

    /// Names of every workflow the canonical skill exposes a definition for.
    fn workflow_inventory(&self) -> BTreeSet<String> {
        self.subdirectories(CANONICAL_WORKFLOWS_DIRECTORY)
            .into_iter()
            .filter(|name| {
                self.directory.is_file(format!(
                    "{CANONICAL_WORKFLOWS_DIRECTORY}/{name}/{WORKFLOW_DEFINITION_NAME}"
                ))
            })
            .collect()
    }
}

/// Lowercase hexadecimal digest of one byte string.
fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut text, byte| {
            use std::fmt::Write as _;
            write!(text, "{byte:02x}").expect("a String write cannot fail");
            text
        })
}

/// Resolve every supported host against this repository's real manifests.
fn resolve_bundle(bundle: &BundleRoot) -> Vec<ResolvedSkill> {
    validate_surface_set(&HOST_SURFACES).expect("the declared host set must be the supported set");
    HOST_SURFACES
        .iter()
        .map(|surface| {
            validate_manifest_document(surface, &bundle.manifest(surface)).unwrap_or_else(|error| {
                panic!("{} must resolve canonically: {error}", surface.name)
            })
        })
        .collect()
}

#[trace("TC-009", "FR-002-AC-1", "TC-010", "FR-002-AC-2", "US-002-EX-1")]
#[test]
fn tc_009_the_bundle_owns_one_skill_that_every_supported_host_resolves() {
    let bundle = BundleRoot::open();

    // The inventory is asserted as a set rather than by a "the canonical skill
    // exists" check, because a second skill file passes the existence check and
    // is exactly the drift this refuses: both validate, both are installable,
    // and nothing records which one a host was supposed to run.
    let skills = bundle.skill_inventory();
    validate_skill_inventory(&skills).expect("the bundle must own exactly one skill");
    assert_eq!(skills.len(), 1, "the skill population must not be empty");
    assert!(skills.contains(CANONICAL_SKILL_FILE));
    assert!(CANONICAL_SKILL_FILE.contains(CANONICAL_SKILL_NAME));

    let resolved = resolve_bundle(&bundle);
    validate_resolution(&resolved).expect("every supported host must resolve one skill");
    assert_eq!(
        resolved.len(),
        HOST_SURFACES.len(),
        "a host that resolved nothing must not leave the population short"
    );

    // Each host is asserted to have resolved through its own manifest file, so
    // this cannot pass with four entries produced from one manifest read four
    // times — the shape that makes an agreement check agree with itself.
    let by_manifest = resolved
        .iter()
        .map(|entry| (entry.manifest, entry.surface))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(by_manifest.len(), HOST_SURFACES.len());
    for surface in &HOST_SURFACES {
        assert_eq!(by_manifest.get(surface.manifest), Some(&surface.name));
        assert!(
            !bundle.read(surface.manifest).is_empty(),
            "{} must be a real file in this bundle",
            surface.manifest
        );
    }
}

#[trace("TC-011", "FR-002-AC-3", "US-002-EX-2")]
#[test]
fn tc_011_the_canonical_skill_exposes_exactly_the_promoted_workflows() {
    let bundle = BundleRoot::open();
    let workflows = bundle.workflow_inventory();

    // Compared as a set in both directions, and then asserted non-empty on its
    // own. A check written as a loop over whatever the directory listed would
    // pass over a renamed or emptied workflows directory, which is the case
    // this criterion exists to catch.
    validate_workflow_set(&workflows).expect("the canonical workflow set must be exact");
    assert_eq!(workflows, expected_workflows());
    assert_eq!(workflows.len(), 4);

    // Each promoted name must be backed by a definition with bytes in it. An
    // empty def.yaml satisfies the name set and starts no run.
    for name in &workflows {
        let relative = format!("{CANONICAL_WORKFLOWS_DIRECTORY}/{name}/{WORKFLOW_DEFINITION_NAME}");
        assert!(
            !bundle.read(&relative).is_empty(),
            "{relative} must carry a definition"
        );
    }
}

#[trace("TC-038", "NFR-001-AC-1", "NFR-001-AC-2", "NFR-001-AC-3")]
#[test]
fn tc_038_supported_hosts_install_the_same_canonical_bytes() {
    let bundle = BundleRoot::open();
    let resolved = resolve_bundle(&bundle);

    // Host parity is asserted over the bytes each host would install, not over
    // the digest set of a path list. Discovery returns the same path for every
    // host by construction, so a digest set built from those paths has one
    // member whatever the bundle contains and can never fail.
    let skill_bytes = bundle.read(CANONICAL_SKILL_FILE);
    assert!(!skill_bytes.is_empty());
    let canonical_digest = digest(&skill_bytes);
    for entry in &resolved {
        assert_eq!(entry.skill_file, CANONICAL_SKILL_FILE);
        assert_eq!(digest(&bundle.read(&entry.skill_file)), canonical_digest);
    }

    // NFR-001-AC-2 is about the promoted workflow definitions rather than the
    // skill file, and nothing was checking it. Every host installs one skill
    // tree, so the definitions are shared only if each promoted name resolves
    // to bytes inside that tree; four distinct digests prove four distinct
    // definitions rather than one file found four times.
    let definitions = expected_workflows()
        .into_iter()
        .map(|name| {
            let relative =
                format!("{CANONICAL_WORKFLOWS_DIRECTORY}/{name}/{WORKFLOW_DEFINITION_NAME}");
            (name, digest(&bundle.read(&relative)))
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(definitions.len(), 4);
    assert_eq!(
        definitions.values().collect::<BTreeSet<_>>().len(),
        4,
        "two promoted workflows must not share one definition file"
    );

    // NFR-001-AC-3 says no manifest carries an agent-specific copy of the
    // behavioural content. The allowed-key rule refuses a new key, but a copy
    // pasted into an allowed key's value would pass it, so the manifest bytes
    // are checked against the skill's own prose as well.
    let skill_text = String::from_utf8(skill_bytes).expect("the canonical skill must be UTF-8");
    let sentences = skill_text
        .lines()
        .map(str::trim)
        .filter(|line| line.len() > 40 && !line.starts_with('#') && !line.starts_with('-'))
        .collect::<Vec<_>>();
    assert!(
        sentences.len() > 4,
        "the behavioural population must not be empty"
    );
    for surface in &HOST_SURFACES {
        let manifest = String::from_utf8(bundle.read(surface.manifest))
            .unwrap_or_else(|error| panic!("{} must be UTF-8: {error}", surface.manifest));
        for sentence in &sentences {
            assert!(
                !manifest.contains(sentence),
                "{} carries a copy of the canonical behaviour",
                surface.manifest
            );
        }
    }
}

#[trace("TC-042", "FR-002-CON-2", "TC-012", "FR-002-AC-4")]
#[test]
fn tc_042_real_manifests_are_metadata_and_one_target_only() {
    let bundle = BundleRoot::open();

    // The adverse cases live in the library's own tests, where a document can be
    // constructed. This case runs the same rule over the four manifests this
    // repository actually ships, which is the population a host installs from.
    for surface in &HOST_SURFACES {
        let document = bundle.manifest(surface);
        let object = document
            .as_object()
            .unwrap_or_else(|| panic!("{} must be a JSON object", surface.manifest));
        assert!(!object.is_empty(), "{} must not be empty", surface.manifest);
        for key in object.keys() {
            assert!(
                surface.allowed_keys.contains(&key.as_str()),
                "{} carries unsupported key {key:?}",
                surface.manifest
            );
        }
        validate_manifest_document(surface, &document)
            .unwrap_or_else(|error| panic!("{} must be thin: {error}", surface.manifest));

        // Adding behaviour to the shipped manifest must refuse. Constructing the
        // refusal from the real document rather than from a fixture is what makes
        // this fail if the shipped manifest ever stops being validated at all.
        let mut behavioural = document.clone();
        behavioural["instructions"] = Value::String("copied behavioral instructions".to_owned());
        let refusal = validate_manifest_document(surface, &behavioural)
            .expect_err("embedded behaviour must refuse");
        assert!(matches!(
            refusal,
            DiscoveryError::UnsupportedManifestKeys { .. }
        ));
    }
}

#[trace("TC-013", "FR-002-AC-5", "TC-041", "FR-002-CON-1")]
#[test]
fn tc_013_absent_and_escaping_targets_are_refused_against_the_real_bundle() {
    let bundle = BundleRoot::open();
    let surface = HOST_SURFACES[0];

    // The target is redirected in a copy of the shipped manifest, so each case
    // starts from a document that really validates and differs from it in one
    // field. A hand-written fixture would prove only that the fixture is wrong.
    let document = bundle.manifest(&surface);
    for target in [
        "./missing-skills",
        "../../outside",
        "/etc",
        "engineering_assurance/skills/assurance-onboarding",
        "engineering_assurance",
    ] {
        let mut redirected = document.clone();
        redirected["skills"] = Value::String(target.to_owned());
        let Err(refusal) = validate_manifest_document(&surface, &redirected) else {
            panic!("{target:?} must not resolve canonically");
        };
        assert!(
            matches!(
                refusal,
                DiscoveryError::TargetEscapesBundle { .. }
                    | DiscoveryError::TargetNotCanonical { .. }
            ),
            "{target:?} refused for the wrong reason: {refusal}"
        );
    }

    // An absent target is refused by the bundle boundary and confirmed absent
    // here, so the criterion covers "missing" as well as "escaping".
    assert!(!bundle.directory.exists("missing-skills"));
    assert!(bundle.directory.is_file(CANONICAL_SKILL_FILE));

    // The exact-host rule is the other half of FR-002-CON-1: the resolution is
    // only meaningful over the whole declared set.
    let mut short = HOST_SURFACES.to_vec();
    short.pop();
    assert!(validate_surface_set(&short).is_err());
    let resolved = resolve_bundle(&bundle);
    assert!(validate_resolution(&resolved[..3]).is_err());
}

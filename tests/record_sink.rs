// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Record sinks, record forms, sink selection and the fail-closed leak check
//! (FR-054 to FR-057, NFR-008 to NFR-010), against stand-in logs, stores,
//! visibility sources and signers. Nothing touches a network.

#![allow(
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]

use std::{cell::RefCell, collections::HashMap, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;

use engineering_assurance::record_sink::{
    config::{
        ErrorCode, PolicySource, SINK_CONFIG_PATH, SelectionSource, SinkConfig, SinkKind,
        SinkSelection, Visibility, read_policy_document, resolve_from_source, resolve_sink,
    },
    form::{FormError, PrivateRecord, PublicRecord, Record, RecordId, Reference, Signature},
    leak::{
        CheckRefusal, LookupFailed, OffenseReason, PublicPathProof, VisibilityLookup, leak_check,
    },
    sink::{
        CheckedEntry, LogRejected, Ports, PrivateStore, PublicLogClient, RecordSink, Signer,
        SigningFailed, SinkLayer, SinkResult, Status, StoreError,
    },
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const ID: &str = "0b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34";
const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const PUBLISHER: &str = "acme/ledger";
const ENDPOINT: &str = concat!("https", "://log.example.test/api/v1");
const DESTINATION: &str = "store-a";

// ---- stand-ins -----------------------------------------------------------

#[derive(Default)]
struct Store {
    private: RefCell<Vec<(String, PrivateRecord)>>,
    public: RefCell<Vec<(String, PublicRecord)>>,
    fail_private: bool,
    events: Events,
}

impl PrivateStore for Store {
    fn write_private(&self, destination: &str, private: &PrivateRecord) -> Result<(), StoreError> {
        self.events.borrow_mut().push("private_write");
        if self.fail_private {
            return Err(StoreError);
        }
        self.private
            .borrow_mut()
            .push((destination.to_owned(), private.clone()));
        Ok(())
    }

    fn write_public(&self, destination: &str, public: &PublicRecord) -> Result<(), StoreError> {
        self.public
            .borrow_mut()
            .push((destination.to_owned(), public.clone()));
        Ok(())
    }
}

#[derive(Default)]
struct Log {
    entries: RefCell<Vec<(String, PublicRecord)>>,
    reject: bool,
    events: Events,
}

impl PublicLogClient for Log {
    fn send(&self, endpoint: &str, entry: &CheckedEntry) -> Result<(), LogRejected> {
        self.events.borrow_mut().push("send");
        if self.reject {
            return Err(LogRejected);
        }
        self.entries
            .borrow_mut()
            .push((endpoint.to_owned(), entry.record().clone()));
        Ok(())
    }
}

#[derive(Default)]
struct Visibilities {
    map: RefCell<HashMap<String, Result<Visibility, ()>>>,
    calls: RefCell<Vec<String>>,
    events: Events,
}

impl Visibilities {
    fn with(items: &[(&str, Visibility)]) -> Self {
        let this = Self::default();
        for (repo, visibility) in items {
            this.set(repo, Ok(*visibility));
        }
        this
    }

    fn set(&self, repo: &str, value: Result<Visibility, ()>) {
        self.map.borrow_mut().insert(repo.to_owned(), value);
    }
}

impl VisibilityLookup for Visibilities {
    fn repo_visibility(&self, repo: &str) -> Result<Visibility, LookupFailed> {
        self.calls.borrow_mut().push(repo.to_owned());
        self.events.borrow_mut().push("lookup");
        self.map
            .borrow()
            .get(repo)
            .copied()
            .unwrap_or(Err(()))
            .map_err(|()| LookupFailed)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SignMode {
    Honest,
    SignsOther,
    NonCanonical,
    Fails,
}

/// Stand-in for the delegated Sigstore stack. The "bundle" is the hex of the
/// signed payload, padded like base64, so the stand-in reads it back without
/// any cryptography.
struct StandInSigner {
    mode: SignMode,
    public_calls: RefCell<u32>,
    non_public_calls: RefCell<u32>,
    events: Events,
}

impl StandInSigner {
    fn new(mode: SignMode) -> Self {
        Self {
            mode,
            public_calls: RefCell::new(0),
            non_public_calls: RefCell::new(0),
            events: Events::default(),
        }
    }

    fn bundle(&self, canonical: &[u8]) -> Result<String, SigningFailed> {
        self.events.borrow_mut().push("sign");
        let reordered;
        let payload: &[u8] = match self.mode {
            SignMode::Honest => canonical,
            SignMode::SignsOther => b"{\"full\":\"statement\"}",
            SignMode::NonCanonical => {
                // The same members in non-canonical form (insertion order, with
                // whitespace): a broken canonicaliser would produce this.
                let value: Value = serde_json::from_slice(canonical).unwrap();
                reordered = serde_json::to_string_pretty(&value).unwrap().into_bytes();
                &reordered
            }
            SignMode::Fails => return Err(SigningFailed),
        };
        let mut hex: String = payload.iter().map(|b| format!("{b:02x}")).collect();
        while !hex.len().is_multiple_of(4) {
            hex.push('=');
        }
        Ok(hex)
    }
}

impl Signer for StandInSigner {
    fn sign_for_public_log(&self, proof: &PublicPathProof<'_>) -> Result<String, SigningFailed> {
        *self.public_calls.borrow_mut() += 1;
        self.bundle(proof.signed_bytes())
    }

    fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed> {
        *self.non_public_calls.borrow_mut() += 1;
        self.bundle(canonical)
    }

    fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SigningFailed> {
        let hex = bundle.trim_end_matches('=');
        (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| SigningFailed))
            .collect()
    }
}

// ---- fixtures --------------------------------------------------------------

fn public_json(subject: &str, references: &[(&str, &str)]) -> Value {
    json!({
        "protocol": "engineering-assurance.record-public/v1",
        "record_id": ID,
        "kind": "change_record",
        "subject": {"kind": "repo", "id": subject},
        "references": references.iter().map(|(k, i)| json!({"kind": k, "id": i})).collect::<Vec<_>>(),
        "hashes": {"private_form": DIGEST, "subject_commit": COMMIT},
        "verdicts": {"change_record": "recorded"},
        "timestamps": {"created": "2026-11-13T10:00:00Z"}
    })
}

fn record_with(subject: &str, references: &[(&str, &str)]) -> Record {
    let public = PublicRecord::from_json(public_json(subject, references).to_string().as_bytes())
        .expect("public form");
    let mut private = PrivateRecord::new(public.record_id.clone());
    private.ticket_text = Some("Rotate the ledger keys".to_owned());
    private.criteria_text = Some("Keys rotate within a day".to_owned());
    private.model_names = Some(vec!["model-x-7".to_owned()]);
    Record::new(public, private).expect("record")
}

fn record() -> Record {
    record_with(PUBLISHER, &[])
}

fn selection(sink: SinkKind, teams: &[String]) -> SinkSelection {
    SinkSelection {
        sink,
        source: SelectionSource::Configured,
        public_log_endpoint: Some(ENDPOINT.to_owned()),
        private_store_destination: (sink != SinkKind::None).then(|| DESTINATION.to_owned()),
        public_linear_teams: teams.to_vec(),
    }
}

struct World {
    store: Store,
    log: Log,
    lookup: Visibilities,
    signer: StandInSigner,
    teams: Vec<String>,
    events: Events,
}

impl World {
    fn new() -> Self {
        let events = Events::default();
        let mut this = Self {
            store: Store::default(),
            log: Log::default(),
            lookup: Visibilities::with(&[
                (PUBLISHER, Visibility::Public),
                ("acme/open", Visibility::Public),
                ("acme/open2", Visibility::Public),
                ("acme/secret", Visibility::Private),
                ("acme/secret2", Visibility::Private),
            ]),
            signer: StandInSigner::new(SignMode::Honest),
            teams: vec!["PUB".to_owned()],
            events: Rc::clone(&events),
        };
        this.wire();
        this
    }

    /// Points every stand-in at the world's shared event log.
    fn wire(&mut self) {
        self.store.events = Rc::clone(&self.events);
        self.log.events = Rc::clone(&self.events);
        self.lookup.events = Rc::clone(&self.events);
        self.signer.events = Rc::clone(&self.events);
    }

    fn with_signer(mut self, mode: SignMode) -> Self {
        self.signer = StandInSigner::new(mode);
        self.wire();
        self
    }

    fn publish(&self, sink: SinkKind, record: &Record) -> SinkResult {
        self.publish_with(&selection(sink, &self.teams), record)
    }

    fn publish_with(&self, selection: &SinkSelection, record: &Record) -> SinkResult {
        SinkLayer::new(selection, PUBLISHER, self.ports()).publish(record)
    }

    fn ports(&self) -> Ports<'_> {
        Ports {
            store: &self.store,
            lookup: &self.lookup,
            signer: &self.signer,
            client: &self.log,
        }
    }

    fn nothing_sent_or_stored(&self) -> bool {
        self.log.entries.borrow().is_empty()
            && self.store.private.borrow().is_empty()
            && self.store.public.borrow().is_empty()
            && *self.signer.public_calls.borrow() == 0
    }
}

// ---- FR-054 ------------------------------------------------------------------

#[trace("TC-624", "FR-054-AC-1")]
#[test]
fn tc_624_each_implementation_returns_its_status_with_the_closed_result() {
    for (sink, status) in [
        (SinkKind::PublicLog, Status::Published),
        (SinkKind::PrivateStore, Status::Published),
        (SinkKind::None, Status::Discarded),
    ] {
        let world = World::new();
        let result = world.publish(sink, &record());
        assert_eq!(result.status, status, "{sink:?}");
        assert_eq!(result.sink, sink);
        assert_eq!(result.record_id.as_str(), ID);
        let members: Vec<String> = serde_json::to_value(&result)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for member in &members {
            assert!(
                ["protocol", "sink", "status", "record_id"].contains(&member.as_str()),
                "{member}"
            );
        }
    }

    let world = World::new();
    world.publish(SinkKind::PublicLog, &record());
    let private = world.store.private.borrow();
    assert_eq!(private.len(), 1);
    assert_eq!(
        private[0].1.ticket_text.as_deref(),
        Some("Rotate the ledger keys")
    );
    let entries = world.log.entries.borrow();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, ENDPOINT);
    let sent = serde_json::to_string(&entries[0].1).unwrap();
    assert!(!sent.contains("Rotate") && !sent.contains("model-x-7") && !sent.contains("criteria"));
}

#[trace("TC-625", "FR-054-AC-2")]
#[test]
fn tc_625_none_sends_and_stores_nothing() {
    let world = World::new();
    let result = world.publish(SinkKind::None, &record());
    assert_eq!(result.status, Status::Discarded);
    assert!(world.nothing_sent_or_stored());
    assert_eq!(*world.signer.non_public_calls.borrow(), 0);
}

#[trace("TC-626", "FR-054-AC-3")]
#[test]
fn tc_626_a_rejecting_public_log_refuses_and_does_not_fall_back() {
    let mut world = World::new();
    world.log.reject = true;
    let result = world.publish(SinkKind::PublicLog, &record());
    assert_eq!(result.status, Status::Refused);
    assert_eq!(result.error, Some(ErrorCode::PublicLogRejected));
    assert_eq!(world.store.private.borrow().len(), 1);
    assert!(world.store.public.borrow().is_empty());
    assert!(world.log.entries.borrow().is_empty());
}

struct RegisteredSink;

impl RecordSink for RegisteredSink {
    fn publish(&self, record: &Record) -> SinkResult {
        // The registered implementation sees the forms exactly as built.
        assert_eq!(record.public().record_id, record.private().record_id);
        SinkResult::published(SinkKind::PrivateStore, record.id())
    }
}

#[trace("TC-627", "FR-054-AC-4")]
#[test]
fn tc_627_a_registered_implementation_publishes_the_unchanged_forms() {
    let world = World::new();
    let selection = selection(SinkKind::PrivateStore, &world.teams);
    let mut layer = SinkLayer::new(&selection, PUBLISHER, world.ports());
    layer.register_private_store(&RegisteredSink);
    let rec = record();
    let before = rec.clone();
    let result = layer.publish(&rec);
    assert_eq!(result.status, Status::Published);
    assert_eq!(rec, before);
    assert!(world.store.public.borrow().is_empty());
}

#[trace("TC-628", "FR-054-AC-5", "NFR-008-M-1")]
#[test]
fn tc_628_the_result_schema_is_closed() {
    let base = json!({
        "protocol": "engineering-assurance.sink-result/v1",
        "sink": "public_log", "status": "published", "record_id": ID
    });
    assert!(serde_json::from_value::<SinkResult>(base.clone()).is_ok());
    let mut extra = base.clone();
    extra["note"] = json!("x");
    assert!(serde_json::from_value::<SinkResult>(extra).is_err());
    for status in ["accepted", "Published", ""] {
        let mut bad = base.clone();
        bad["status"] = json!(status);
        assert!(
            serde_json::from_value::<SinkResult>(bad).is_err(),
            "{status}"
        );
    }
    // Members must fit the status: `error` only with `refused`, `offending` only
    // with the leak-check code, and `null` is not a member value.
    let refused = |extra: Value| {
        let mut doc = base.clone();
        doc["status"] = json!("refused");
        for (key, value) in extra.as_object().unwrap() {
            doc[key] = value.clone();
        }
        serde_json::from_value::<SinkResult>(doc)
    };
    assert!(refused(json!({})).is_err(), "refused without error");
    assert!(refused(json!({"error": "public_log_rejected"})).is_ok());
    assert!(refused(json!({"error": "public_log_rejected", "offending": []})).is_err());
    assert!(refused(json!({"error": "public_sink_private_reference", "offending": []})).is_ok());
    assert!(refused(json!({"error": null})).is_err());
    for status in ["published", "discarded"] {
        let mut doc = base.clone();
        doc["status"] = json!(status);
        doc["error"] = json!("public_log_rejected");
        assert!(serde_json::from_value::<SinkResult>(doc.clone()).is_err());
        doc.as_object_mut().unwrap().remove("error");
        doc["offending"] = json!([]);
        assert!(serde_json::from_value::<SinkResult>(doc).is_err());
    }
}

#[trace("TC-629", "FR-054-AC-6")]
#[test]
fn tc_629_the_private_form_is_written_first_and_survives_a_refusal() {
    let world = World::new();
    world.lookup.set("acme/secret", Ok(Visibility::Private));
    let leaky = record_with(PUBLISHER, &[("repo", "acme/secret")]);
    let result = world.publish(SinkKind::PublicLog, &leaky);
    assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
    assert_eq!(world.store.private.borrow().len(), 1);
    assert!(world.log.entries.borrow().is_empty());

    let mut failing = World::new();
    failing.store.fail_private = true;
    let result = failing.publish(SinkKind::PublicLog, &record());
    assert_eq!(result.status, Status::Refused);
    assert_eq!(result.error, Some(ErrorCode::PrivateStoreWriteFailed));
    assert!(
        failing.lookup.calls.borrow().is_empty(),
        "check ran before the store write"
    );
    assert!(failing.log.entries.borrow().is_empty());
    assert_eq!(*failing.signer.public_calls.borrow(), 0);

    // The private write precedes every lookup, sign and send.
    let ordered = World::new();
    ordered.publish(SinkKind::PublicLog, &record());
    let events = ordered.events.borrow();
    assert_eq!(events.first(), Some(&"private_write"));
    assert_eq!(events.iter().filter(|e| **e == "private_write").count(), 1);
    assert!(events.contains(&"lookup") && events.contains(&"sign") && events.contains(&"send"));
}

// ---- FR-055 ------------------------------------------------------------------

fn config(extra: Value) -> SinkConfig {
    let mut doc = json!({
        "protocol": "engineering-assurance.sink-config/v1",
        "private_store": {"destination": DESTINATION}
    });
    for (key, value) in extra.as_object().unwrap() {
        doc[key] = value.clone();
    }
    SinkConfig::parse(doc.to_string().as_bytes()).expect("config")
}

#[trace("TC-630", "FR-055-AC-1", "FR-055-AC-2")]
#[test]
fn tc_630_visibility_chooses_the_default_and_an_override_wins() {
    let only_store = config(json!({}));
    let public = resolve_sink(Some(&only_store), Visibility::Public).unwrap();
    assert_eq!(
        (public.sink, public.source),
        (SinkKind::PublicLog, SelectionSource::Default)
    );
    // The default public log has no endpoint to send to: the publish is refused.
    let world = World::new();
    let refused = world.publish_with(&public, &record());
    assert_eq!(refused.error, Some(ErrorCode::SinkConfigIncomplete));
    assert!(world.nothing_sent_or_stored());
    let private = resolve_sink(Some(&only_store), Visibility::Private).unwrap();
    assert_eq!(
        (private.sink, private.source),
        (SinkKind::PrivateStore, SelectionSource::Default)
    );

    for visibility in [Visibility::Public, Visibility::Private] {
        for name in ["private_store", "none"] {
            let cfg = config(json!({"sink": name}));
            let got = resolve_sink(Some(&cfg), visibility).unwrap();
            assert_eq!(got.source, SelectionSource::Configured);
            assert_eq!(serde_json::to_value(got.sink).unwrap(), json!(name));
        }
    }
    let cfg = config(json!({"sink": "public_log", "public_log": {"endpoint": ENDPOINT}}));
    assert_eq!(
        resolve_sink(Some(&cfg), Visibility::Private),
        Err(ErrorCode::SinkConfigPublicLogForNonPublicRepo)
    );
}

#[trace("TC-631", "FR-055-AC-3")]
#[test]
fn tc_631_undeterminable_visibility_counts_as_private() {
    let cfg = config(json!({}));
    let got = resolve_sink(Some(&cfg), Visibility::Unknown).unwrap();
    assert_eq!(got.sink, SinkKind::PrivateStore);
    let cfg = config(json!({"sink": "public_log", "public_log": {"endpoint": ENDPOINT}}));
    assert_eq!(
        resolve_sink(Some(&cfg), Visibility::Unknown),
        Err(ErrorCode::SinkConfigPublicLogForNonPublicRepo)
    );
}

#[trace("TC-632", "FR-055-AC-4", "NFR-008-M-1")]
#[test]
fn tc_632_invalid_and_incomplete_configurations_are_refused() {
    let base = |extra: Value| {
        let mut doc = json!({
            "protocol": "engineering-assurance.sink-config/v1",
            "private_store": {"destination": DESTINATION}
        });
        for (key, value) in extra.as_object().unwrap() {
            doc[key] = value.clone();
        }
        SinkConfig::parse(doc.to_string().as_bytes())
    };
    for bad in [
        json!({"surprise": 1}),
        json!({"sink": "elsewhere"}),
        json!({"sink": null}),
        json!({"public_log": null}),
        json!({"public_log": {"endpoint": null}}),
        json!({"private_store": {"destination": null}}),
        json!({"public_log": {"endpoint": concat!("http", "://log.example.test")}}),
        json!({"public_log": {"endpoint": "https://"}}),
        json!({"public_log": {"endpoint": ENDPOINT, "extra": true}}),
        json!({"private_store": {"destination": ""}}),
        json!({"public_linear_teams": ["pub"]}),
        json!({"protocol": "engineering-assurance.sink-config/v2"}),
    ] {
        assert_eq!(
            base(bad.clone()),
            Err(ErrorCode::SinkConfigInvalid),
            "{bad}"
        );
    }
    assert_eq!(SinkConfig::parse(b"{"), Err(ErrorCode::SinkConfigInvalid));

    let no_endpoint = base(json!({"sink": "public_log"})).unwrap();
    assert_eq!(
        resolve_sink(Some(&no_endpoint), Visibility::Public),
        Err(ErrorCode::SinkConfigIncomplete)
    );
    let no_destination = SinkConfig::parse(
        json!({"protocol": "engineering-assurance.sink-config/v1", "sink": "private_store"})
            .to_string()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(
        resolve_sink(Some(&no_destination), Visibility::Private),
        Err(ErrorCode::SinkConfigIncomplete)
    );
}

struct Branches {
    protected: Option<bool>,
    base: HashMap<String, Vec<u8>>,
}

impl PolicySource for Branches {
    fn base_branch_protected(&self) -> Option<bool> {
        self.protected
    }

    fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
        self.base.get(path).cloned()
    }
}

fn config_doc(sink: &str) -> Vec<u8> {
    json!({
        "protocol": "engineering-assurance.sink-config/v1",
        "sink": sink,
        "private_store": {"destination": DESTINATION}
    })
    .to_string()
    .into_bytes()
}

#[trace("TC-633", "FR-055-AC-5")]
#[test]
fn tc_633_the_protected_base_branch_document_is_the_one_used() {
    let source = Branches {
        protected: Some(true),
        base: HashMap::from([(SINK_CONFIG_PATH.to_owned(), config_doc("none"))]),
    };
    let got = resolve_from_source(&source, Visibility::Public).unwrap();
    assert_eq!(got.sink, SinkKind::None);
}

#[trace("TC-634", "FR-055-AC-6")]
#[test]
fn tc_634_an_unprotected_or_unreported_base_branch_refuses_each_policy_document() {
    for protected in [Some(false), None] {
        let source = Branches {
            protected,
            base: HashMap::new(),
        };
        for path in [
            SINK_CONFIG_PATH,
            ".engineering-assurance/approval-policy.json",
            ".engineering-assurance/workflow-identities.json",
        ] {
            assert_eq!(
                read_policy_document(&source, path),
                Err(ErrorCode::PolicyBranchUnprotected)
            );
        }
        assert_eq!(
            resolve_from_source(&source, Visibility::Public),
            Err(ErrorCode::PolicyBranchUnprotected)
        );
    }
}

#[trace("TC-635", "FR-055-AC-7")]
#[test]
fn tc_635_a_missing_destination_is_incomplete_and_the_first_publish_sends_nothing() {
    let absent = Branches {
        protected: Some(true),
        base: HashMap::new(),
    };
    assert_eq!(
        resolve_from_source(&absent, Visibility::Public),
        Err(ErrorCode::SinkConfigIncomplete)
    );
    let without_destination = Branches {
        protected: Some(true),
        base: HashMap::from([(
            SINK_CONFIG_PATH.to_owned(),
            json!({"protocol": "engineering-assurance.sink-config/v1"})
                .to_string()
                .into_bytes(),
        )]),
    };
    for visibility in [Visibility::Public, Visibility::Private] {
        assert_eq!(
            resolve_from_source(&without_destination, visibility),
            Err(ErrorCode::SinkConfigIncomplete)
        );
    }
    // A selection that reaches the layer without a destination is refused with
    // nothing sent, signed or stored.
    for sink in [SinkKind::PublicLog, SinkKind::PrivateStore] {
        let world = World::new();
        let mut selection = selection(sink, &world.teams);
        selection.private_store_destination = None;
        let result = world.publish_with(&selection, &record());
        assert_eq!(result.status, Status::Refused, "{sink:?}");
        assert_eq!(result.error, Some(ErrorCode::SinkConfigIncomplete));
        assert!(world.nothing_sent_or_stored());
        assert_eq!(*world.signer.non_public_calls.borrow(), 0);
        assert!(world.events.borrow().is_empty());
    }
}

// ---- FR-056 ------------------------------------------------------------------

#[trace("TC-636", "FR-056-AC-1", "FR-056-AC-2", "FR-056-AC-10")]
#[test]
fn tc_636_the_forms_split_private_text_from_public_members() {
    let rec = record();
    let public = serde_json::to_string(rec.public()).unwrap();
    for secret in [
        "Rotate the ledger keys",
        "Keys rotate within a day",
        "model-x-7",
    ] {
        assert!(!public.contains(secret), "{secret}");
    }
    let private = serde_json::to_value(rec.private()).unwrap();
    assert_eq!(private["ticket_text"], "Rotate the ledger keys");
    assert_eq!(private["criteria_text"], "Keys rotate within a day");
    assert_eq!(private["model_names"], json!(["model-x-7"]));
    assert_eq!(rec.public().record_id, rec.private().record_id);
    let members: Vec<&String> = private.as_object().unwrap().keys().collect();
    for member in [
        "kind",
        "subject",
        "references",
        "hashes",
        "verdicts",
        "timestamps",
        "signature",
    ] {
        assert!(!members.iter().any(|m| m.as_str() == member), "{member}");
    }
    let other =
        PrivateRecord::new(RecordId::try_from("11111111-1111-4111-8111-111111111111").unwrap());
    assert_eq!(
        Record::new(rec.public().clone(), other),
        Err(FormError::RecordIdMismatch)
    );
    for member in ["ticket_text", "statement", "model_names"] {
        let private = json!({
            "protocol": "engineering-assurance.record-private/v1",
            "record_id": ID,
            member: null
        });
        assert!(
            PrivateRecord::from_json(private.to_string().as_bytes()).is_err(),
            "{member}"
        );
    }
    let id = RecordId::from_random_bytes([0xff; 16]);
    assert_eq!(id.as_str(), "ffffffff-ffff-4fff-bfff-ffffffffffff");
}

#[trace("TC-637", "FR-056-AC-3", "NFR-008-M-1")]
#[test]
fn tc_637_both_forms_reject_an_extra_member() {
    let mut public = public_json(PUBLISHER, &[]);
    public["model"] = json!("model-x-7");
    assert!(PublicRecord::from_json(public.to_string().as_bytes()).is_err());
    for nested in ["hashes", "verdicts", "timestamps"] {
        let mut public = public_json(PUBLISHER, &[]);
        public[nested]["extra"] = json!("sha256:x");
        assert!(
            PublicRecord::from_json(public.to_string().as_bytes()).is_err(),
            "{nested}"
        );
    }
    let private = json!({
        "protocol": "engineering-assurance.record-private/v1",
        "record_id": ID, "ticket_text": "t", "kind": "approval"
    });
    assert!(PrivateRecord::from_json(private.to_string().as_bytes()).is_err());
}

#[trace("TC-638", "FR-056-AC-4")]
#[test]
fn tc_638_a_value_outside_its_grammar_is_rejected_before_any_sink() {
    let mutations: Vec<Box<dyn Fn(&mut Value)>> = vec![
        Box::new(|v| v["record_id"] = json!("0B9D6A3E-4F1C-4A52-9D0E-6C1F7A2B8E34")),
        Box::new(|v| v["record_id"] = json!("TICKET-12")),
        Box::new(|v| v["hashes"] = json!({"model-x-7": DIGEST})),
        Box::new(|v| v["verdicts"] = json!({"approval": "the reviewer was happy"})),
        Box::new(|v| v["verdicts"] = json!({"secret": "satisfied"})),
        Box::new(|v| v["hashes"]["private_form"] = json!("sha256:ABC")),
        Box::new(|v| v["hashes"]["statement"] = json!(COMMIT)),
        Box::new(|v| v["hashes"]["subject_commit"] = json!(format!("sha256:{COMMIT}"))),
        Box::new(|v| v["hashes"]["subject_commit"] = json!("abc123")),
        Box::new(|v| v["signature"] = json!("not base64!")),
        Box::new(|v| v["hashes"]["statement"] = Value::Null),
        Box::new(|v| v["signature"] = Value::Null),
        Box::new(|v| v["signature"] = json!("abc")),
        Box::new(|v| v["signature"] = json!("A".repeat(16_388))),
        Box::new(|v| v["timestamps"]["created"] = json!("2026-11-13 10:00:00")),
        Box::new(|v| v["timestamps"]["created"] = json!("2026-11-13T10:00:00+00:00")),
        Box::new(|v| v["kind"] = json!("memo")),
        Box::new(|v| v["subject"]["kind"] = json!("linear_issue")),
        Box::new(|v| v["references"] = json!([{"kind": "repo", "id": "no-slash"}])),
        Box::new(|v| v["references"] = json!([{"kind": "linear_issue", "id": "abc-1"}])),
        Box::new(|v| v["references"] = json!([{"kind": "linear_issue", "id": "ABCDEFGHIJK-1"}])),
        Box::new(|v| v["references"] = json!([{"kind": "linear_issue", "id": "ABC-1234567890"}])),
        Box::new(|v| v["references"] = json!([{"kind": "repo", "id": "a/b/c"}])),
        Box::new(|v| {
            v["references"] = Value::Array(
                (0..65)
                    .map(|i| json!({"kind": "repo", "id": format!("o/r{i}")}))
                    .collect(),
            );
        }),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut doc = public_json(PUBLISHER, &[]);
        mutate(&mut doc);
        assert!(
            PublicRecord::from_json(doc.to_string().as_bytes()).is_err(),
            "mutation {index}: {doc}"
        );
    }
    let mut ok = public_json(
        PUBLISHER,
        &[("linear_issue", "ENG-7"), ("repo", "acme/x.y_z-1")],
    );
    ok["hashes"]["subject_commit"] = json!("a".repeat(64));
    ok["signature"] = json!("AAAA".repeat(4096));
    assert!(PublicRecord::from_json(ok.to_string().as_bytes()).is_ok());
    let mut max = public_json(PUBLISHER, &[]);
    max["references"] = Value::Array(
        (0..64)
            .map(|i| json!({"kind": "repo", "id": format!("o/r{i}")}))
            .collect(),
    );
    assert!(PublicRecord::from_json(max.to_string().as_bytes()).is_ok());
}

#[trace("TC-639", "FR-056-AC-5")]
#[test]
fn tc_639_private_content_does_not_change_the_signed_bytes_or_signature() {
    let signer = StandInSigner::new(SignMode::Honest);
    let first = record();
    let mut private = first.private().clone();
    private.ticket_text = Some("entirely different ticket".to_owned());
    private.agent_bom = Some(json!({"bomFormat": "CycloneDX"}));
    let second = Record::new(first.public().clone(), private).unwrap();
    let bytes = first.public().signed_bytes().unwrap();
    assert_eq!(bytes, second.public().signed_bytes().unwrap());
    assert_eq!(
        signer.bundle(&bytes).unwrap(),
        signer
            .bundle(&second.public().signed_bytes().unwrap())
            .unwrap()
    );
}

/// A small deterministic generator: the module has no property-test dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn string(&mut self, alphabet: &[u8], max: usize) -> String {
        let len = (self.next() % (max as u64 + 1)) as usize;
        (0..len)
            .map(|_| alphabet[(self.next() % alphabet.len() as u64) as usize] as char)
            .collect()
    }
}

#[trace("TC-640", "FR-056-AC-6")]
#[test]
fn tc_640_a_member_accepts_a_string_only_when_it_matches_its_grammar() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let noisy = b"abcdef0123456789ABCDEFGHxyz-_/.:+= T";
    for _ in 0..4000 {
        let text = rng.string(noisy, 70);
        let hash = |s: &str| {
            let mut v = public_json(PUBLISHER, &[]);
            v["hashes"] = json!({"statement": s});
            PublicRecord::from_json(v.to_string().as_bytes()).is_ok()
        };
        let expected = text.strip_prefix("sha256:").is_some_and(|h| {
            h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        });
        assert_eq!(hash(&text), expected, "{text}");

        let signature = Signature::try_from(text.clone());
        let b64 = !text.is_empty()
            && text.len().is_multiple_of(4)
            && text.trim_end_matches('=').len() + 2 >= text.len()
            && text
                .trim_end_matches('=')
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
        assert_eq!(signature.is_ok(), b64, "{text}");
        if let Ok(accepted) = signature {
            assert!(
                accepted
                    .as_str()
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
            );
        }

        let repo = Reference::repo(&text).is_ok();
        let repo_expected = text.split_once('/').is_some_and(|(o, n)| {
            let part = |p: &str| {
                (1..=100).contains(&p.len())
                    && p.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            };
            part(o) && part(n)
        });
        assert_eq!(repo, repo_expected, "{text}");

        // Pure noise almost never matches the structured grammars, so each is also
        // fed mutations of a valid sample.
        check_structured(&text);
        for valid in [ID, COMMIT, "2026-11-13T10:00:00Z", "ENG-77"] {
            let mut chars: Vec<char> = valid.chars().collect();
            for _ in 0..(rng.next() % 3) {
                let at = (rng.next() % chars.len() as u64) as usize;
                chars[at] = noisy[(rng.next() % noisy.len() as u64) as usize] as char;
            }
            check_structured(&chars.into_iter().collect::<String>());
        }
    }
}

/// Checks `text` against the grammars of `record_id`, `subject_commit`, the
/// timestamps and Linear issue ids, each by an independent oracle.
fn check_structured(text: &str) {
    let uuid = text.len() == 36
        && text.bytes().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => matches!(c, b'0'..=b'9' | b'a'..=b'f'),
        });
    let commit = matches!(text.len(), 40 | 64)
        && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    let instant = text.len() == 20
        && text.bytes().enumerate().all(|(i, c)| match i {
            4 | 7 => c == b'-',
            10 => c == b'T',
            13 | 16 => c == b':',
            19 => c == b'Z',
            _ => c.is_ascii_digit(),
        })
        && (1..=12).contains(&text[5..7].parse::<u32>().unwrap())
        && (1..=31).contains(&text[8..10].parse::<u32>().unwrap())
        && text[11..13].parse::<u32>().unwrap() < 24
        && text[14..16].parse::<u32>().unwrap() < 60
        && text[17..19].parse::<u32>().unwrap() < 60;
    let issue = text.split_once('-').is_some_and(|(team, n)| {
        (1..=10).contains(&team.len())
            && team.as_bytes()[0].is_ascii_uppercase()
            && team
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && (1..=9).contains(&n.len())
            && n.bytes().all(|b| b.is_ascii_digit())
    });
    let accepted = |member: &dyn Fn(&mut Value)| {
        let mut v = public_json(PUBLISHER, &[]);
        member(&mut v);
        PublicRecord::from_json(v.to_string().as_bytes()).is_ok()
    };
    assert_eq!(accepted(&|v| v["record_id"] = json!(text)), uuid, "{text}");
    assert_eq!(
        accepted(&|v| v["hashes"]["subject_commit"] = json!(text)),
        commit,
        "{text}"
    );
    assert_eq!(
        accepted(&|v| v["timestamps"]["signed"] = json!(text)),
        instant,
        "{text}"
    );
    assert_eq!(Reference::linear_issue(text).is_ok(), issue, "{text}");
}

#[trace("TC-641", "FR-056-AC-7")]
#[test]
fn tc_641_an_approval_statement_is_only_in_the_private_form() {
    let mut doc = public_json(PUBLISHER, &[]);
    doc["kind"] = json!("approval");
    doc["hashes"] = json!({"statement": DIGEST});
    doc["verdicts"] = json!({"approval": "satisfied"});
    let public = PublicRecord::from_json(doc.to_string().as_bytes()).unwrap();
    let mut private = PrivateRecord::new(public.record_id.clone());
    private.statement = Some(json!({
        "meaning": "reviewed the rotation", "pull_request": "acme/ledger#12", "approver": "github:42"
    }));
    let rec = Record::new(public, private).unwrap();
    let world = World::new();
    assert_eq!(
        world.publish(SinkKind::PublicLog, &rec).status,
        Status::Published
    );
    let sent = serde_json::to_string(&world.log.entries.borrow()[0].1).unwrap();
    for secret in ["reviewed the rotation", "acme/ledger#12", "github:42"] {
        assert!(!sent.contains(secret), "{secret}");
    }
    assert!(sent.contains(DIGEST));
    let stored = world.store.private.borrow();
    assert_eq!(
        stored[0].1.statement.as_ref().unwrap()["approver"],
        "github:42"
    );
}

#[trace("TC-642", "FR-056-AC-8", "FR-056-AC-9")]
#[test]
fn tc_642_the_bundle_signs_exactly_the_canonical_public_bytes() {
    let world = World::new();
    let rec = record();
    let result = world.publish(SinkKind::PublicLog, &rec);
    assert_eq!(result.status, Status::Published);
    let sent = world.log.entries.borrow()[0].1.clone();
    let payload = world
        .signer
        .signed_payload(sent.signature.as_ref().unwrap().as_str())
        .unwrap();
    assert_eq!(payload, rec.public().signed_bytes().unwrap());
    // RFC 8785: sorted members, no whitespace, no signature member.
    let text = String::from_utf8(payload).unwrap();
    assert!(text.starts_with("{\"hashes\":{") && !text.contains("signature"));

    for (mode, sink) in [
        (SignMode::SignsOther, SinkKind::PublicLog),
        (SignMode::SignsOther, SinkKind::PrivateStore),
        (SignMode::NonCanonical, SinkKind::PublicLog),
        (SignMode::NonCanonical, SinkKind::PrivateStore),
    ] {
        let other = World::new().with_signer(mode);
        let result = other.publish(sink, &rec);
        assert_eq!(
            result.error,
            Some(ErrorCode::SignaturePayloadMismatch),
            "{sink:?}"
        );
        assert!(other.log.entries.borrow().is_empty());
        assert!(other.store.public.borrow().is_empty());
    }
    let failing = World::new().with_signer(SignMode::Fails);
    assert_eq!(
        failing.publish(SinkKind::PublicLog, &rec).error,
        Some(ErrorCode::SigningFailed)
    );
    assert!(failing.log.entries.borrow().is_empty());
    let mut doc = public_json(PUBLISHER, &[]);
    doc["signature"] = json!("AAAA");
    let public = PublicRecord::from_json(doc.to_string().as_bytes()).unwrap();
    let private = PrivateRecord::new(public.record_id.clone());
    assert_eq!(
        Record::new(public, private),
        Err(FormError::SignatureNotAccepted)
    );
}

// ---- FR-057 ------------------------------------------------------------------

#[trace("TC-643", "FR-057-AC-1", "FR-057-AC-5")]
#[test]
fn tc_643_every_private_reference_is_reported_and_nothing_is_sent() {
    let world = World::new();
    let rec = record_with(
        PUBLISHER,
        &[
            ("repo", "acme/open"),
            ("repo", "acme/secret"),
            ("repo", "acme/open2"),
        ],
    );
    let result = world.publish(SinkKind::PublicLog, &rec);
    assert_eq!(result.status, Status::Refused);
    assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
    let offending = result.offending.unwrap();
    assert_eq!(offending.len(), 1);
    assert_eq!(
        (offending[0].id.as_str(), offending[0].reason),
        ("acme/secret", OffenseReason::Private)
    );
    assert!(world.log.entries.borrow().is_empty());
    assert_eq!(*world.signer.public_calls.borrow(), 0);

    let two = record_with(
        PUBLISHER,
        &[
            ("repo", "acme/secret"),
            ("repo", "acme/secret2"),
            ("repo", "acme/secret"),
        ],
    );
    let result = World::new().publish(SinkKind::PublicLog, &two);
    let ids: Vec<String> = result
        .offending
        .unwrap()
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(ids, ["acme/secret", "acme/secret2"]);
}

#[trace("TC-644", "FR-057-AC-2")]
#[test]
fn tc_644_a_linear_issue_is_public_only_when_its_team_is_listed() {
    let rec = record_with(PUBLISHER, &[("linear_issue", "SECRET-4")]);
    let mut world = World::new();
    let result = world.publish(SinkKind::PublicLog, &rec);
    let offending = result.offending.unwrap();
    assert_eq!(offending[0].id, "SECRET-4");
    assert_eq!(offending[0].reason, OffenseReason::Private);
    world.teams.push("SECRET".to_owned());
    assert_eq!(
        world.publish(SinkKind::PublicLog, &rec).status,
        Status::Published
    );
}

#[trace("TC-645", "FR-057-AC-3")]
#[test]
fn tc_645_an_unknown_visibility_is_refused_under_the_one_code() {
    let rec = record_with(PUBLISHER, &[("repo", "acme/open"), ("repo", "acme/errors")]);
    let world = World::new();
    world.lookup.set("acme/errors", Err(()));
    let result = world.publish(SinkKind::PublicLog, &rec);
    assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
    let offending = result.offending.unwrap();
    assert_eq!(offending.len(), 1);
    assert_eq!(offending[0].reason, OffenseReason::VisibilityUnknown);
    assert!(world.log.entries.borrow().is_empty());

    let world = World::new();
    world.lookup.set("acme/open", Ok(Visibility::Unknown));
    let rec = record_with(PUBLISHER, &[("repo", "acme/open")]);
    assert_eq!(
        world.publish(SinkKind::PublicLog, &rec).offending.unwrap()[0].reason,
        OffenseReason::VisibilityUnknown
    );

    let mut dark = World::new();
    dark.lookup = Visibilities::default();
    dark.wire();
    let result = dark.publish(SinkKind::PublicLog, &rec);
    let offending = result.offending.unwrap();
    assert_eq!(offending.len(), 2);
    assert!(
        offending
            .iter()
            .all(|o| o.reason == OffenseReason::VisibilityUnknown)
    );
    assert!(dark.log.entries.borrow().is_empty());
}

#[trace("TC-646", "FR-057-AC-4")]
#[test]
fn tc_646_the_publisher_and_the_subject_are_checked_even_with_no_references() {
    let world = World::new();
    world.lookup.set(PUBLISHER, Ok(Visibility::Private));
    let result = world.publish(SinkKind::PublicLog, &record());
    let offending = result.offending.unwrap();
    assert_eq!(offending.len(), 1);
    assert_eq!(
        (offending[0].id.as_str(), offending[0].reason),
        (PUBLISHER, OffenseReason::Private)
    );

    let world = World::new();
    let hidden_subject = record_with("acme/secret", &[]);
    let result = world.publish(SinkKind::PublicLog, &hidden_subject);
    assert_eq!(result.offending.unwrap()[0].id, "acme/secret");
}

#[trace("TC-647", "FR-057-AC-6")]
#[test]
fn tc_647_the_check_runs_on_every_publish() {
    let world = World::new();
    assert_eq!(
        world.publish(SinkKind::PublicLog, &record()).status,
        Status::Published
    );
    world.lookup.set(PUBLISHER, Ok(Visibility::Private));
    assert_eq!(
        world.publish(SinkKind::PublicLog, &record()).status,
        Status::Refused
    );
    assert_eq!(world.log.entries.borrow().len(), 1);
}

#[trace("TC-648", "FR-057-AC-7", "FR-057-AC-9")]
#[test]
fn tc_648_private_store_and_none_publish_what_the_public_sink_refused() {
    let rec = record_with(PUBLISHER, &[("repo", "acme/secret")]);
    let public = World::new();
    assert_eq!(
        public.publish(SinkKind::PublicLog, &rec).status,
        Status::Refused
    );
    assert!(public.log.entries.borrow().is_empty());
    assert!(public.store.public.borrow().is_empty());

    let store = World::new();
    assert_eq!(
        store.publish(SinkKind::PrivateStore, &rec).status,
        Status::Published
    );
    assert!(store.lookup.calls.borrow().is_empty());
    assert_eq!(store.store.public.borrow().len(), 1);
    assert!(store.store.public.borrow()[0].1.signature.is_some());

    let none = World::new();
    assert_eq!(none.publish(SinkKind::None, &rec).status, Status::Discarded);
    assert!(none.lookup.calls.borrow().is_empty());
}

#[trace("TC-649", "FR-057-AC-8", "NFR-009-M-1")]
#[test]
fn tc_649_only_all_public_records_reach_or_sign_for_the_public_log() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let repos = ["acme/a", "acme/b", "acme/c", "acme/d"];
    let kinds = [Visibility::Public, Visibility::Private, Visibility::Unknown];
    for _ in 0..500 {
        let world = World::new();
        let mut all_public = true;
        for repo in repos {
            let pick = (rng.next() % 4) as usize;
            let entry = if pick == 3 { Err(()) } else { Ok(kinds[pick]) };
            world.lookup.set(repo, entry);
        }
        world.lookup.set(
            PUBLISHER,
            if rng.next().is_multiple_of(5) {
                Ok(Visibility::Private)
            } else {
                Ok(Visibility::Public)
            },
        );
        let subject = repos[(rng.next() % 4) as usize];
        let count = (rng.next() % 4) as usize;
        let refs: Vec<(&str, &str)> = (0..count)
            .map(|_| ("repo", repos[(rng.next() % 4) as usize]))
            .collect();
        let rec = record_with(subject, &refs);
        for repo in std::iter::once(PUBLISHER)
            .chain(std::iter::once(subject))
            .chain(refs.iter().map(|r| r.1))
        {
            all_public &= world.lookup.map.borrow()[repo] == Ok(Visibility::Public);
        }
        let result = world.publish(SinkKind::PublicLog, &rec);
        let sent = !world.log.entries.borrow().is_empty();
        assert_eq!(sent, all_public);
        assert_eq!(*world.signer.public_calls.borrow() == 1, all_public);
        assert_eq!(result.status == Status::Published, all_public);
        if !all_public {
            assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
        }
        let direct = leak_check(PUBLISHER, rec.public(), &[], &world.lookup);
        assert_eq!(direct.is_ok(), all_public);
    }
}

#[trace(
    "TC-650",
    "FR-057-AC-10",
    "FR-057-AC-11",
    "NFR-010-AC-1",
    "NFR-010-M-2"
)]
#[test]
fn tc_650_signing_uses_the_public_path_only_after_the_check_passed() {
    let rec = record();
    for sink in [SinkKind::PrivateStore, SinkKind::None] {
        let world = World::new();
        world.lookup.set(PUBLISHER, Ok(Visibility::Private));
        world.publish(sink, &rec);
        assert_eq!(*world.signer.public_calls.borrow(), 0, "{sink:?}");
    }
    let world = World::new();
    world.publish(SinkKind::PublicLog, &rec);
    assert_eq!(*world.signer.public_calls.borrow(), 1);
    assert_eq!(*world.signer.non_public_calls.borrow(), 0);
    assert_eq!(
        world.lookup.calls.borrow().first().map(String::as_str),
        Some(PUBLISHER)
    );

    let refused = World::new();
    refused.lookup.set("acme/secret", Ok(Visibility::Private));
    refused.publish(
        SinkKind::PublicLog,
        &record_with(PUBLISHER, &[("repo", "acme/secret")]),
    );
    assert_eq!(*refused.signer.public_calls.borrow(), 0);
}

#[trace("TC-651", "NFR-010-AC-2", "NFR-010-M-1")]
#[test]
fn tc_651_signing_is_delegated_and_record_sink_has_only_its_declared_dependencies() {
    use std::collections::BTreeSet;

    use syn::{Expr, ExprMethodCall, ItemImpl, ItemTrait, Type, visit::Visit};

    #[derive(Default)]
    struct Boundary {
        signer_methods: Vec<String>,
        signer_implementations: Vec<String>,
        signer_calls: Vec<(String, Option<String>)>,
        signer_port_is_trait_object: bool,
    }

    fn field_path(expression: &Expr) -> Option<String> {
        match expression {
            Expr::Path(path) => Some(
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            Expr::Field(field) => Some(format!(
                "{}.{}",
                field_path(&field.base)?,
                match &field.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                }
            )),
            _ => None,
        }
    }

    impl<'ast> Visit<'ast> for Boundary {
        fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
            if item.ident == "Signer" {
                self.signer_methods
                    .extend(item.items.iter().filter_map(|item| {
                        if let syn::TraitItem::Fn(method) = item {
                            Some(method.sig.ident.to_string())
                        } else {
                            None
                        }
                    }));
            }
            syn::visit::visit_item_trait(self, item);
        }

        fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
            if item.trait_.as_ref().is_some_and(|(path, _)| {
                path.segments
                    .last()
                    .is_some_and(|segment| segment.ident == "Signer")
            }) {
                self.signer_implementations.push("impl Signer".to_owned());
            }
            syn::visit::visit_item_impl(self, item);
        }

        fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
            if matches!(
                call.method.to_string().as_str(),
                "sign_for_public_log" | "sign_without_public_log" | "signed_payload"
            ) {
                self.signer_calls
                    .push((call.method.to_string(), field_path(&call.receiver)));
            }
            syn::visit::visit_expr_method_call(self, call);
        }
    }

    fn visit_rust_files(path: &std::path::Path, boundary: &mut Boundary) {
        for entry in std::fs::read_dir(path).expect("read sink source directory") {
            let path = entry.expect("read sink source entry").path();
            if path.is_dir() {
                visit_rust_files(&path, boundary);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).expect("read sink source");
                let syntax = syn::parse_file(&source).expect("parse sink Rust source");
                boundary.visit_file(&syntax);
                if path.file_name().is_some_and(|name| name == "sink.rs") {
                    let ports = syntax
                        .items
                        .iter()
                        .find_map(|item| match item {
                            syn::Item::Struct(item) if item.ident == "Ports" => Some(item),
                            _ => None,
                        })
                        .expect("the sink ports type exists");
                    let signer = ports
                        .fields
                        .iter()
                        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == "signer"))
                        .expect("the sink ports carry the injected signer");
                    boundary.signer_port_is_trait_object = match &signer.ty {
                        Type::Reference(reference) => match reference.elem.as_ref() {
                            Type::TraitObject(object) => object.bounds.iter().any(|bound| {
                                matches!(bound, syn::TypeParamBound::Trait(trait_bound)
                                    if trait_bound.path.segments.last().is_some_and(|segment| segment.ident == "Signer"))
                            }),
                            _ => false,
                        },
                        _ => false,
                    };
                }
            }
        }
    }

    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/record_sink");
    let mut boundary = Boundary::default();
    visit_rust_files(&source_dir, &mut boundary);
    assert_eq!(
        boundary.signer_methods,
        [
            "sign_for_public_log",
            "sign_without_public_log",
            "signed_payload"
        ]
    );
    assert!(boundary.signer_implementations.is_empty());
    assert!(boundary.signer_port_is_trait_object);
    let signer_calls: BTreeSet<_> = boundary.signer_calls.into_iter().collect();
    assert_eq!(
        signer_calls,
        BTreeSet::from([
            (
                "sign_for_public_log".to_owned(),
                Some("self.ports.signer".to_owned())
            ),
            (
                "sign_without_public_log".to_owned(),
                Some("self.ports.signer".to_owned())
            ),
            ("signed_payload".to_owned(), Some("verifier".to_owned())),
        ])
    );

    let metadata = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--no-deps",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs without building or network access");
    assert!(
        metadata.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: Value = serde_json::from_slice(&metadata.stdout).expect("cargo metadata JSON");
    let package = metadata["packages"]
        .as_array()
        .expect("cargo metadata packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .expect("engineering-assurance package metadata");
    let dependencies: std::collections::HashMap<_, _> = package["dependencies"]
        .as_array()
        .expect("package dependencies")
        .iter()
        .map(|dependency| {
            let package_name = dependency["name"]
                .as_str()
                .expect("dependency package name");
            let alias = dependency["rename"].as_str().unwrap_or(package_name);
            (alias, package_name)
        })
        .collect();
    assert!(
        package["dependencies"]
            .as_array()
            .expect("package dependencies")
            .iter()
            .filter(|dependency| dependency["kind"].is_null())
            .all(|dependency| dependency["optional"] == true)
    );
    let sink_dependencies: BTreeSet<_> = package["features"]["record-sink"]
        .as_array()
        .expect("record-sink feature")
        .iter()
        .map(|dependency| {
            let alias = dependency
                .as_str()
                .and_then(|value| value.strip_prefix("dep:"))
                .expect("record-sink enables only direct dependencies");
            *dependencies
                .get(alias)
                .expect("record-sink dependency alias resolves")
        })
        .collect();
    assert_eq!(
        sink_dependencies,
        BTreeSet::from([
            "serde",
            "serde_json",
            "serde_json_canonicalizer",
            "thiserror"
        ])
    );
}

/// A sink that would reach any log if it could. It stands in for a pluggable
/// implementation: it has no way to build the entry type the log client takes.
struct Rogue(RefCell<u32>);

impl RecordSink for Rogue {
    fn publish(&self, record: &Record) -> SinkResult {
        *self.0.borrow_mut() += 1;
        SinkResult::published(SinkKind::PublicLog, record.id())
    }
}

#[trace("TC-652", "FR-057-AC-8", "NFR-009-M-2")]
#[test]
fn tc_652_the_public_log_is_reachable_only_through_the_checked_sink() {
    // A registered implementation takes the private-store slot only; a public-log
    // selection still goes through the leak check, and a leaking record sends nothing.
    let world = World::new();
    let rogue = Rogue(RefCell::new(0));
    let selection = selection(SinkKind::PublicLog, &world.teams);
    let mut layer = SinkLayer::new(&selection, PUBLISHER, world.ports());
    layer.register_private_store(&rogue);
    let leaking = record_with(PUBLISHER, &[("repo", "acme/secret")]);
    let result = layer.publish(&leaking);
    assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
    assert_eq!(*rogue.0.borrow(), 0);
    assert!(world.log.entries.borrow().is_empty());
    assert_eq!(*world.signer.public_calls.borrow(), 0);

    // A check that failed to encode, or refused, yields no proof to sign with.
    world.lookup.set(PUBLISHER, Ok(Visibility::Private));
    let public = record();
    assert!(matches!(
        leak_check(PUBLISHER, public.public(), &[], &world.lookup),
        Err(CheckRefusal::Offending(_))
    ));
    world.lookup.set(PUBLISHER, Ok(Visibility::Public));
    let proof = leak_check(PUBLISHER, public.public(), &[], &world.lookup).unwrap();
    assert_eq!(
        proof.signed_bytes(),
        public.public().signed_bytes().unwrap()
    );
}

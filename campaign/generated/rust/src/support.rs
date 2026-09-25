//! Support types the generated declarations are composed from.
//!
//! Every named type the mapping table refers to lives here: the nullability
//! wrapper, the dynamic value the unknown-member policies retain, the validated
//! identity and locus-path newtypes, the four validated scalar newtypes, the
//! generated pattern matcher, and the validation error every fallible
//! constructor returns.
//!
//! The crate depends on `serde` and on nothing else, so the matcher is a
//! generated program executed by the interpreter below rather than a call into
//! a regular-expression engine, and the instant arithmetic a date or date-time
//! bound is compared with is written out rather than taken from a date library.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

/// The longest input-derived fragment a validation message echoes.
pub const MAX_ECHOED_CODE_POINTS: usize = 120;

/// Truncates input-derived text at the declared code-point bound.
///
/// The cut is on a code point rather than on a byte, because slicing a `String`
/// between the bytes of one scalar value is not text and would panic.
pub fn truncate_echo(input: &str) -> String {
    let mut out = String::new();
    for (index, ch) in input.chars().enumerate() {
        if index >= MAX_ECHOED_CODE_POINTS {
            out.push('\u{2026}');
            break;
        }
        out.push(ch);
    }
    out
}

/// A constraint the value did not satisfy.
///
/// The error names the constraint's semantic identity, the keyword, the path of
/// the failing member inside the value it was found in, and the operand the
/// check compared against, so a rejection is traceable to the contract clause
/// that caused it rather than only to the type that reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    constraint: String,
    keyword: String,
    path: String,
    operand: String,
    input: Option<String>,
}

impl ValidationError {
    /// Builds an error naming the constraint, the keyword, the member path and
    /// the operand.
    pub fn new(constraint: &str, keyword: &str, path: &str, operand: &str) -> Self {
        Self {
            constraint: constraint.to_owned(),
            keyword: keyword.to_owned(),
            path: path.to_owned(),
            operand: operand.to_owned(),
            input: None,
        }
    }

    /// Builds an error that also echoes the offending input, truncated at the
    /// declared bound.
    pub fn with_input(
        constraint: &str,
        keyword: &str,
        path: &str,
        operand: &str,
        input: &str,
    ) -> Self {
        let mut error = Self::new(constraint, keyword, path, operand);
        error.input = Some(truncate_echo(input));
        error
    }

    /// Prefixes the member path, so an error raised inside a member reports the
    /// path from the value the caller handed in.
    #[must_use]
    pub fn at(mut self, member: &str) -> Self {
        self.path = if self.path.is_empty() {
            member.to_owned()
        } else {
            format!("{}.{}", member, self.path)
        };
        self
    }

    /// The semantic identity of the constraint that rejected the value.
    pub fn constraint(&self) -> &str {
        &self.constraint
    }

    /// The constraint keyword.
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// The path of the failing member inside the value.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The operand the check compared against.
    pub fn operand(&self) -> &str {
        &self.operand
    }

    /// The echoed input, truncated at the declared bound.
    pub fn input(&self) -> Option<&str> {
        self.input.as_deref()
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} rejected {}: keyword {}, operand {}",
            self.constraint,
            if self.path.is_empty() {
                "the value"
            } else {
                self.path.as_str()
            },
            self.keyword,
            self.operand
        )?;
        match &self.input {
            Some(input) => write!(formatter, ", input {input}"),
            None => Ok(()),
        }
    }
}

impl std::error::Error for ValidationError {}

/// One diagnostic a generated `validate` reports.
///
/// The severity and the blocking disposition are carried rather than derived,
/// because a consumer that decides them itself would be a second registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    code: &'static str,
    severity: &'static str,
    owner: &'static str,
    blocking: bool,
    message: String,
}

impl Diagnostic {
    /// Builds a diagnostic from a registered code and a message.
    pub fn new(
        code: &'static str,
        severity: &'static str,
        owner: &'static str,
        blocking: bool,
        message: String,
    ) -> Self {
        Self {
            code,
            severity,
            owner,
            blocking,
            message,
        }
    }

    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// The severity the registry declares for the code.
    pub fn severity(&self) -> &'static str {
        self.severity
    }

    /// The owner the registry declares for the code.
    pub fn owner(&self) -> &'static str {
        self.owner
    }

    /// Whether the registry declares the code blocking.
    pub fn blocking(&self) -> bool {
        self.blocking
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A member that may carry a JSON `null` as a value of its own.
///
/// `Nullable` is the middle axis of the three-axis member composition, and it
/// is deliberately not `Option`: an absent member and a present `null` are two
/// states the contract distinguishes, and collapsing them onto one constructor
/// is what the composition exists to prevent.
#[derive(Clone, Debug, PartialEq)]
pub enum Nullable<T> {
    /// A present JSON `null`.
    Null,
    /// A present value.
    Value(T),
}

impl<T> Nullable<T> {
    /// The value, where one is present.
    pub fn value(&self) -> Option<&T> {
        match self {
            Nullable::Null => None,
            Nullable::Value(value) => Some(value),
        }
    }

    /// True where the member carried a JSON `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Nullable::Null)
    }
}

impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Nullable::Null => serializer.serialize_none(),
            Nullable::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            None => Nullable::Null,
            Some(value) => Nullable::Value(value),
        })
    }
}

/// Wraps whatever it is given in `Some`.
///
/// Serde invokes a `deserialize_with` only when the member is present, so a
/// present `null` reaches `Nullable`'s own `Deserialize` and becomes
/// `Some(Nullable::Null)`, while an absent member takes the `default` and
/// becomes `None`. Without this helper `Option`'s own `Deserialize` consumes
/// the `null` first and both states arrive as `None`.
pub fn present_or_absent<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// The source text of a JSON number.
///
/// A retained value's number is kept as text so that a round trip is a fact
/// about the value rather than about a parser's normalization. The text a
/// deserializer can hand back is limited by what the deserializer exposes:
/// serde's data model carries `i64`, `u64` and `f64`, so a number written
/// `1.0` arrives as the `f64` one and is rendered `1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberLexeme(String);

impl NumberLexeme {
    /// Builds a lexeme from source text.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The retained source text.
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl Serialize for NumberLexeme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Ok(value) = self.0.parse::<i64>() {
            return serializer.serialize_i64(value);
        }
        if let Ok(value) = self.0.parse::<u64>() {
            return serializer.serialize_u64(value);
        }
        match self.0.parse::<f64>() {
            Ok(value) => serializer.serialize_f64(value),
            Err(_) => Err(serde::ser::Error::custom(format!(
                "the retained number lexeme {} is not a JSON number",
                self.0
            ))),
        }
    }
}

/// The JSON value space, declared in this crate rather than borrowed.
///
/// The dynamic surface of the generated crate is exactly this type, together
/// with `UnknownMembers` and `Extension`. `Object` keeps its members in the
/// order they arrived and keeps a repeated name twice, because both are facts
/// about the retained value that a map would discard.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticValue {
    /// JSON `null`.
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON number, with its lexeme retained.
    Number(NumberLexeme),
    /// A JSON string.
    String(String),
    /// A JSON array.
    Array(Vec<SemanticValue>),
    /// A JSON object, in arrival order, with repeated names retained.
    Object(Vec<(String, SemanticValue)>),
}

impl Serialize for SemanticValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SemanticValue::Null => serializer.serialize_unit(),
            SemanticValue::Bool(value) => serializer.serialize_bool(*value),
            SemanticValue::Number(value) => value.serialize(serializer),
            SemanticValue::String(value) => serializer.serialize_str(value),
            SemanticValue::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(item)?;
                }
                sequence.end()
            }
            SemanticValue::Object(members) => {
                let mut map = serializer.serialize_map(Some(members.len()))?;
                for (name, value) in members {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
        }
    }
}

struct SemanticValueVisitor;

impl<'de> Visitor<'de> for SemanticValueVisitor {
    type Value = SemanticValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(SemanticValue::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(SemanticValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(SemanticValueVisitor)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(SemanticValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(SemanticValue::Number(NumberLexeme::new(value.to_string())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(SemanticValue::Number(NumberLexeme::new(value.to_string())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(SemanticValue::Number(NumberLexeme::new(format_number(
            value,
        ))))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(SemanticValue::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(SemanticValue::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = access.next_element()? {
            items.push(item);
        }
        Ok(SemanticValue::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut members = Vec::new();
        while let Some((name, value)) = access.next_entry::<String, SemanticValue>()? {
            members.push((name, value));
        }
        Ok(SemanticValue::Object(members))
    }
}

impl<'de> Deserialize<'de> for SemanticValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SemanticValueVisitor)
    }
}

/// Renders an `f64` the way ECMAScript's `Number::toString` does for the values
/// a JSON document can carry.
///
/// Rust's own `Display` for `f64` is already the shortest round-tripping form,
/// which is the rule ECMAScript states; the two differ only in how they choose
/// between fixed and exponential notation, and that choice is made here.
pub fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_nan() || value.is_infinite() {
        return "null".to_owned();
    }
    let magnitude = value.abs();
    if (1e-6..1e21).contains(&magnitude) {
        return format!("{value}");
    }
    let rendered = format!("{value:e}");
    match rendered.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => rendered,
    }
}

/// The members a record with a `preserve` or `surface` unknown policy retained.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnknownMembers(BTreeMap<String, SemanticValue>);

impl UnknownMembers {
    /// The retained members, ordered by name.
    pub fn members(&self) -> &BTreeMap<String, SemanticValue> {
        &self.0
    }

    /// True where nothing was retained.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of retained members.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// An unrecognised variant of a closed variant set, retained verbatim.
///
/// An `enum` and a `union` have a closed variant set, so their unknown policy
/// is a real obligation rather than an inert one: under `preserve` or
/// `surface` an unrecognised tag is kept here instead of failing
/// deserialization.
///
/// The two wire forms are kept apart because the round trip has to be
/// unchanged. An externally tagged variant reaches the wire either as a bare
/// tag string, which is how a variant with no payload is written, or as a
/// one-member object. Collapsing them onto one constructor would re-serialize
/// a value in the form it did not arrive in.
#[derive(Clone, Debug, PartialEq)]
pub enum UnknownVariant {
    /// The wire form was a bare tag string.
    Tag(String),
    /// The wire form was a one-member object: a tag and its payload.
    Tagged(String, SemanticValue),
}

impl UnknownVariant {
    /// The unrecognised tag.
    pub fn tag(&self) -> &str {
        match self {
            UnknownVariant::Tag(tag) => tag,
            UnknownVariant::Tagged(tag, _) => tag,
        }
    }

    /// The retained payload, where the wire form carried one.
    pub fn payload(&self) -> Option<&SemanticValue> {
        match self {
            UnknownVariant::Tag(_) => None,
            UnknownVariant::Tagged(_, payload) => Some(payload),
        }
    }
}

impl Serialize for UnknownVariant {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            UnknownVariant::Tag(tag) => serializer.serialize_str(tag),
            UnknownVariant::Tagged(tag, payload) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(tag, payload)?;
                map.end()
            }
        }
    }
}

/// A declared extension carried beside a contract node.
///
/// An extension is never folded into an unknown member: the two mean different
/// things and the contract carries both. This is the *value* type — the type a
/// consumer deserializes an extension into and asks for a verdict on — and it
/// is separate from the `ExtensionMeta` constant, which records what the
/// contract declared at generation time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extension {
    /// The extension's semantic identity.
    pub identity: String,
    /// The extension's version.
    pub version: String,
    /// Whether a consumer must understand the extension.
    pub required: bool,
    /// The capability the extension claims, where it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    /// The extension payload.
    pub payload: SemanticValue,
}

impl Extension {
    /// Builds an extension, checking the shape of its identity.
    ///
    /// The payload is deliberately unchecked: an extension payload is opaque to
    /// the contract that carries it, which is the whole point of the member.
    pub fn try_new(
        identity: String,
        version: String,
        required: bool,
        capability: Option<String>,
        payload: SemanticValue,
    ) -> Result<Self, ValidationError> {
        SemanticIdentity::try_new(identity.clone())?;
        Ok(Self {
            identity,
            version,
            required,
            capability,
            payload,
        })
    }

    /// Decides this extension against what the generated crate declares.
    ///
    /// `declared` is every extension identity the contract this crate was
    /// generated from carries. `admitted` is the set of capabilities the crate
    /// admits, which is **empty**, and empty is a stated decision rather than an
    /// omission: `consumer-policy.schema.json` is sealed and carries no
    /// capability member, and the published `rust` target contract declares no
    /// capability list either, so there is no published input from which a
    /// non-empty set could be read. That is GAP-007, owned by issue #9. A crate
    /// that claimed to admit a capability nobody published would be inventing
    /// the very rule the gap records as missing.
    ///
    /// Two rules follow, and they are the whole decision:
    ///
    /// - An extension whose `required` is false is preserved whatever its
    ///   identity, and carries no diagnostic. Forward compatibility is what the
    ///   member is for.
    /// - An extension whose `required` is true is rejected when its identity is
    ///   not one the contract declares, or when it names any capability at all,
    ///   because the admitted set is empty.
    ///
    /// A blocking diagnostic in the returned list is the rejection; an empty
    /// list is acceptance.
    pub fn decide(&self, declared: &[&str], admitted: &[&str]) -> Vec<Diagnostic> {
        if !self.required {
            return Vec::new();
        }
        let known = declared.contains(&self.identity.as_str());
        let capability_admitted = match &self.capability {
            None => true,
            Some(capability) => admitted.contains(&capability.as_str()),
        };
        if known && capability_admitted {
            return Vec::new();
        }
        let reason = match &self.capability {
            Some(capability) if !capability_admitted => format!(
                "it requires the capability {}, which this crate admits none of",
                truncate_echo(capability)
            ),
            _ => "this contract does not declare it".to_owned(),
        };
        vec![Diagnostic::new(
            UNKNOWN_REQUIRED_EXTENSION_CODE,
            UNKNOWN_REQUIRED_EXTENSION_SEVERITY,
            UNKNOWN_REQUIRED_EXTENSION_OWNER,
            true,
            format!(
                "the required extension {} is rejected because {}",
                truncate_echo(&self.identity),
                reason
            ),
        )]
    }
}

/// One instruction of a generated matcher program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatcherInst {
    /// Consumes one UTF-16 code unit equal to the operand.
    Char(u16),
    /// Consumes one code unit that is not a line terminator.
    Any,
    /// Consumes one code unit inside, or when negated outside, the ranges.
    Class {
        /// True where the class is negated.
        negated: bool,
        /// The inclusive code-unit ranges, sorted and merged.
        ranges: &'static [(u16, u16)],
    },
    /// Succeeds only at the start of the subject.
    Bol,
    /// Succeeds only at the end of the subject.
    Eol,
    /// Continues at the first target, and on failure at the second.
    Split(usize, usize),
    /// Continues at the target.
    Jump(usize),
    /// The subject matched.
    Match,
}

/// The declared step bound of the matcher.
///
/// The matcher backtracks, so a pathological pattern and subject pair can ask
/// for exponentially many steps. It returns `BoundExceeded` at this count
/// rather than running on, which is a refusal the caller can see rather than a
/// process that does not come back.
pub const MATCHER_STEP_BOUND: u64 = 1_000_000;

/// The four ECMAScript line terminators, which `.` does not match.
pub const LINE_TERMINATORS: [u16; 4] = [0x000a, 0x000d, 0x2028, 0x2029];

/// The outcome of running a matcher program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOutcome {
    /// The pattern matched somewhere in the subject.
    Matched,
    /// The pattern matched nowhere in the subject.
    Rejected,
    /// The declared step bound was reached before either was decided.
    BoundExceeded,
}

/// Runs a matcher program over a subject read as UTF-16 code units.
///
/// The basis is code units rather than scalar values because the dialect this
/// backend decides is ECMA-262 without the `u` flag, in which `.` matches one
/// code unit and an astral character is two.
pub fn run_matcher(program: &[MatcherInst], subject: &[u16]) -> MatchOutcome {
    let mut steps: u64 = 0;
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start in 0..=subject.len() {
        stack.clear();
        stack.push((0, start));
        while let Some((pc, at)) = stack.pop() {
            steps += 1;
            if steps > MATCHER_STEP_BOUND {
                return MatchOutcome::BoundExceeded;
            }
            match program[pc] {
                MatcherInst::Match => return MatchOutcome::Matched,
                MatcherInst::Char(unit) => {
                    if subject.get(at) == Some(&unit) {
                        stack.push((pc + 1, at + 1));
                    }
                }
                MatcherInst::Any => {
                    if let Some(unit) = subject.get(at) {
                        if !LINE_TERMINATORS.contains(unit) {
                            stack.push((pc + 1, at + 1));
                        }
                    }
                }
                MatcherInst::Class { negated, ranges } => {
                    if let Some(unit) = subject.get(at) {
                        let inside = ranges.iter().any(|(lo, hi)| unit >= lo && unit <= hi);
                        if inside != negated {
                            stack.push((pc + 1, at + 1));
                        }
                    }
                }
                MatcherInst::Bol => {
                    if at == 0 {
                        stack.push((pc + 1, at));
                    }
                }
                MatcherInst::Eol => {
                    if at == subject.len() {
                        stack.push((pc + 1, at));
                    }
                }
                MatcherInst::Split(first, second) => {
                    stack.push((second, at));
                    stack.push((first, at));
                }
                MatcherInst::Jump(target) => stack.push((target, at)),
            }
        }
    }
    MatchOutcome::Rejected
}

/// Runs a matcher program over a `&str`, encoding it to UTF-16 first.
pub fn matches_pattern(program: &[MatcherInst], subject: &str) -> MatchOutcome {
    let units: Vec<u16> = subject.encode_utf16().collect();
    run_matcher(program, &units)
}

/// A validated `ix://` semantic identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SemanticIdentity(String);

impl SemanticIdentity {
    /// Accepts exactly the strings the published `semanticIdentity` pattern
    /// accepts, decided by the generated matcher.
    pub fn try_new(value: String) -> Result<Self, ValidationError> {
        match matches_pattern(SEMANTIC_IDENTITY_PATTERN, &value) {
            MatchOutcome::Matched => Ok(Self(value)),
            MatchOutcome::Rejected => Err(ValidationError::with_input(
                SEMANTIC_IDENTITY_CONSTRAINT,
                "pattern",
                "",
                SEMANTIC_IDENTITY_SOURCE,
                &value,
            )),
            MatchOutcome::BoundExceeded => Err(ValidationError::with_input(
                SEMANTIC_IDENTITY_CONSTRAINT,
                "pattern",
                "",
                "the matcher step bound",
                &value,
            )),
        }
    }

    /// The identity text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The identity text, consuming the wrapper.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for SemanticIdentity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for SemanticIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The semantic identity of the published locus-path pattern, used as the
/// constraint identity of a `SourceLocusPath` rejection.
pub const SOURCE_LOCUS_PATH_CONSTRAINT: &str =
    "ix://agent-ix/filament-core-data/schema/common/sourceLocus/path";

/// A validated `sourceLocus.path`.
///
/// `try_new` decides the **published** language: exactly the strings the
/// pattern in `common.schema.json` accepts under ECMA-262. That language is not
/// the language a reader of the pattern expects, because each of its guards is
/// a lookahead over `.`, and `.` stops at the first line terminator: the
/// published language admits `a\n../../etc/passwd`. `is_traversal_free` decides
/// the intended language instead, and the two are separately named rather than
/// one silently standing for the other. The divergence is GAP-002 and
/// filament-core-data issue #56.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SourceLocusPath(String);

impl SourceLocusPath {
    /// Accepts exactly the strings the published pattern accepts.
    ///
    /// The five clauses, decided over UTF-16 code units, are: the subject is
    /// non-empty and carries no U+0000; it does not begin with `/`; it does not
    /// begin with an ASCII letter followed by `:`; its head — the prefix before
    /// the first line terminator, or the whole subject when it carries none —
    /// carries no `\`; and there is no index `i` such that the units at `i` and
    /// `i + 1` both lie in the head and are both `.`, `i` is zero or the unit
    /// before it is `/`, and either the unit at `i + 2` is `/` or `i + 2` is the
    /// end of the subject.
    pub fn try_new(value: String) -> Result<Self, ValidationError> {
        let units: Vec<u16> = value.encode_utf16().collect();
        let reject = |clause: &str| {
            Err(ValidationError::with_input(
                SOURCE_LOCUS_PATH_CONSTRAINT,
                "pattern",
                "",
                clause,
                &value,
            ))
        };
        if units.is_empty() {
            return reject("a non-empty path");
        }
        if units.contains(&0) {
            return reject("a path carrying no U+0000");
        }
        if units[0] == b'/' as u16 {
            return reject("a path not beginning with `/`");
        }
        if units.len() >= 2 && is_ascii_letter(units[0]) && units[1] == b':' as u16 {
            return reject("a path not beginning with a drive letter");
        }
        let head = units
            .iter()
            .position(|unit| LINE_TERMINATORS.contains(unit))
            .unwrap_or(units.len());
        if units[..head].contains(&(b'\\' as u16)) {
            return reject("a path whose head carries no `\\`");
        }
        let dot = b'.' as u16;
        let slash = b'/' as u16;
        for index in 0..head {
            if index + 1 >= head {
                break;
            }
            if units[index] != dot || units[index + 1] != dot {
                continue;
            }
            if index != 0 && units[index - 1] != slash {
                continue;
            }
            let ends_here = index + 2 == units.len();
            let followed_by_slash = units.get(index + 2) == Some(&slash);
            if ends_here || followed_by_slash {
                return reject("a path carrying no parent-directory segment in its head");
            }
        }
        Ok(Self(value))
    }

    /// Decides the *intended* language: no `..` segment anywhere in the value,
    /// no `\` anywhere in it, and no line terminator anywhere in it.
    ///
    /// A value used to address the filesystem must satisfy this predicate in
    /// addition to the published one. It is never substituted for the published
    /// language, because the two decide different sets and a member validated
    /// against the wrong one is validated against nothing.
    pub fn is_traversal_free(&self) -> bool {
        let units: Vec<u16> = self.0.encode_utf16().collect();
        if units.iter().any(|unit| LINE_TERMINATORS.contains(unit)) {
            return false;
        }
        if units.contains(&(b'\\' as u16)) {
            return false;
        }
        !self.0.split('/').any(|segment| segment == "..")
    }

    /// The path text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path text, consuming the wrapper.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for SourceLocusPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for SourceLocusPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_ascii_letter(unit: u16) -> bool {
    (unit >= b'A' as u16 && unit <= b'Z' as u16) || (unit >= b'a' as u16 && unit <= b'z' as u16)
}

/// The semantic identity used as the constraint identity of a kernel scalar
/// newtype's own format rejection.
pub const KERNEL_SCALAR_CONSTRAINT: &str =
    "ix://agent-ix/filament-core-data/schema/semantic-ir/kernel-scalar";

macro_rules! validated_scalar {
    ($name:ident, $keyword:literal, $shape:literal, $check:path) => {
        /// A kernel scalar carried as a validated newtype over its JSON string.
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Accepts exactly the strings the declared shape admits.
            pub fn try_new(value: String) -> Result<Self, ValidationError> {
                if $check(&value) {
                    Ok(Self(value))
                } else {
                    Err(ValidationError::with_input(
                        KERNEL_SCALAR_CONSTRAINT,
                        $keyword,
                        "",
                        $shape,
                        &value,
                    ))
                }
            }

            /// The value's text.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// The value's text, consuming the wrapper.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::try_new(String::deserialize(deserializer)?).map_err(de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

validated_scalar!(Date, "format", "an RFC 3339 full-date", is_rfc3339_date);
validated_scalar!(
    DateTime,
    "format",
    "an RFC 3339 date-time",
    is_rfc3339_date_time
);
validated_scalar!(
    Duration,
    "format",
    "an ISO 8601 duration",
    is_iso8601_duration
);
validated_scalar!(Uuid, "format", "an 8-4-4-4-12 UUID", is_uuid);

/// True where the civil date exists in the proleptic Gregorian calendar.
fn is_valid_civil(year: i64, month: i64, day: i64) -> bool {
    if !(1..=12).contains(&month) || day < 1 {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    day <= lengths[(month - 1) as usize]
}

/// Days from 1970-01-01 to a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

/// An RFC 3339 full-date as nanoseconds since the epoch.
pub fn date_instant(text: &str) -> Option<i128> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = digits(&text[0..4])?;
    let month = digits(&text[5..7])?;
    let day = digits(&text[8..10])?;
    if !is_valid_civil(year, month, day) {
        return None;
    }
    Some(i128::from(days_from_civil(year, month, day)) * 86_400_000_000_000)
}

fn is_rfc3339_date(text: &str) -> bool {
    date_instant(text).is_some()
}

/// An RFC 3339 date-time as nanoseconds since the epoch.
///
/// The offset is applied, so two values written with different offsets compare
/// by the instant they name and not by their text.
pub fn date_time_instant(text: &str) -> Option<i128> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    if bytes[10] != b'T' && bytes[10] != b't' {
        return None;
    }
    let date = date_instant(&text[0..10])?;
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let hour = digits(&text[11..13])?;
    let minute = digits(&text[14..16])?;
    let second = digits(&text[17..19])?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let mut rest = &text[19..];
    let mut nanos: i128 = 0;
    if let Some(stripped) = rest.strip_prefix('.') {
        let end = stripped
            .bytes()
            .position(|byte| !byte.is_ascii_digit())
            .unwrap_or(stripped.len());
        if end == 0 {
            return None;
        }
        let fraction = &stripped[..end.min(9)];
        let mut scaled = digits(fraction)?;
        for _ in fraction.len()..9 {
            scaled *= 10;
        }
        nanos = i128::from(scaled);
        rest = &stripped[end..];
    }
    let offset: i128 = if rest == "Z" || rest == "z" {
        0
    } else {
        let sign = match rest.as_bytes().first() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return None,
        };
        if rest.len() != 6 || rest.as_bytes()[3] != b':' {
            return None;
        }
        let offset_hour = digits(&rest[1..3])?;
        let offset_minute = digits(&rest[4..6])?;
        if offset_hour > 23 || offset_minute > 59 {
            return None;
        }
        i128::from(sign)
            * (i128::from(offset_hour) * 3_600 + i128::from(offset_minute) * 60)
            * 1_000_000_000
    };
    Some(
        date + i128::from(hour) * 3_600_000_000_000
            + i128::from(minute) * 60_000_000_000
            + i128::from(second) * 1_000_000_000
            + nanos
            - offset,
    )
}

fn is_rfc3339_date_time(text: &str) -> bool {
    date_time_instant(text).is_some()
}

fn is_iso8601_duration(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((left, right)) => (left, Some(right)),
        None => (rest, None),
    };
    let mut components = 0usize;
    if !consume_components(date_part, b"YMWD", &mut components) {
        return false;
    }
    if let Some(time) = time_part {
        if time.is_empty() {
            return false;
        }
        if !consume_components(time, b"HMS", &mut components) {
            return false;
        }
    }
    components > 0
}

fn consume_components(text: &str, designators: &[u8], components: &mut usize) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let mut next_designator = 0usize;
    while at < bytes.len() {
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == start {
            return false;
        }
        if at < bytes.len() && bytes[at] == b'.' {
            at += 1;
            let fraction = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at == fraction {
                return false;
            }
        }
        if at >= bytes.len() {
            return false;
        }
        let designator = bytes[at];
        let Some(position) = designators[next_designator..]
            .iter()
            .position(|candidate| *candidate == designator)
        else {
            return false;
        };
        next_designator += position + 1;
        *components += 1;
        at += 1;
    }
    true
}

fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        let expected_dash = matches!(index, 8 | 13 | 18 | 23);
        if expected_dash {
            if *byte != b'-' {
                return false;
            }
            continue;
        }
        if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// Compares two instants, for a `date` or `datetime` bound.
pub fn compare_instant(left: i128, right: i128) -> Ordering {
    left.cmp(&right)
}

/// The semantic identity of the published `semanticIdentity` pattern.
pub const SEMANTIC_IDENTITY_CONSTRAINT: &str =
    "ix://agent-ix/filament-core-data/schema/common/semanticIdentity";

/// The published `semanticIdentity` pattern, as text, for a rejection message.
pub const SEMANTIC_IDENTITY_SOURCE: &str =
    "^ix://[a-z0-9][a-z0-9._-]*/[A-Za-z0-9][A-Za-z0-9._~:/-]*$";

/// The published `semanticIdentity` pattern, lowered to a matcher program.
pub const SEMANTIC_IDENTITY_PATTERN: &[MatcherInst] = &[
    MatcherInst::Bol,
    MatcherInst::Char(105),
    MatcherInst::Char(120),
    MatcherInst::Char(58),
    MatcherInst::Char(47),
    MatcherInst::Char(47),
    MatcherInst::Class {
        negated: false,
        ranges: &[(48, 57), (97, 122)],
    },
    MatcherInst::Split(8, 10),
    MatcherInst::Class {
        negated: false,
        ranges: &[(45, 46), (48, 57), (95, 95), (97, 122)],
    },
    MatcherInst::Jump(7),
    MatcherInst::Char(47),
    MatcherInst::Class {
        negated: false,
        ranges: &[(48, 57), (65, 90), (97, 122)],
    },
    MatcherInst::Split(13, 15),
    MatcherInst::Class {
        negated: false,
        ranges: &[(45, 58), (65, 90), (95, 95), (97, 122), (126, 126)],
    },
    MatcherInst::Jump(12),
    MatcherInst::Eol,
    MatcherInst::Match,
];

/// The code a rejected required extension is reported under.
///
/// It carries the published reader spelling rather than a generator one,
/// because `conformance/diagnostic-codes.json` already names this defect and
/// a second spelling for one defect is two registries that have to agree.
pub const UNKNOWN_REQUIRED_EXTENSION_CODE: &str = "agent-ix.semantic-ir.UNKNOWN_REQUIRED_EXTENSION";

/// The severity the registry declares for that code.
pub const UNKNOWN_REQUIRED_EXTENSION_SEVERITY: &str = "error";

/// The owner the registry declares for that code.
pub const UNKNOWN_REQUIRED_EXTENSION_OWNER: &str = "ix://agent-ix/filament-core-data/semantic-ir";

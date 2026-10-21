// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Human-facing onboarding report derived from installed schema and module data.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;

use crate::process_host::{self, ProcessLimits};

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT: usize = 16 * 1024 * 1024;
const MAX_FINDINGS: usize = 10;
const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

fn read_text_bounded(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_FILE_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    File::open(path)
        .ok()?
        .take(limit)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_FILE_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// JSON object member order affects the published checklist arrays. Keep it
/// alongside `serde_json::Value`, whose map uses sorted keys in this crate.
enum OrderedNode {
    Leaf,
    Sequence(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl<'de> Deserialize<'de> for OrderedNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NodeVisitor;
        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = OrderedNode;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_string<E: serde::de::Error>(self, _: String) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(OrderedNode::Leaf)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(OrderedNode::Sequence(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    values.push(entry);
                }
                Ok(OrderedNode::Object(values))
            }
        }
        deserializer.deserialize_any(NodeVisitor)
    }
}

type ObjectOrders = BTreeMap<*const Value, Vec<String>>;

impl OrderedNode {
    fn collect_orders(&self, value: &Value, orders: &mut ObjectOrders) {
        match (self, value) {
            (Self::Object(entries), Value::Object(map)) => {
                let keys = entries
                    .iter()
                    .map(|(key, _)| key.clone())
                    .collect::<Vec<_>>();
                orders.insert(std::ptr::from_ref(value), keys);
                for (key, child) in entries {
                    if let Some(value) = map.get(key) {
                        child.collect_orders(value, orders);
                    }
                }
            }
            (Self::Sequence(entries), Value::Array(values)) => {
                for (entry, value) in entries.iter().zip(values) {
                    entry.collect_orders(value, orders);
                }
            }
            _ => {}
        }
    }
}

fn ordered_entries<'a>(value: &'a Value, orders: &ObjectOrders) -> Vec<(&'a String, &'a Value)> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    let order = orders.get(&std::ptr::from_ref(value));
    order.map_or_else(
        || map.iter().collect(),
        |keys| {
            keys.iter()
                .filter_map(|key| map.get_key_value(key))
                .collect()
        },
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    target_repo: String,
    module_root: Option<String>,
    installed_module_version: Option<String>,
    relationship: Relationship,
    spec_assurance: SpecAssurance,
    working_examples: WorkingExamples,
    artifact_checklists: BTreeMap<String, Checklist>,
    #[serde(skip)]
    artifact_order: Vec<String>,
    measurement_record_checklist: MeasurementChecklist,
    next_steps: Vec<&'static str>,
}

#[derive(Serialize)]
struct Relationship {
    module: &'static str,
    instances: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpecAssurance {
    path: String,
    exists: bool,
    existing_artifacts: Vec<Artifact>,
    note: Option<&'static str>,
}

#[derive(Clone, Serialize)]
struct Artifact {
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkingExamples {
    measurement_collections: MeasurementCollections,
    real_artifact_instances: LinkedExample,
    skeletons: Vec<String>,
}

#[derive(Serialize)]
struct MeasurementCollections {
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'static str>,
}

#[derive(Serialize)]
struct LinkedExample {
    note: &'static str,
    url: &'static str,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Checklist {
    schema_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unreadable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conditional_required: Option<Vec<ConditionalRequired>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exactly_one_of: Option<Vec<ExactlyOneOf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conditional_constraints: Option<Vec<ConditionalConstraint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enums: Option<BTreeMap<String, Vec<Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arrays: Option<BTreeMap<String, ListShape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<Vec<String>>,
    #[serde(skip)]
    enum_order: Vec<String>,
    #[serde(skip)]
    array_order: Vec<String>,
}

impl Checklist {
    fn add_enum(&mut self, path: &str, values: &[Value]) {
        let enums = self.enums.as_mut().expect("initialized checklist");
        if !enums.contains_key(path) {
            self.enum_order.push(path.to_owned());
        }
        enums.insert(path.to_owned(), values.to_vec());
    }

    fn add_array(&mut self, path: &str, shape: ListShape) {
        let arrays = self.arrays.as_mut().expect("initialized checklist");
        if !arrays.contains_key(path) {
            self.array_order.push(path.to_owned());
        }
        arrays.insert(path.to_owned(), shape);
    }
}

#[derive(Serialize)]
struct ConditionalRequired {
    when: String,
    required: Vec<String>,
}
#[derive(Serialize)]
struct ExactlyOneOf {
    when: String,
    fields: Vec<String>,
}
#[derive(Serialize)]
struct ConditionalConstraint {
    when: String,
    constraint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListShape {
    min_items: u64,
    unique_items: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_description: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeasurementChecklist {
    collection: CollectionChecklist,
    observation: Vec<String>,
    source_of_truth: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollectionChecklist {
    always: Vec<String>,
    schema_version2_verification_stack: Vec<String>,
    new_collection_intake_only: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    target_repo: String,
    installed_module_version: Option<String>,
    artifacts: Vec<ValidatedArtifact>,
    toolchain: Toolchain,
}

#[derive(Serialize)]
struct ValidatedArtifact {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    validation: Validation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Validation {
    status: &'static str,
    findings: Vec<String>,
    more_findings: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Toolchain {
    quire: Option<String>,
    quoin: Option<String>,
    quoin_measurement_verify: Option<bool>,
}

const USAGE: &str = "Usage: engineering-assurance onboard-report --module-root <path> [--repo <path>] [--json] [--summary]\n\n  --module-root <path>  installed engineering-assurance module root\n  --repo <path>         consuming repository root (default: current directory)\n  --json                emit the report as JSON\n  --summary             compact validation and toolchain report\n  -h, --help            print this message\n";

pub(crate) fn run_argv() -> ExitCode {
    let args = std::env::args_os().skip(2).collect::<Vec<_>>();
    if args
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let mut repo: Option<PathBuf> = None;
    let mut module_root: Option<PathBuf> = None;
    let mut json = false;
    let mut summary = false;
    let mut index = 0;
    while let Some(argument) = args.get(index) {
        match argument.to_str() {
            Some("--json") => json = true,
            Some("--summary") => summary = true,
            Some("--repo" | "--module-root") => {
                let value = args
                    .get(index + 1)
                    .filter(|value| !value.to_string_lossy().starts_with('-'));
                let Some(value) = value else {
                    return usage_error(&format!(
                        "{} requires a path argument",
                        argument.to_string_lossy()
                    ));
                };
                let slot = if argument == "--repo" {
                    &mut repo
                } else {
                    &mut module_root
                };
                if slot.is_some() {
                    return usage_error(&format!(
                        "{} may be given only once",
                        argument.to_string_lossy()
                    ));
                }
                *slot = Some(PathBuf::from(value));
                index += 1;
            }
            _ => return usage_error(&format!("unknown argument: {}", argument.to_string_lossy())),
        }
        index += 1;
    }
    let Some(module_root) = module_root else {
        return usage_error("--module-root requires a path argument");
    };
    run_with_args(
        repo.unwrap_or_else(|| PathBuf::from(".")),
        module_root,
        json,
        summary,
    )
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n\n{USAGE}");
    ExitCode::from(2)
}

fn run_with_args(repo: PathBuf, module_root: PathBuf, json: bool, summary: bool) -> ExitCode {
    let repo = if repo.is_absolute() {
        repo
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(repo)
    };
    let repo = fs::canonicalize(&repo).unwrap_or(repo);
    let module_root = fs::canonicalize(&module_root).unwrap_or(module_root);
    let result = if summary {
        let summary = summarize(&repo, &module_root);
        if json {
            print_json(&summary);
        } else {
            print_summary(&summary);
        }
        if summary
            .artifacts
            .iter()
            .any(|a| a.validation.status == "invalid")
        {
            1
        } else if summary
            .artifacts
            .iter()
            .any(|a| a.validation.status == "unavailable")
        {
            eprintln!(
                "validation unavailable: quire did not run to completion, so nothing was validated"
            );
            3
        } else {
            0
        }
    } else {
        let report = report(&repo, &module_root);
        if json {
            print_json(&report);
        } else {
            print_report(&report);
        }
        0
    };
    ExitCode::from(result)
}

fn print_json(value: &impl Serialize) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("onboarding report serialization failed: {error}"),
    }
}

fn module_version(module_root: &Path) -> Option<String> {
    read_text_bounded(&module_root.join("manifest.yaml"))?
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("version:")
                .map(|v| v.trim().trim_matches('"').to_owned())
        })
}

fn artifacts(repo: &Path) -> (bool, Vec<Artifact>) {
    let directory = repo.join("spec/assurance");
    let exists = directory.exists();
    if fs::canonicalize(&directory).is_ok_and(|resolved| !resolved.starts_with(repo)) {
        return (exists, Vec::new());
    }
    let mut entries = fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if Path::new(&name).extension() != Some(OsStr::new("md")) {
                return None;
            }
            let confined_path = fs::canonicalize(entry.path())
                .ok()
                .filter(|path| path.starts_with(repo));
            let kind = match confined_path.as_deref().and_then(read_text_bounded) {
                None => "unreadable".to_owned(),
                Some(text) if !text.starts_with("---") => "no-frontmatter".to_owned(),
                Some(text) => text
                    .lines()
                    .skip(1)
                    .take_while(|line| *line != "---")
                    .find_map(|line| {
                        line.strip_prefix("type:")
                            .and_then(|v| v.split_whitespace().next())
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "unrecognized".to_owned()),
            };
            Some(Artifact { name, kind })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    (exists, entries)
}

fn run_command(
    repo: &Path,
    command: &str,
    args: &[&OsStr],
) -> Option<process_host::CompletedProcess> {
    process_host::run_configured(
        OsStr::new(command),
        args,
        Some(repo),
        &[],
        &[],
        ProcessLimits {
            timeout: TIMEOUT,
            max_output_bytes: MAX_OUTPUT,
        },
    )
    .ok()
}

fn summarize(repo: &Path, module_root: &Path) -> Summary {
    let (_, found) = artifacts(repo);
    let artifacts = found
        .into_iter()
        .map(|artifact| {
            let relative = Path::new("spec/assurance").join(&artifact.name);
            let args = [
                OsStr::new("validate"),
                OsStr::new("--scope"),
                repo.as_os_str(),
                relative.as_os_str(),
            ];
            let validation = if let Some(output) = run_command(repo, "quire", &args) {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let marker = format!("{}: [", artifact.name);
                let findings = stderr
                    .lines()
                    .filter_map(|line| {
                        line.find(&marker)
                            .map(|i| line[i + artifact.name.len() + 2..].to_owned())
                    })
                    .collect::<Vec<_>>();
                Validation {
                    status: if output.status.success() {
                        "valid"
                    } else {
                        "invalid"
                    },
                    more_findings: findings.len().saturating_sub(MAX_FINDINGS),
                    findings: findings.into_iter().take(MAX_FINDINGS).collect(),
                }
            } else {
                Validation {
                    status: "unavailable",
                    findings: vec![],
                    more_findings: 0,
                }
            };
            ValidatedArtifact {
                name: artifact.name,
                kind: artifact.kind,
                validation,
            }
        })
        .collect();
    let quire = run_command(repo, "quire", &[OsStr::new("provenance")])
        .filter(|v| v.status.success())
        .and_then(|v| serde_json::from_slice::<Value>(&v.stdout).ok())
        .and_then(|v| {
            v.pointer("/cli/version")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let quoin = run_command(repo, "quoin", &[OsStr::new("--version")])
        .filter(|v| v.status.success())
        .and_then(|v| {
            let stdout = String::from_utf8_lossy(&v.stdout);
            stdout
                .split_whitespace()
                .find(|part| {
                    part.chars().next().is_some_and(|c| c.is_ascii_digit())
                        && part.matches('.').count() >= 2
                })
                .map(str::to_owned)
        });
    let quoin_measurement_verify = run_command(
        repo,
        "quoin",
        &[OsStr::new("measurement"), OsStr::new("--help")],
    )
    .filter(|v| v.status.success())
    .map(|v| {
        String::from_utf8_lossy(&v.stdout)
            .lines()
            .any(|line| line.trim_start().starts_with("measurement verify"))
    });
    Summary {
        target_repo: repo.display().to_string(),
        installed_module_version: module_version(module_root),
        artifacts,
        toolchain: Toolchain {
            quire,
            quoin,
            quoin_measurement_verify,
        },
    }
}

fn print_summary(summary: &Summary) {
    println!(
        "engineering-assurance module in this checkout: {}",
        summary
            .installed_module_version
            .as_deref()
            .unwrap_or("not found")
    );
    println!(
        "quire: {}",
        summary.toolchain.quire.as_deref().unwrap_or("not found")
    );
    println!(
        "quoin: {}",
        summary.toolchain.quoin.as_deref().unwrap_or("not found")
    );
    println!(
        "quoin measurement verify: {}",
        summary
            .toolchain
            .quoin_measurement_verify
            .map_or("unknown", |v| if v { "available" } else { "not available" })
    );
    println!("spec/assurance artifacts: {}", summary.artifacts.len());
    for artifact in &summary.artifacts {
        println!(
            "  {:11} {} ({})",
            artifact.validation.status, artifact.name, artifact.kind
        );
        for finding in &artifact.validation.findings {
            println!("      {finding}");
        }
        if artifact.validation.more_findings > 0 {
            println!("      +{} more", artifact.validation.more_findings);
        }
    }
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn value_text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

fn describe_condition(condition: &Value, prefix: &str, orders: &ObjectOrders) -> String {
    let clauses = condition
        .get("properties")
        .into_iter()
        .flat_map(|value| ordered_entries(value, orders))
        .map(|(name, def)| {
            let path = format!("{prefix}{name}");
            if let Some(value) = def.get("const") {
                return format!("{path} = {}", value_text(value));
            }
            if let Some(values) = def.get("enum").and_then(Value::as_array) {
                return format!(
                    "{path} is one of {}",
                    values.iter().map(value_text).collect::<Vec<_>>().join(", ")
                );
            }
            if let Some(value) = def.pointer("/not/const") {
                return format!("{path} is not {}", value_text(value));
            }
            if let Some(contains) = def.get("contains") {
                return format!(
                    "{path} has an item where {}",
                    describe_condition(contains, "", orders)
                );
            }
            if def.get("properties").is_some() {
                return describe_condition(def, &format!("{path}."), orders);
            }
            format!("{path} is present")
        })
        .collect::<Vec<_>>();
    if !clauses.is_empty() {
        clauses.join(" and ")
    } else if let Some(required) = condition
        .get("required")
        .and_then(Value::as_array)
        .filter(|required| !required.is_empty())
    {
        required
            .iter()
            .map(|name| format!("{prefix}{} is present", value_text(name)))
            .collect::<Vec<_>>()
            .join(" and ")
    } else {
        "(unrecognized condition)".to_owned()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one JSON Schema constraint is described as an ordered set of clauses"
)]
fn describe_constraint(
    def: &Value,
    prefix: &str,
    path: &str,
    orders: &ObjectOrders,
    skip_required: bool,
) -> Option<Vec<String>> {
    const KEYS: &[&str] = &[
        "$ref",
        "const",
        "enum",
        "not",
        "required",
        "properties",
        "anyOf",
        "type",
    ];
    if def == &Value::Bool(false) {
        return Some(vec![format!("{path} must be absent")]);
    }
    let map = def.as_object()?;
    if map.keys().any(|key| !KEYS.contains(&key.as_str())) {
        return None;
    }
    let mut clauses = Vec::new();
    if let Some(reference) = def.get("$ref") {
        let name = reference.as_str()?.strip_prefix("#/$defs/")?;
        clauses.push(format!("{path} must match {name}"));
    }
    if let Some(value) = def.get("const") {
        clauses.push(format!("{path} must be {}", value_text(value)));
    }
    if let Some(values) = def.get("enum").and_then(Value::as_array) {
        clauses.push(format!(
            "{path} must be one of {}",
            values.iter().map(value_text).collect::<Vec<_>>().join(", ")
        ));
    }
    if let Some(not) = def.get("not") {
        let object = not.as_object()?;
        if object.len() != 1 {
            return None;
        }
        if let Some(value) = not.get("const") {
            clauses.push(format!("{path} must not be {}", value_text(value)));
        } else if let Some(required) = not.get("required").and_then(Value::as_array) {
            if required.is_empty() {
                return None;
            }
            let fields = names(&Value::Array(required.clone()))
                .iter()
                .map(|name| format!("{prefix}{name}"))
                .collect::<Vec<_>>();
            if fields.len() == 1 {
                clauses.push(format!("{} must be absent", fields[0]));
            } else {
                clauses.push(format!("{} must not appear together", fields.join(" and ")));
            }
        } else {
            let branches = not.get("anyOf").and_then(Value::as_array)?;
            if branches.is_empty() {
                return None;
            }
            let fields = branches
                .iter()
                .map(|branch| {
                    if branch.as_object()?.len() != 1 {
                        return None;
                    }
                    let required = branch.get("required")?.as_array()?;
                    if required.len() != 1 {
                        return None;
                    }
                    Some(format!("{prefix}{}", required[0].as_str()?))
                })
                .collect::<Option<Vec<_>>>()?;
            clauses.push(format!("{} must be absent", fields.join(", ")));
        }
    }
    if !skip_required
        && let Some(required) = def.get("required").and_then(Value::as_array)
        && !required.is_empty()
    {
        clauses.push(format!(
            "{} required",
            names(&Value::Array(required.clone()))
                .iter()
                .map(|name| format!("{prefix}{name}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(properties) = def.get("properties") {
        for (name, child) in ordered_entries(properties, orders) {
            let child_path = format!("{prefix}{name}");
            clauses.extend(describe_constraint(
                child,
                &format!("{child_path}."),
                &child_path,
                orders,
                false,
            )?);
        }
    }
    if let Some(branches) = def.get("anyOf").and_then(Value::as_array) {
        if branches.is_empty() {
            return None;
        }
        let branches = branches
            .iter()
            .map(|branch| {
                let parts = describe_constraint(branch, prefix, path, orders, false)?;
                if parts.is_empty() {
                    None
                } else {
                    Some(format!("({})", parts.join(" and ")))
                }
            })
            .collect::<Option<Vec<_>>>()?;
        clauses.push(format!("either {}", branches.join(" or ")));
    }
    Some(clauses)
}

#[allow(
    clippy::too_many_lines,
    reason = "schema conditional keywords are handled together so unsupported shapes warn"
)]
fn read_all_of(
    schema: &Value,
    prefix: &str,
    present: &str,
    checklist: &mut Checklist,
    orders: &ObjectOrders,
) {
    let required = checklist
        .conditional_required
        .as_mut()
        .expect("initialized checklist");
    if schema.get("dependentSchemas").is_some() {
        checklist
            .warnings
            .as_mut()
            .expect("initialized checklist")
            .push("schema uses `dependentSchemas`, which this checklist does not read".to_owned());
    }
    if let Some(entries) = schema.get("dependentRequired") {
        for (name, dependency) in ordered_entries(entries, orders) {
            if dependency.is_array() {
                required.push(ConditionalRequired {
                    when: format!("{prefix}{name} is present"),
                    required: names(dependency)
                        .iter()
                        .map(|name| format!("{prefix}{name}"))
                        .collect(),
                });
            } else {
                checklist.warnings.as_mut().expect("initialized checklist").push("schema uses a non-array `dependentRequired` entry, which this checklist does not read".to_owned());
            }
        }
    }
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let fields = branches
            .iter()
            .map(|branch| {
                if branch.as_object()?.len() != 1 {
                    return None;
                }
                let fields = branch.get("required")?.as_array()?;
                if fields.len() != 1 {
                    return None;
                }
                fields[0].as_str().map(str::to_owned)
            })
            .collect::<Option<Vec<_>>>();
        if let Some(fields) = fields.filter(|fields| !fields.is_empty()) {
            checklist
                .exactly_one_of
                .as_mut()
                .expect("initialized checklist")
                .push(ExactlyOneOf {
                    when: if present.is_empty() {
                        "always".to_owned()
                    } else {
                        present.to_owned()
                    },
                    fields: fields
                        .iter()
                        .map(|field| format!("{prefix}{field}"))
                        .collect(),
                });
        } else {
            checklist.warnings.as_mut().expect("initialized checklist").push("schema uses a `oneOf` that is empty or not one `required` field per branch, which this checklist does not read".to_owned());
        }
    }
    if schema.get("anyOf").is_some() {
        checklist
            .warnings
            .as_mut()
            .expect("initialized checklist")
            .push("schema uses a top-level `anyOf`, which this checklist does not read".to_owned());
    }
    if schema.get("if").is_some() || schema.get("then").is_some() {
        checklist.warnings.as_mut().expect("initialized checklist").push("schema uses a bare `if`/`then` not wrapped in `allOf`, which this checklist does not read".to_owned());
    }
    for branch in schema
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let condition = branch.get("if").unwrap_or(&Value::Null);
        if ["$ref", "anyOf", "oneOf", "allOf"]
            .iter()
            .any(|key| condition.get(key).is_some())
        {
            checklist.warnings.as_mut().expect("initialized checklist").push("an `allOf` branch's `if` uses $ref/anyOf/oneOf/allOf, which this checklist cannot describe".to_owned());
            continue;
        }
        let then = branch.get("then").unwrap_or(&Value::Null);
        if then.get("allOf").is_some() {
            checklist.warnings.as_mut().expect("initialized checklist").push("an `allOf` branch's `then` nests another `allOf`, which this checklist does not read".to_owned());
            continue;
        }
        let when = describe_condition(condition, prefix, orders);
        if let Some(fields) = then
            .get("required")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
        {
            checklist
                .conditional_required
                .as_mut()
                .expect("initialized checklist")
                .push(ConditionalRequired {
                    when: when.clone(),
                    required: names(&Value::Array(fields.clone()))
                        .iter()
                        .map(|name| format!("{prefix}{name}"))
                        .collect(),
                });
        }
        if then
            .as_object()
            .is_some_and(|map| map.keys().any(|key| key != "required"))
        {
            match describe_constraint(then, prefix, prefix.trim_end_matches('.'), orders, true) {
                    Some(clauses) if !clauses.is_empty() => checklist.conditional_constraints.as_mut().expect("initialized checklist").push(ConditionalConstraint { when: when.clone(), constraint: clauses.join(" and ") }),
                    _ => checklist.warnings.as_mut().expect("initialized checklist").push("an `allOf` branch's `then` constrains something this checklist cannot describe".to_owned()),
                }
        }
        if let Some(otherwise) = branch.get("else") {
            match describe_constraint(otherwise, prefix, prefix.trim_end_matches('.'), orders, false) {
                Some(clauses) if !clauses.is_empty() => checklist.conditional_constraints.as_mut().expect("initialized checklist").push(ConditionalConstraint { when: format!("otherwise ({when} does not hold)"), constraint: clauses.join(" and ") }),
                _ => checklist.warnings.as_mut().expect("initialized checklist").push("an `allOf` branch's `else` constrains something this checklist cannot describe".to_owned()),
            }
        }
    }
}

fn local_reference<'a>(schema: &'a Value, def: &'a Value) -> &'a Value {
    let reference = def
        .get("$ref")
        .or_else(|| def.pointer("/items/$ref"))
        .and_then(Value::as_str);
    reference
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .and_then(|name| schema.pointer(&format!("/$defs/{name}")))
        .unwrap_or(def)
}

fn current_reference<'a>(schema: &'a Value, name: &str, def: &'a Value) -> &'a Value {
    if def.get("properties").is_some() || def.get("$ref").is_some() {
        return def;
    }
    schema
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|branch| {
            branch
                .pointer(&format!("/else/properties/{name}/$ref"))
                .and_then(Value::as_str)
        })
        .find_map(|reference| {
            reference
                .strip_prefix("#/$defs/")
                .and_then(|name| schema.pointer(&format!("/$defs/{name}")))
        })
        .unwrap_or(def)
}

#[allow(
    clippy::too_many_lines,
    reason = "schema property traversal records every nested checklist field in one pass"
)]
fn visit(
    schema: &Value,
    properties: &Value,
    prefix: &str,
    seen: &BTreeSet<String>,
    checklist: &mut Checklist,
    orders: &ObjectOrders,
) {
    for (name, raw) in ordered_entries(properties, orders) {
        let resolved = local_reference(schema, current_reference(schema, name, raw));
        let path = format!("{prefix}{name}");
        if raw.get("type").and_then(Value::as_str) == Some("array") {
            let unknown = ordered_entries(raw, orders)
                .into_iter()
                .map(|(key, _)| key.as_str())
                .filter(|key| {
                    !["type", "items", "minItems", "uniqueItems", "description"].contains(key)
                })
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                checklist
                    .warnings
                    .as_mut()
                    .expect("initialized checklist")
                    .push(format!(
                        "{path}: list uses {}, which this checklist does not read",
                        unknown.join(", ")
                    ));
            }
            let item = if raw.pointer("/items/$ref").is_some() {
                resolved
            } else {
                raw.get("items").unwrap_or(&Value::Null)
            };
            let shape = ListShape {
                min_items: raw.get("minItems").and_then(Value::as_u64).unwrap_or(0),
                unique_items: raw
                    .get("uniqueItems")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                item_pattern: item
                    .get("pattern")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                item_description: if item.get("pattern").is_some() {
                    item.get("description")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                } else {
                    None
                },
            };
            if shape.min_items > 0 || shape.unique_items || shape.item_pattern.is_some() {
                checklist.add_array(&path, shape);
            }
            if raw.pointer("/items/$ref").is_none()
                && let Some(values) = item.get("enum").and_then(Value::as_array)
            {
                checklist.add_enum(&path, values);
            }
        }
        if let Some(values) = resolved.get("enum").and_then(Value::as_array) {
            checklist.add_enum(&path, values);
        }
        if resolved.get("type").and_then(Value::as_str) != Some("object") {
            continue;
        }
        let Some(children) = resolved.get("properties") else {
            continue;
        };
        let identity = serde_json::to_string(resolved).unwrap_or_default();
        if seen.contains(&identity) {
            continue;
        }
        if let Some(required) = resolved
            .get("required")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
        {
            checklist
                .conditional_required
                .as_mut()
                .expect("initialized checklist")
                .push(ConditionalRequired {
                    when: format!("{path} is present"),
                    required: names(&Value::Array(required.clone()))
                        .iter()
                        .map(|name| format!("{path}.{name}"))
                        .collect(),
                });
        }
        let warning_start = checklist
            .warnings
            .as_ref()
            .expect("initialized checklist")
            .len();
        read_all_of(
            resolved,
            &format!("{path}."),
            &format!("{path} is present"),
            checklist,
            orders,
        );
        for warning in
            &mut checklist.warnings.as_mut().expect("initialized checklist")[warning_start..]
        {
            *warning = format!("{path}: {warning}");
        }
        let mut next_seen = seen.clone();
        next_seen.insert(identity);
        visit(
            schema,
            children,
            &format!("{path}."),
            &next_seen,
            checklist,
            orders,
        );
    }
}

fn schema_checklists(module_root: &Path) -> (BTreeMap<String, Checklist>, Vec<String>) {
    let mut mapping = Vec::new();
    let manifest = read_text_bounded(&module_root.join("manifest.yaml")).unwrap_or_default();
    let mut current_name: Option<String> = None;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name:") {
            current_name = Some(name.trim().to_owned());
        }
        if let (Some(name), Some(file)) = (
            &current_name,
            trimmed.strip_prefix("frontmatter_schema_ref: schemas/"),
        ) {
            mapping.push((name.clone(), file.trim().to_owned()));
        }
    }
    let entries = mapping
        .into_iter()
        .filter_map(|(kind, file)| {
            let relative = Path::new(&file);
            if !relative
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            {
                return None;
            }
            let schema_directory = module_root.join("schemas");
            let path = schema_directory.join(relative);
            if !fs::canonicalize(&path)
                .ok()
                .zip(fs::canonicalize(&schema_directory).ok())
                .is_some_and(|(file, root)| file.starts_with(root))
            {
                return None;
            }
            let mut checklist = Checklist {
                schema_path: path.display().to_string(),
                ..Checklist::default()
            };
            match read_text_bounded(&path).and_then(|text| {
                Some((
                    serde_json::from_str::<Value>(&text).ok()?,
                    serde_json::from_str::<OrderedNode>(&text).ok()?,
                ))
            }) {
                None => {
                    checklist.unreadable = Some("schema could not be read or parsed".to_owned());
                }
                Some((schema, ordered)) => {
                    let mut orders = ObjectOrders::new();
                    ordered.collect_orders(&schema, &mut orders);
                    checklist.required =
                        Some(names(schema.get("required").unwrap_or(&Value::Null)));
                    checklist.conditional_required = Some(Vec::new());
                    checklist.exactly_one_of = Some(Vec::new());
                    checklist.conditional_constraints = Some(Vec::new());
                    checklist.enums = Some(BTreeMap::new());
                    checklist.arrays = Some(BTreeMap::new());
                    checklist.warnings = Some(Vec::new());
                    read_all_of(&schema, "", "", &mut checklist, &orders);
                    visit(
                        &schema,
                        schema.get("properties").unwrap_or(&Value::Null),
                        "",
                        &BTreeSet::new(),
                        &mut checklist,
                        &orders,
                    );
                }
            }
            Some((kind, checklist))
        })
        .collect::<Vec<_>>();
    let order = entries.iter().map(|(kind, _)| kind.clone()).collect();
    (entries.into_iter().collect(), order)
}

const MODULE_RELATIONSHIP: &str = "engineering-assurance is the SCHEMA/SKELETON SOURCE: it supplies manifest.yaml, schemas/*.schema.json (what Quire validates), and skeletons/*.md (starting templates). It holds no project's actual decisions.";
const NO_ASSURANCE_NOTE: &str = "No spec/assurance/ yet — that is normal. Do not create one, or a generic AssuranceProfile/MeasurementPlan inside it, until a stated decision boundary justifies it.";
const EXAMPLES_NOTE: &str = "agent-ix/quoin's own spec/assurance/ is a live consumer of this module — real, validated AssuranceProfile and MeasurementPlan instances, not just skeletons.";
const COLLECTION_URL: &str = concat!(
    "https:",
    "//github.com/agent-ix/qa-corpus/tree/main/spec/evidence/measurements"
);
const INSTANCE_URL: &str = concat!(
    "https:",
    "//github.com/agent-ix/quoin/tree/main/spec/assurance"
);

fn directory_files(directory: &Path, suffix: Option<&str>) -> Vec<String> {
    let mut files = fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            suffix.is_none_or(|suffix| path.extension().and_then(OsStr::to_str) == Some(suffix))
        })
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn report(repo: &Path, module_root: &Path) -> Report {
    let available = module_root.join("schemas").is_dir() && module_root.join("skeletons").is_dir();
    let module_root = available.then_some(module_root);
    let assurance = repo.join("spec/assurance");
    let (exists, existing_artifacts) = artifacts(repo);
    let local_collection = module_root
        .map(|root| {
            directory_files(
                &root
                    .parent()
                    .unwrap_or(root)
                    .join("corpus/spec/evidence/measurements"),
                Some("json"),
            )
        })
        .unwrap_or_default();
    let measurement_collections = if local_collection.is_empty() {
        MeasurementCollections {
            source: "GitHub (not present in this install — corpus/ is engineering-assurance's own qa-corpus submodule, checked out only in a full dev clone with submodules initialized)",
            files: None,
            url: Some(COLLECTION_URL),
        }
    } else {
        MeasurementCollections {
            source: "local checkout",
            files: Some(local_collection),
            url: None,
        }
    };
    let measurement_record_checklist: MeasurementChecklist =
        serde_json::from_str(include_str!("onboard_report_measurement.json"))
            .expect("checked-in checklist must parse");
    let (artifact_checklists, artifact_order) =
        module_root.map(schema_checklists).unwrap_or_default();
    Report {
        target_repo: repo.display().to_string(),
        module_root: module_root.map(|path| path.display().to_string()),
        installed_module_version: module_root.and_then(module_version),
        relationship: Relationship {
            module: MODULE_RELATIONSHIP,
            instances: format!(
                "{} is the INSTANCE HOME: every AssuranceProfile (AP-*.md), MeasurementPlan (MP-*.md), ArchitectureDescription (AD-*.md), ComponentAssuranceContract (CAC-*.md), and AssuranceArgument (AA-*.md) this project actually authors lives there, validated against the schemas the module supplies.",
                assurance.display()
            ),
        },
        spec_assurance: SpecAssurance {
            path: assurance.display().to_string(),
            exists,
            existing_artifacts,
            note: (!exists).then_some(NO_ASSURANCE_NOTE),
        },
        working_examples: WorkingExamples {
            measurement_collections,
            real_artifact_instances: LinkedExample {
                note: EXAMPLES_NOTE,
                url: INSTANCE_URL,
            },
            skeletons: module_root
                .map(|root| directory_files(&root.join("skeletons"), None))
                .unwrap_or_default(),
        },
        artifact_checklists,
        artifact_order,
        measurement_record_checklist,
        next_steps: vec![
            "Decide the exact decision boundary and its human owner before authoring anything.",
            "Reuse an existing valid artifact in spec/assurance/ if one already covers the decision.",
            "Author from the module's skeleton, not from memory.",
            "Validate with `quire validate --scope . 'spec/assurance/**/*.md'` before treating it as real.",
            "Enter a workflow (assurance-intake / architecture-evaluation / measurement-promotion / change-assurance) through ix-flow once the artifact is validated — see SKILL.md.",
        ],
    }
}

fn heading(title: &str) {
    println!("\n{title}\n{}", "=".repeat(title.chars().count()));
}

#[allow(
    clippy::too_many_lines,
    reason = "the six report sections are rendered in their published reading order"
)]
fn print_report(report: &Report) {
    heading("Engineering Assurance — onboarding report");
    println!("Target repository: {}", report.target_repo);
    println!(
        "Module root:        {}",
        report
            .module_root
            .as_deref()
            .unwrap_or("(not found locally — using GitHub links only)")
    );
    heading("1. How the two homes relate");
    println!(
        "- {}\n- {}",
        report.relationship.module, report.relationship.instances
    );
    heading("2. This repo's spec/assurance/");
    if report.spec_assurance.exists {
        println!(
            "{} exists with {} artifact(s):",
            report.spec_assurance.path,
            report.spec_assurance.existing_artifacts.len()
        );
        for artifact in &report.spec_assurance.existing_artifacts {
            println!("  - {} ({})", artifact.name, artifact.kind);
        }
    } else {
        println!("{}", report.spec_assurance.note.unwrap_or(""));
    }
    heading("3. Working examples — read these, not the Rust source");
    if let Some(files) = &report.working_examples.measurement_collections.files {
        println!("Valid measurement-collection records on disk:");
        for file in files {
            println!("  - {file}");
        }
    } else {
        println!("Valid measurement-collection records (this install has no local corpus/):");
        if let Some(url) = report.working_examples.measurement_collections.url {
            println!("  - {url}");
        }
    }
    println!(
        "\n{}\n  - {}",
        report.working_examples.real_artifact_instances.note,
        report.working_examples.real_artifact_instances.url
    );
    if !report.working_examples.skeletons.is_empty() {
        println!("\nCanonical starting skeletons in this module:");
        for file in &report.working_examples.skeletons {
            println!("  - {file}");
        }
    }
    heading("4. What each artifact type actually requires");
    if report.artifact_checklists.is_empty() {
        println!("(schemas not found locally — resolve your installed module root and re-run)");
    }
    for kind in &report.artifact_order {
        let Some(checklist) = report.artifact_checklists.get(kind) else {
            continue;
        };
        println!("{kind} ({}):", checklist.schema_path);
        if let Some(error) = &checklist.unreadable {
            println!("  (schema could not be parsed: {error})\n");
            continue;
        }
        println!(
            "  required: {}",
            checklist.required.as_deref().unwrap_or(&[]).join(", ")
        );
        for path in &checklist.enum_order {
            let Some(values) = checklist.enums.as_ref().and_then(|m| m.get(path)) else {
                continue;
            };
            println!(
                "  {path} must be one of: {}",
                values.iter().map(value_text).collect::<Vec<_>>().join(", ")
            );
        }
        for path in &checklist.array_order {
            let Some(shape) = checklist.arrays.as_ref().and_then(|m| m.get(path)) else {
                continue;
            };
            let mut clauses = Vec::new();
            if shape.min_items > 0 {
                clauses.push(format!("at least {} item(s)", shape.min_items));
            }
            if shape.unique_items {
                clauses.push("no repeated item".to_owned());
            }
            if let Some(pattern) = &shape.item_pattern {
                clauses.push(format!(
                    "each item: {}",
                    shape
                        .item_description
                        .as_deref()
                        .map_or_else(|| format!("matching {pattern}"), str::to_owned)
                ));
            }
            println!("  {path} is a list: {}", clauses.join(", "));
        }
        for entry in checklist
            .conditional_required
            .as_ref()
            .into_iter()
            .flatten()
        {
            println!(
                "  when {}: also required: {}",
                entry.when,
                entry.required.join(", ")
            );
        }
        for entry in checklist.exactly_one_of.as_ref().into_iter().flatten() {
            println!(
                "  when {}: exactly one of: {}",
                entry.when,
                entry.fields.join(", ")
            );
        }
        for entry in checklist
            .conditional_constraints
            .as_ref()
            .into_iter()
            .flatten()
        {
            println!("  when {}: {}", entry.when, entry.constraint);
        }
        for warning in checklist.warnings.as_ref().into_iter().flatten() {
            println!("  WARNING: {warning} — this checklist may be incomplete for {kind}");
        }
        println!();
    }
    heading("5. What a measurement record actually requires");
    println!(
        "(hand-restated from quoin-measurement's Rust validator, not schema-derived — see the source citation at the end of this section)\n"
    );
    println!("Every collection, read back or newly submitted (schemaVersion 1 or 2):");
    for item in &report.measurement_record_checklist.collection.always {
        println!("  - {item}");
    }
    println!("\nWhenever schemaVersion is 2 — read back or newly submitted:");
    for item in &report
        .measurement_record_checklist
        .collection
        .schema_version2_verification_stack
    {
        println!("  - {item}");
    }
    println!(
        "\nA NEW collection being submitted (not a stored one being read) additionally requires:"
    );
    for item in &report
        .measurement_record_checklist
        .collection
        .new_collection_intake_only
    {
        println!("  - {item}");
    }
    println!("\nEach observation:");
    for item in &report.measurement_record_checklist.observation {
        println!("  - {item}");
    }
    println!("\n{}", report.measurement_record_checklist.source_of_truth);
    heading("6. Next steps");
    for step in &report.next_steps {
        println!("  - {step}");
    }
    println!();
}

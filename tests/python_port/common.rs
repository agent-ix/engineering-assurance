//! Helpers shared by the ported suites.
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `engineering_assurance/` package root (was `package.PACKAGE_ROOT`).
pub fn package_root() -> PathBuf {
    root().join("engineering_assurance")
}

pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub fn yaml(text: &str) -> serde_json::Value {
    yaml_serde::from_str(text).expect("valid yaml")
}

pub fn manifest() -> serde_json::Value {
    yaml(&read(&package_root().join("manifest.yaml")))
}

/// Leading `---` YAML frontmatter of a markdown file.
pub fn frontmatter(path: &Path) -> serde_json::Value {
    let text = read(path);
    let rest = text.strip_prefix("---\n").expect("frontmatter start");
    let end = rest.find("\n---\n").expect("frontmatter end");
    yaml(&rest[..end])
}

pub fn schema(name: &str) -> serde_json::Value {
    let p = package_root().join("schemas").join(format!("{name}.json"));
    serde_json::from_str(&read(&p)).expect("valid json")
}

/// Merge the entries of JSON object `over` into JSON object `base`.
pub fn merged(mut base: serde_json::Value, over: serde_json::Value) -> serde_json::Value {
    let (Some(target), serde_json::Value::Object(extra)) = (base.as_object_mut(), over) else {
        panic!("merged needs two JSON objects");
    };
    target.extend(extra);
    base
}

/// Files directly in `dir` with the given extension, sorted.
pub fn glob_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == ext))
        .collect();
    out.sort();
    out
}

/// Schema validation messages for `instance`, one per error.
pub fn schema_errors(
    validator: &jsonschema::Validator,
    instance: &serde_json::Value,
) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect()
}

/// Python truthiness of a JSON value, as the retired suite's bare `assert x`
/// judged it: `null`, `false`, `0`, `""`, `[]` and `{}` are falsy.
pub fn truthy(value: &serde_json::Value) -> bool {
    use serde_json::Value;
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|n| n.abs() > 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(members) => !members.is_empty(),
    }
}

/// The message jsonschema gives when `name` is missing from an object.
pub fn required_msg(name: &str) -> String {
    format!("\"{name}\" is a required property")
}

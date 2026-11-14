// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Producer-reported CycloneDX 1.7 agent BOM construction (FR-073).

use std::collections::BTreeMap;

use cyclonedx_bom::{
    models::bom::{Bom, SpecVersion},
    prelude::Validate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Errors returned for invalid session descriptors or BOM documents.
#[derive(Debug, Error)]
pub enum BomError {
    /// The descriptor or resulting BOM is not valid for CycloneDX 1.7.
    #[error("agent BOM is invalid")]
    Invalid,
    /// The input document contains a forbidden formulation or declaration.
    #[error("agent BOM contains a forbidden section")]
    ForbiddenSection,
}

/// A producer-reported plugin package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    /// Display name.
    pub name: String,
    /// CycloneDX package URL.
    pub purl: String,
}

/// A producer-reported MCP server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServer {
    /// Service name.
    pub name: String,
    /// Optional endpoint reported by the producer.
    pub endpoint: Option<String>,
}

/// Session descriptor supplied by the producer and treated as unverified data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSession {
    /// Harness application name.
    pub harness: String,
    /// Model name, kept inside the private form.
    pub model: String,
    /// Plugins loaded by the session.
    pub plugins: Vec<Plugin>,
    /// MCP servers available to the session.
    pub mcp_servers: Vec<McpServer>,
    /// Other producer-reported session facts.
    #[serde(default)]
    pub facts: BTreeMap<String, String>,
}

/// CycloneDX 1.7 BOM document produced for one agent session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CycloneDxBom {
    /// CycloneDX format marker.
    pub bom_format: String,
    /// Specification version.
    pub spec_version: String,
    /// Unique version 4 URN UUID.
    pub serial_number: String,
    /// BOM revision.
    pub version: u32,
    /// Producer-reported components.
    pub components: Vec<Component>,
    /// Producer-reported services.
    pub services: Vec<Service>,
    /// Producer-reported document properties.
    pub properties: Vec<Property>,
}

/// Component type and producer-reported metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentType {
    /// Agent harness.
    Application,
    /// Agent model.
    MachineLearningModel,
    /// Plugin package.
    Library,
}

/// CycloneDX component.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Component {
    /// CycloneDX component kind.
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    /// Component name.
    pub name: String,
    /// Package URL for plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
    /// Producer-reported component properties.
    pub properties: Vec<Property>,
}

/// CycloneDX service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Service {
    /// Service name.
    pub name: String,
    /// Service endpoints, when reported.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub endpoints: Vec<String>,
    /// Producer-reported service properties.
    pub properties: Vec<Property>,
}

/// A CycloneDX key/value property.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Property {
    /// Property name, always prefixed `agent-ix:`.
    pub name: String,
    /// Producer-reported value.
    pub value: String,
}

/// Builds and validates a CycloneDX 1.7 BOM with a random serial number.
pub fn build_bom(session: &AgentSession) -> Result<CycloneDxBom, BomError> {
    if session.harness.is_empty()
        || session.model.is_empty()
        || session
            .plugins
            .iter()
            .any(|plugin| plugin.name.is_empty() || !plugin.purl.starts_with("pkg:"))
        || session
            .mcp_servers
            .iter()
            .any(|server| server.name.is_empty())
    {
        return Err(BomError::Invalid);
    }
    let mut components = vec![
        Component {
            component_type: ComponentType::Application,
            name: session.harness.clone(),
            purl: None,
            properties: reported_properties(),
        },
        Component {
            component_type: ComponentType::MachineLearningModel,
            name: session.model.clone(),
            purl: None,
            properties: reported_properties(),
        },
    ];
    components.extend(session.plugins.iter().map(|plugin| Component {
        component_type: ComponentType::Library,
        name: plugin.name.clone(),
        purl: Some(plugin.purl.clone()),
        properties: reported_properties(),
    }));
    let services = std::iter::once(Service {
        name: session.model.clone(),
        endpoints: Vec::new(),
        properties: reported_properties(),
    })
    .chain(session.mcp_servers.iter().map(|server| Service {
        name: server.name.clone(),
        endpoints: server.endpoint.iter().cloned().collect(),
        properties: reported_properties(),
    }))
    .collect();
    let mut properties = reported_properties();
    for (key, value) in &session.facts {
        if key.is_empty() || key.contains(':') {
            return Err(BomError::Invalid);
        }
        properties.push(Property {
            name: format!("agent-ix:session:{key}"),
            value: value.clone(),
        });
        properties.push(Property {
            name: "agent-ix:reported-by".to_owned(),
            value: "producer".to_owned(),
        });
    }
    let bom = CycloneDxBom {
        bom_format: "CycloneDX".to_owned(),
        spec_version: "1.7".to_owned(),
        serial_number: format!("urn:uuid:{}", Uuid::new_v4()),
        version: 1,
        components,
        services,
        properties,
    };
    validate_bom(&bom)?;
    Ok(bom)
}

fn reported_properties() -> Vec<Property> {
    vec![Property {
        name: "agent-ix:reported-by".to_owned(),
        value: "producer".to_owned(),
    }]
}

/// Validates that a BOM has no forbidden sections and passes CycloneDX 1.7 validation.
pub fn validate_bom(bom: &CycloneDxBom) -> Result<(), BomError> {
    let value = serde_json::to_value(bom).map_err(|_| BomError::Invalid)?;
    validate_bom_value(&value)
}

/// Validates untrusted serialized BOM JSON and rejects formulation/declarations.
pub fn validate_bom_value(value: &Value) -> Result<(), BomError> {
    let object = value.as_object().ok_or(BomError::Invalid)?;
    if object.contains_key("formulation") || object.contains_key("declarations") {
        return Err(BomError::ForbiddenSection);
    }
    let bom: CycloneDxBom = serde_json::from_value(value.clone()).map_err(|_| BomError::Invalid)?;
    if bom.bom_format != "CycloneDX" || bom.spec_version != "1.7" {
        return Err(BomError::Invalid);
    }
    let bytes = serde_json::to_vec(&bom).map_err(|_| BomError::Invalid)?;
    let parsed = Bom::parse_from_json_with_version(bytes.as_slice(), SpecVersion::V1_7)
        .map_err(|_| BomError::Invalid)?;
    if parsed.validate_version(SpecVersion::V1_7).passed() {
        Ok(())
    } else {
        Err(BomError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> AgentSession {
        AgentSession {
            harness: "Fictional Harness".to_owned(),
            model: "Private Model Name".to_owned(),
            plugins: vec![Plugin {
                name: "Fictional Plugin".to_owned(),
                purl: "pkg:generic/fictional-plugin@1.0".to_owned(),
            }],
            mcp_servers: vec![McpServer {
                name: "Fictional MCP".to_owned(),
                endpoint: None,
            }],
            facts: BTreeMap::from([("session-kind".to_owned(), "coding".to_owned())]),
        }
    }

    /// Trace: FR-073-AC-1, FR-073-AC-2
    #[test]
    fn builds_valid_reported_cyclonedx_17_document() {
        let bom = build_bom(&session()).expect("BOM builds");
        assert_eq!(bom.spec_version, "1.7");
        assert!(
            bom.components
                .iter()
                .any(|component| component.component_type == ComponentType::Application)
        );
        assert!(
            bom.components
                .iter()
                .any(|component| component.component_type == ComponentType::MachineLearningModel)
        );
        assert!(
            bom.components
                .iter()
                .any(|component| component.purl.is_some())
        );
        assert_eq!(bom.services.len(), 2);
        let value = serde_json::to_value(&bom).expect("JSON");
        let properties = value["properties"].as_array().expect("properties");
        assert!(properties.iter().all(|property| {
            property["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("agent-ix:"))
        }));
        assert!(
            properties
                .iter()
                .filter(|property| property["name"] == "agent-ix:reported-by")
                .all(|property| property["value"] == "producer")
        );
        assert!(validate_bom_value(&value).is_ok());
    }

    /// Trace: FR-073-AC-3
    #[test]
    fn rejects_formulation_section() {
        let mut value =
            serde_json::to_value(build_bom(&session()).expect("BOM builds")).expect("JSON");
        value["formulation"] = serde_json::json!([]);
        assert!(matches!(
            validate_bom_value(&value),
            Err(BomError::ForbiddenSection)
        ));
    }

    /// Trace: FR-073-AC-6
    #[test]
    fn identical_sessions_get_distinct_random_serials() {
        let first = build_bom(&session()).expect("first BOM");
        let second = build_bom(&session()).expect("second BOM");
        assert_ne!(first.serial_number, second.serial_number);
    }
}

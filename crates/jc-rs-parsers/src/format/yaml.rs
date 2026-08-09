//! YAML file parser.
//!
//! Parses YAML documents into JSON. Supports single and multi-document YAML.
//! Single document → Object or Array depending on top-level type.
//! Multi-document (---) → Array of documents.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct YamlParser;

static YAML_INFO: ParserInfo = ParserInfo {
    name: "yaml",
    argument: "--yaml",
    version: "1.0.0",
    description: "YAML file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

impl Parser for YamlParser {
    fn info(&self) -> &'static ParserInfo {
        &YAML_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // One call covers both shapes: a single document comes back as a
        // one-element vector.
        let docs: Vec<serde_json::Value> = serde_saphyr::from_multiple(input)
            .map_err(|e| ParseError::Generic(format!("YAML parse error: {e}")))?;

        if docs.is_empty() {
            return Err(ParseError::InvalidInput("empty YAML document".to_string()));
        }

        // jc always returns an Array for YAML (single or multi-doc)
        let rows = docs
            .into_iter()
            .map(|v| match v {
                serde_json::Value::Object(m) => m,
                other => {
                    let mut m = serde_json::Map::new();
                    m.insert("value".to_string(), other);
                    m
                }
            })
            .collect();
        Ok(ParseOutput::Array(rows))
    }
}

static YAML_PARSER_INSTANCE: YamlParser = YamlParser;

inventory::submit! {
    ParserEntry::new(&YAML_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/generic");

    fn load_fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    #[test]
    fn test_yaml_istio_sidecar_multidoc() {
        let input = load_fixture("yaml-istio-sidecar.yaml");
        let expected: serde_json::Value =
            serde_json::from_str(&load_fixture("yaml-istio-sidecar.json"))
                .expect("invalid fixture JSON");

        let parser = YamlParser;
        let result = parser.parse(&input, false).unwrap();

        let result_json = match result {
            ParseOutput::Array(rows) => {
                serde_json::Value::Array(rows.into_iter().map(serde_json::Value::Object).collect())
            }
            ParseOutput::Object(m) => serde_json::Value::Object(m),
        };

        assert_eq!(result_json, expected);
    }

    #[test]
    fn test_yaml_istio_sc_single() {
        let input = load_fixture("yaml-istio-sc.yaml");
        let expected: serde_json::Value = serde_json::from_str(&load_fixture("yaml-istio-sc.json"))
            .expect("invalid fixture JSON");

        let parser = YamlParser;
        let result = parser.parse(&input, false).unwrap();

        let result_json = match result {
            ParseOutput::Array(rows) => {
                serde_json::Value::Array(rows.into_iter().map(serde_json::Value::Object).collect())
            }
            ParseOutput::Object(m) => serde_json::Value::Object(m),
        };

        assert_eq!(result_json, expected);
    }
}

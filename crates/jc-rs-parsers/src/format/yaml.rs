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
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Convert a serde_yaml Value into a serde_json Value.
fn yaml_to_json(v: serde_yaml::Value) -> Result<serde_json::Value, ParseError> {
    match v {
        serde_yaml::Value::Null => Ok(serde_json::Value::Null),
        serde_yaml::Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::Number(serde_json::Number::from(i)))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| ParseError::Generic("non-finite float in YAML".to_string()))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        serde_yaml::Value::String(s) => Ok(serde_json::Value::String(s)),
        serde_yaml::Value::Sequence(seq) => {
            let arr: Result<Vec<_>, _> = seq.into_iter().map(yaml_to_json).collect();
            Ok(serde_json::Value::Array(arr?))
        }
        serde_yaml::Value::Mapping(m) => {
            let mut map = serde_json::Map::new();
            for (k, v) in m {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    other => format!("{other:?}"),
                };
                map.insert(key, yaml_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

impl Parser for YamlParser {
    fn info(&self) -> &'static ParserInfo {
        &YAML_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // Try multi-document parsing first
        let docs: Vec<serde_yaml::Value> = serde_yaml::Deserializer::from_str(input)
            .map(serde_yaml::Value::deserialize)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ParseError::Generic(format!("YAML parse error: {e}")))?;

        if docs.is_empty() {
            return Err(ParseError::InvalidInput("empty YAML document".to_string()));
        }

        // jc always returns an Array for YAML (single or multi-doc)
        let rows: Result<Vec<_>, _> = docs
            .into_iter()
            .map(|d| {
                yaml_to_json(d).map(|v| match v {
                    serde_json::Value::Object(m) => m,
                    other => {
                        let mut m = serde_json::Map::new();
                        m.insert("value".to_string(), other);
                        m
                    }
                })
            })
            .collect();
        Ok(ParseOutput::Array(rows?))
    }
}

use serde::Deserialize;

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

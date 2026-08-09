//! Parser for `/etc/shadow` file format.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct ShadowParser;

static INFO: ParserInfo = ParserInfo {
    name: "shadow",
    argument: "--shadow",
    version: "1.5.0",
    description: "Converts `/etc/shadow` file content to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SHADOW_PARSER: ShadowParser = ShadowParser;

inventory::submit! {
    ParserEntry::new(&SHADOW_PARSER)
}

fn nullable_int(s: &str) -> Value {
    if s.is_empty() {
        Value::Null
    } else {
        convert_to_int(s).map(Value::from).unwrap_or(Value::Null)
    }
}

impl Parser for ShadowParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: username:password:last_changed:minimum:maximum:warn:inactive:expire:reserved
            let parts: Vec<&str> = line.splitn(9, ':').collect();
            if parts.len() < 2 {
                continue;
            }

            let mut obj = Map::new();
            obj.insert("username".to_string(), Value::String(parts[0].to_string()));
            obj.insert("password".to_string(), Value::String(parts[1].to_string()));
            obj.insert(
                "last_changed".to_string(),
                nullable_int(parts.get(2).unwrap_or(&"")),
            );
            obj.insert(
                "minimum".to_string(),
                nullable_int(parts.get(3).unwrap_or(&"")),
            );
            obj.insert(
                "maximum".to_string(),
                nullable_int(parts.get(4).unwrap_or(&"")),
            );
            obj.insert(
                "warn".to_string(),
                nullable_int(parts.get(5).unwrap_or(&"")),
            );
            obj.insert(
                "inactive".to_string(),
                nullable_int(parts.get(6).unwrap_or(&"")),
            );
            obj.insert(
                "expire".to_string(),
                nullable_int(parts.get(7).unwrap_or(&"")),
            );
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/shadow.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/shadow.json"
        ))
        .unwrap();
        let parser = ShadowParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    got["username"], exp["username"],
                    "username mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["password"], exp["password"],
                    "password mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["last_changed"], exp["last_changed"],
                    "last_changed mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["minimum"], exp["minimum"],
                    "minimum mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["maximum"], exp["maximum"],
                    "maximum mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_shadow_empty() {
        let parser = ShadowParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

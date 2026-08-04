//! Parser for `airport -I` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct AirportParser;

static INFO: ParserInfo = ParserInfo {
    name: "airport",
    argument: "--airport",
    version: "1.5.0",
    description: "`airport -I` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Darwin],
    tags: &[Tag::Command],
    magic_commands: &["airport -I"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static AIRPORT_PARSER: AirportParser = AirportParser;

inventory::submit! {
    ParserEntry::new(&AIRPORT_PARSER)
}

/// Normalize a key: lowercase, spaces → _, dots → _
fn normalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .replace(' ', "_")
        .replace('.', "_")
}

impl Parser for AirportParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let int_keys = [
            "agrctlrssi",
            "agrextrssi",
            "agrctlnoise",
            "agrextnoise",
            "lasttxrate",
            "maxrate",
            "lastassocstatus",
            "mcs",
        ];

        let mut obj = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Split on first ':'
            if let Some(colon_pos) = line.find(':') {
                let raw_key = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim().to_string();
                let key = normalize_key(raw_key);

                if int_keys.contains(&key.as_str()) {
                    let int_val = convert_to_int(&value)
                        .map(Value::from)
                        .unwrap_or(Value::Null);
                    obj.insert(key, int_val);
                } else {
                    obj.insert(key, Value::String(value));
                }
            }
        }

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airport_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/airport-I.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/airport-I.json"
        ))
        .unwrap();

        let parser = AirportParser;
        let result = parser.parse(input, false).unwrap();

        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(got, expected);
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_airport_empty() {
        let parser = AirportParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}

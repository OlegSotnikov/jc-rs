//! Parser for `airport -s` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_bool, convert_to_int, sparse_table_parse};
use serde_json::{Map, Value};

pub struct AirportSParser;

static INFO: ParserInfo = ParserInfo {
    name: "airport_s",
    argument: "--airport-s",
    version: "1.6.0",
    description: "`airport -s` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Darwin],
    tags: &[Tag::Command],
    magic_commands: &["airport -s"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static AIRPORT_S_PARSER: AirportSParser = AirportSParser;

inventory::submit! {
    ParserEntry::new(&AIRPORT_S_PARSER)
}

impl Parser for AirportSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Filter empty lines
        let clean_lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        if clean_lines.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Preprocess header: lowercase, replace '-' with '_', replace security header
        let header = clean_lines[0]
            .to_lowercase()
            .replace('-', "_")
            .replace("security (auth/unicast/group)", "security");

        // Build processed data string for sparse_table_parse
        let mut processed_lines = vec![header.as_str()];
        processed_lines.extend_from_slice(&clean_lines[1..]);
        let table_input = processed_lines.join("\n");

        let raw_rows = sparse_table_parse(&table_input);

        let mut result = Vec::new();

        for row in raw_rows {
            let mut obj = Map::new();

            // ssid
            let ssid = row.get("ssid").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            obj.insert(
                "ssid".to_string(),
                ssid.map(Value::String).unwrap_or(Value::Null),
            );

            // bssid
            let bssid = row.get("bssid").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            obj.insert(
                "bssid".to_string(),
                bssid.map(Value::String).unwrap_or(Value::Null),
            );

            // rssi (integer)
            let rssi_str = row.get("rssi").and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            });
            let rssi_val = rssi_str
                .and_then(convert_to_int)
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("rssi".to_string(), rssi_val);

            // channel
            let channel = row.get("channel").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            obj.insert(
                "channel".to_string(),
                channel.map(Value::String).unwrap_or(Value::Null),
            );

            // ht (boolean: "Y" → true, "N" → false)
            let ht_str = row.get("ht").and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            });
            let ht_val = ht_str
                .and_then(convert_to_bool)
                .map(Value::Bool)
                .unwrap_or(Value::Null);
            obj.insert("ht".to_string(), ht_val);

            // cc
            let cc = row.get("cc").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            obj.insert(
                "cc".to_string(),
                cc.map(Value::String).unwrap_or(Value::Null),
            );

            // security: split on whitespace to get list of security types
            let security_str = row.get("security").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            let security_arr: Vec<Value> = security_str
                .as_deref()
                .map(|s| {
                    s.split_whitespace()
                        .map(|sec| Value::String(sec.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            obj.insert("security".to_string(), Value::Array(security_arr));

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airport_s_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/airport-s.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/airport-s.json"
        ))
        .unwrap();

        let parser = AirportSParser;
        let result = parser.parse(input, false).unwrap();

        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    serde_json::Value::Object(got.clone()),
                    *exp,
                    "mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_airport_s_empty() {
        let parser = AirportSParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

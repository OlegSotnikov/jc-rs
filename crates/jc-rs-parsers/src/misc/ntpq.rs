//! Parser for `ntpq -p` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int, simple_table_parse};
use serde_json::{Map, Value};

pub struct NtpqParser;

static INFO: ParserInfo = ParserInfo {
    name: "ntpq",
    argument: "--ntpq",
    version: "1.7.0",
    description: "Converts `ntpq -p` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["ntpq"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static NTPQ_PARSER: NtpqParser = NtpqParser;

inventory::submit! {
    ParserEntry::new(&NTPQ_PARSER)
}

impl Parser for NtpqParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();
        if lines.len() < 2 {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Prepend 's ' to header line and lowercase it
        lines[0] = format!("s {}", lines[0].to_lowercase());

        // Remove separator line (was at index 1)
        if lines.len() > 1 {
            lines.remove(1);
        }

        // Process data lines (index 1..n after removal)
        for slot in lines.iter_mut().skip(1) {
            let line = slot.clone();
            let processed = if line.starts_with(' ') {
                // Space prefix = no state = null (~)
                format!("~  {}", &line[1..])
            } else if !line.is_empty() {
                // State char is first character
                let ch = &line[..1];
                format!("{}  {}", ch, &line[1..])
            } else {
                line.clone()
            };
            // Replace " (" with "_(" to handle hostnames with parentheses
            let processed = processed.replace(" (", "_(");
            *slot = processed;
        }

        let table_str = lines.join("\n");
        let rows = simple_table_parse(&table_str);

        let mut result = Vec::new();
        for row in rows {
            let mut obj = Map::new();

            // state: rename from 's', convert ~ to null
            let state_raw = row
                .get("s")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let state = if state_raw == "~" || state_raw.trim().is_empty() {
                Value::Null
            } else {
                Value::String(state_raw)
            };
            obj.insert("state".to_string(), state);

            // remote
            obj.insert(
                "remote".to_string(),
                row.get("remote")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            );

            // refid
            obj.insert(
                "refid".to_string(),
                row.get("refid")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            );

            // st -> integer
            let st = row
                .get("st")
                .and_then(|v| v.as_str())
                .and_then(convert_to_int)
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("st".to_string(), st);

            // t -> string
            obj.insert(
                "t".to_string(),
                row.get("t")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            );

            // when -> integer or null if "-"
            let when_str = row
                .get("when")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string();
            let when = if when_str == "-" {
                Value::Null
            } else {
                convert_to_int(&when_str)
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            };
            obj.insert("when".to_string(), when);

            // poll -> integer
            let poll = row
                .get("poll")
                .and_then(|v| v.as_str())
                .and_then(convert_to_int)
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("poll".to_string(), poll);

            // reach -> integer
            let reach = row
                .get("reach")
                .and_then(|v| v.as_str())
                .and_then(convert_to_int)
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("reach".to_string(), reach);

            // delay -> float
            let delay = row
                .get("delay")
                .and_then(|v| v.as_str())
                .and_then(convert_to_float)
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null);
            obj.insert("delay".to_string(), delay);

            // offset -> float
            let offset = row
                .get("offset")
                .and_then(|v| v.as_str())
                .and_then(convert_to_float)
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null);
            obj.insert("offset".to_string(), offset);

            // jitter -> float
            let jitter = row
                .get("jitter")
                .and_then(|v| v.as_str())
                .and_then(convert_to_float)
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null);
            obj.insert("jitter".to_string(), jitter);

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_fixture(input: &str, expected_json: &str) {
        let parser = NtpqParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), expected.len(), "row count mismatch");
                for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                    for field in &[
                        "state", "remote", "refid", "st", "t", "when", "poll", "reach", "delay",
                        "offset", "jitter",
                    ] {
                        assert_eq!(
                            got.get(*field).unwrap_or(&Value::Null),
                            exp.get(*field).unwrap_or(&Value::Null),
                            "field '{}' mismatch at row {}",
                            field,
                            i
                        );
                    }
                }
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_ntpq_centos77() {
        check_fixture(
            include_str!("../../../../tests/fixtures/centos-7.7/ntpq-p.out"),
            include_str!("../../../../tests/fixtures/centos-7.7/ntpq-p.json"),
        );
    }

    #[test]
    fn test_ntpq_centos77_pn() {
        check_fixture(
            include_str!("../../../../tests/fixtures/centos-7.7/ntpq-pn.out"),
            include_str!("../../../../tests/fixtures/centos-7.7/ntpq-pn.json"),
        );
    }

    #[test]
    fn test_ntpq_ubuntu1804() {
        check_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-18.04/ntpq-p.out"),
            include_str!("../../../../tests/fixtures/ubuntu-18.04/ntpq-p.json"),
        );
    }

    #[test]
    fn test_ntpq_ubuntu1804_pn() {
        check_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-18.04/ntpq-pn.out"),
            include_str!("../../../../tests/fixtures/ubuntu-18.04/ntpq-pn.json"),
        );
    }

    #[test]
    fn test_ntpq_empty() {
        let parser = NtpqParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}

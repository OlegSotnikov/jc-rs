//! Parser for `pip show` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct PipShowParser;

static INFO: ParserInfo = ParserInfo {
    name: "pip_show",
    argument: "--pip-show",
    version: "1.0.0",
    description: "Converts `pip show` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &["pip show", "pip3 show"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PIP_SHOW_PARSER: PipShowParser = PipShowParser;

inventory::submit! {
    ParserEntry::new(&PIP_SHOW_PARSER)
}

impl Parser for PipShowParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let clean: Vec<&str> = input.lines().collect();

        if clean.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut package: Map<String, Value> = Map::new();
        let mut last_key = String::new();
        let mut last_key_data: Vec<String> = Vec::new();

        for row in &clean {
            if row.starts_with("---") {
                // Finalize multiline field
                if !last_key_data.is_empty() {
                    let existing = package
                        .get(&last_key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let combined = format!("{}\n{}", existing, last_key_data.join("\n"));
                    package.insert(last_key.clone(), Value::String(combined));
                    last_key_data.clear();
                }
                results.push(package);
                package = Map::new();
                last_key = String::new();
                continue;
            }

            if !row.starts_with(' ') {
                // New key-value line
                let parts: Vec<&str> = row.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    let key = parts[0].to_lowercase().replace('-', "_");
                    let val = parts[1];

                    // Flush previous multiline data
                    if !last_key_data.is_empty() && last_key != key {
                        let existing = package
                            .get(&last_key)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let combined = format!("{}\n{}", existing, last_key_data.join("\n"));
                        package.insert(last_key.clone(), Value::String(combined));
                        last_key_data.clear();
                    }

                    let value = if val.is_empty() {
                        Value::Null
                    } else {
                        Value::String(val.to_string())
                    };
                    package.insert(key.clone(), value);
                    last_key = key;
                }
            } else {
                // Continuation line
                last_key_data.push(row.trim().to_string());
            }
        }

        // Push final package
        if !package.is_empty() {
            if !last_key_data.is_empty() {
                let existing = package
                    .get(&last_key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let combined = format!("{}\n{}", existing, last_key_data.join("\n"));
                package.insert(last_key.clone(), Value::String(combined));
            }
            results.push(package);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_show_smoke() {
        let input = "Name: foo\nVersion: 1.0\nSummary: A package\nHome-page: http://example.com\nAuthor: Test\nAuthor-email: test@example.com\nLicense: MIT\nLocation: /usr/lib\nRequires: \nRequired-by: bar\n";
        let parser = PipShowParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].get("name"), Some(&Value::String("foo".into())));
            assert_eq!(arr[0].get("requires"), Some(&Value::Null));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_pip_show_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/centos-7.7/pip-show.out");
        let fixture_json = include_str!("../../../../tests/fixtures/centos-7.7/pip-show.json");

        let parser = PipShowParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "pip_show fixture mismatch");
    }
}

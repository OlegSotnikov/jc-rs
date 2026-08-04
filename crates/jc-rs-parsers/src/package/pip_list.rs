//! Parser for `pip list` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::simple_table_parse;
use serde_json::{Map, Value};

pub struct PipListParser;

static INFO: ParserInfo = ParserInfo {
    name: "pip_list",
    argument: "--pip-list",
    version: "1.0.0",
    description: "Converts `pip list` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &["pip list", "pip3 list"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PIP_LIST_PARSER: PipListParser = PipListParser;

inventory::submit! {
    ParserEntry::new(&PIP_LIST_PARSER)
}

impl Parser for PipListParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut clean: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

        if clean.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Detect legacy format: "Package (version)"
        if clean[0].contains(" (") {
            let results: Vec<Map<String, Value>> = clean
                .iter()
                .map(|row| {
                    let mut entry: Map<String, Value> = Map::new();
                    let mut parts = row.splitn(2, " (");
                    let pkg = parts.next().unwrap_or("").trim().to_string();
                    let ver = parts.next().unwrap_or("").trim_end_matches(')').to_string();
                    entry.insert("package".to_string(), Value::String(pkg));
                    entry.insert("version".to_string(), Value::String(ver));
                    entry
                })
                .collect();
            return Ok(ParseOutput::Array(results));
        }

        // Normal table: remove separator lines (contain ---)
        clean.retain(|l| !l.contains("---"));

        if clean.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Lowercase header
        let header_lower = clean[0].to_lowercase();
        let mut lines: Vec<String> = vec![header_lower];
        for l in &clean[1..] {
            lines.push(l.to_string());
        }

        let table_str = lines.join("\n");
        let raw = simple_table_parse(&table_str);

        let results: Vec<Map<String, Value>> = raw
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_list_table() {
        let input = "Package    Version\n---------- -------\nfoo        1.0\nbar        2.0\n";
        let parser = PipListParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].get("package"), Some(&Value::String("foo".into())));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_pip_list_legacy() {
        let input = "foo (1.0)\nbar (2.0)\n";
        let parser = PipListParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].get("package"), Some(&Value::String("foo".into())));
            assert_eq!(arr[0].get("version"), Some(&Value::String("1.0".into())));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_pip_list_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/ubuntu-18.04/pip-list.out");
        let fixture_json = include_str!("../../../../tests/fixtures/ubuntu-18.04/pip-list.json");

        let parser = PipListParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "pip_list fixture mismatch");
    }
}

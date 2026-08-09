//! Parser for `wc` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct WcParser;

static INFO: ParserInfo = ParserInfo {
    name: "wc",
    argument: "--wc",
    version: "1.4.0",
    description: "Converts `wc` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["wc"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static WC_PARSER: WcParser = WcParser;

inventory::submit! {
    ParserEntry::new(&WC_PARSER)
}

impl Parser for WcParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_wc(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Parse `wc` output.
///
/// Each line has: [lines] [words] [characters/bytes] [optional_filename]
/// The filename can contain spaces (it's the last field, capturing the rest).
///
/// The "total" line has "total" as the filename when multiple files are given.
fn parse_wc(input: &str) -> Vec<Map<String, Value>> {
    let mut output = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();

        let lines_val: Option<i64> = parts.next().and_then(|s| s.parse().ok());
        let words_val: Option<i64> = parts.next().and_then(|s| s.parse().ok());
        let chars_val: Option<i64> = parts.next().and_then(|s| s.parse().ok());

        // Remaining parts form the filename (may contain spaces)
        let filename_parts: Vec<&str> = parts.collect();
        let filename = if filename_parts.is_empty() {
            None
        } else {
            Some(filename_parts.join(" "))
        };

        let mut record = Map::new();
        match filename {
            Some(name) => record.insert("filename".to_string(), Value::String(name)),
            None => record.insert("filename".to_string(), Value::Null),
        };
        if let Some(n) = lines_val {
            record.insert("lines".to_string(), Value::Number(n.into()));
        }
        if let Some(n) = words_val {
            record.insert("words".to_string(), Value::Number(n.into()));
        }
        if let Some(n) = chars_val {
            record.insert("characters".to_string(), Value::Number(n.into()));
        }

        output.push(record);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wc_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/wc.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/wc.json"
        ))
        .unwrap();

        let parser = WcParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_wc_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/wc.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/wc.json"
        ))
        .unwrap();

        let parser = WcParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

//! Parser for `history` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct HistoryParser;

static INFO: ParserInfo = ParserInfo {
    name: "history",
    argument: "--history",
    version: "1.7.0",
    description: "Converts `history` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HISTORY_PARSER: HistoryParser = HistoryParser;

inventory::submit! {
    ParserEntry::new(&HISTORY_PARSER)
}

impl Parser for HistoryParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Use trim_start to strip leading indent, but preserve any
            // trailing spaces that are part of the recorded command.
            let line = line.trim_start();
            let mut parts = line.splitn(2, char::is_whitespace);
            let num_str = match parts.next() {
                Some(s) => s.trim(),
                None => continue,
            };
            let cmd = match parts.next() {
                Some(s) => s.trim_start_matches(' '),
                None => continue,
            };

            // Must be a numeric line number
            if let Some(line_num) = convert_to_int(num_str) {
                let mut entry = Map::new();
                entry.insert("line".to_string(), Value::Number(line_num.into()));
                entry.insert("command".to_string(), Value::String(cmd.to_string()));
                result.push(entry);
            }
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/history.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/history.json"
        ))
        .unwrap();
        let parser = HistoryParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len());
            for (got, exp) in arr.iter().zip(expected.iter()) {
                assert_eq!(got["line"], exp["line"]);
                assert_eq!(got["command"], exp["command"]);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_history_ubuntu() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/history.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/history.json"
        ))
        .unwrap();
        let parser = HistoryParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len());
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_history_empty() {
        let parser = HistoryParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

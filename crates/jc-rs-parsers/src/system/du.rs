//! Parser for `du` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct DuParser;

static INFO: ParserInfo = ParserInfo {
    name: "du",
    argument: "--du",
    version: "1.1.0",
    description: "Converts `du` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["du"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DU_PARSER: DuParser = DuParser;

inventory::submit! {
    ParserEntry::new(&DU_PARSER)
}

impl Parser for DuParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_du(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_du(input: &str) -> Vec<Map<String, Value>> {
    let mut output = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // du output: "size\tname" or "size  name" (tab or whitespace separated)
        // The size is first, then whitespace, then the rest is the filename
        let mut parts = line.splitn(2, char::is_whitespace);
        let size_str = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let name = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };

        let size: Value = if let Ok(n) = size_str.parse::<i64>() {
            Value::Number(n.into())
        } else {
            Value::String(size_str.to_string())
        };

        let mut record = Map::new();
        record.insert("size".to_string(), size);
        record.insert("name".to_string(), Value::String(name));
        output.push(record);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_du_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/du.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/du.json"
        ))
        .unwrap();

        let parser = DuParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_du_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/du.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/du.json"
        ))
        .unwrap();

        let parser = DuParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_du_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/du.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/du.json"
        ))
        .unwrap();

        let parser = DuParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

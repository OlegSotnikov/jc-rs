//! Parser for `find` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct FindParser;

static INFO: ParserInfo = ParserInfo {
    name: "find",
    argument: "--find",
    version: "1.0.0",
    description: "`find` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FIND_PARSER: FindParser = FindParser;

inventory::submit! {
    ParserEntry::new(&FIND_PARSER)
}

impl Parser for FindParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            if line.is_empty() {
                continue;
            }

            let mut obj = Map::new();

            if line == "." {
                // Special case: single dot means current directory node only
                obj.insert("path".to_string(), Value::Null);
                obj.insert("node".to_string(), Value::String(".".to_string()));
            } else if line.starts_with("find: ") {
                // Error line
                obj.insert("path".to_string(), Value::Null);
                obj.insert("node".to_string(), Value::Null);
                obj.insert("error".to_string(), Value::String(line.to_string()));
            } else {
                // Split on last '/'
                match line.rsplit_once('/') {
                    Some((path, node)) => {
                        if path.is_empty() {
                            obj.insert("path".to_string(), Value::Null);
                        } else {
                            obj.insert("path".to_string(), Value::String(path.to_string()));
                        }
                        if node.is_empty() {
                            obj.insert("node".to_string(), Value::Null);
                        } else {
                            obj.insert("node".to_string(), Value::String(node.to_string()));
                        }
                    }
                    None => {
                        obj.insert("path".to_string(), Value::Null);
                        obj.insert("node".to_string(), Value::Null);
                    }
                }
            }

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = FindParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();

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
    fn test_find_centos() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/centos-7.7/find.out"),
            include_str!("../../../../tests/fixtures/centos-7.7/find.json"),
        );
    }

    #[test]
    fn test_find_ubuntu() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-18.04/find.out"),
            include_str!("../../../../tests/fixtures/ubuntu-18.04/find.json"),
        );
    }

    #[test]
    fn test_find_empty() {
        let parser = FindParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

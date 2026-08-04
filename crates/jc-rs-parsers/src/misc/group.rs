//! Parser for `/etc/group` file format.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct GroupParser;

static INFO: ParserInfo = ParserInfo {
    name: "group",
    argument: "--group",
    version: "1.5.0",
    description: "Converts `/etc/group` file content to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static GROUP_PARSER: GroupParser = GroupParser;

inventory::submit! {
    ParserEntry::new(&GROUP_PARSER)
}

impl Parser for GroupParser {
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

            // Format: group_name:password:gid:members
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() < 3 {
                continue;
            }

            let members: Vec<Value> = if parts.len() >= 4 && !parts[3].is_empty() {
                parts[3]
                    .split(',')
                    .map(|m| Value::String(m.trim().to_string()))
                    .collect()
            } else {
                vec![]
            };

            let mut obj = Map::new();
            obj.insert(
                "group_name".to_string(),
                Value::String(parts[0].to_string()),
            );
            obj.insert("password".to_string(), Value::String(parts[1].to_string()));
            obj.insert(
                "gid".to_string(),
                convert_to_int(parts[2])
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            obj.insert("members".to_string(), Value::Array(members));
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/group.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/group.json"
        ))
        .unwrap();
        let parser = GroupParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    got["group_name"], exp["group_name"],
                    "group_name mismatch at row {}",
                    i
                );
                assert_eq!(got["gid"], exp["gid"], "gid mismatch at row {}", i);
                assert_eq!(
                    got["members"], exp["members"],
                    "members mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_group_empty() {
        let parser = GroupParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

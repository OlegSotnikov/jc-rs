//! Parser for `/etc/passwd` file format.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct PasswdParser;

static INFO: ParserInfo = ParserInfo {
    name: "passwd",
    argument: "--passwd",
    version: "1.5.0",
    description: "Converts `/etc/passwd` file content to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PASSWD_PARSER: PasswdParser = PasswdParser;

inventory::submit! {
    ParserEntry::new(&PASSWD_PARSER)
}

impl Parser for PasswdParser {
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

            // Format: username:password:uid:gid:comment:home:shell
            let parts: Vec<&str> = line.splitn(7, ':').collect();
            if parts.len() < 7 {
                continue;
            }

            let mut obj = Map::new();
            obj.insert("username".to_string(), Value::String(parts[0].to_string()));
            obj.insert("password".to_string(), Value::String(parts[1].to_string()));
            obj.insert(
                "uid".to_string(),
                convert_to_int(parts[2])
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "gid".to_string(),
                convert_to_int(parts[3])
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            obj.insert("comment".to_string(), Value::String(parts[4].to_string()));
            obj.insert("home".to_string(), Value::String(parts[5].to_string()));
            obj.insert("shell".to_string(), Value::String(parts[6].to_string()));
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passwd_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/passwd.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/passwd.json"
        ))
        .unwrap();
        let parser = PasswdParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    got["username"], exp["username"],
                    "username mismatch at row {}",
                    i
                );
                assert_eq!(got["uid"], exp["uid"], "uid mismatch at row {}", i);
                assert_eq!(got["gid"], exp["gid"], "gid mismatch at row {}", i);
                assert_eq!(got["home"], exp["home"], "home mismatch at row {}", i);
                assert_eq!(got["shell"], exp["shell"], "shell mismatch at row {}", i);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_passwd_empty() {
        let parser = PasswdParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

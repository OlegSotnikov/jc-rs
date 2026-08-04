//! Parser for `/etc/fstab` file format.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct FstabParser;

static INFO: ParserInfo = ParserInfo {
    name: "fstab",
    argument: "--fstab",
    version: "1.5.0",
    description: "Converts `/etc/fstab` file content to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FSTAB_PARSER: FstabParser = FstabParser;

inventory::submit! {
    ParserEntry::new(&FSTAB_PARSER)
}

impl Parser for FstabParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            // Strip inline comments (anything after #)
            let line = if let Some(pos) = line.find('#') {
                &line[..pos]
            } else {
                line
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let mut obj = Map::new();
            obj.insert("fs_spec".to_string(), Value::String(parts[0].to_string()));
            obj.insert("fs_file".to_string(), Value::String(parts[1].to_string()));
            obj.insert(
                "fs_vfstype".to_string(),
                Value::String(parts[2].to_string()),
            );
            obj.insert("fs_mntops".to_string(), Value::String(parts[3].to_string()));
            obj.insert(
                "fs_freq".to_string(),
                parts
                    .get(4)
                    .and_then(|s| convert_to_int(s))
                    .map(Value::from)
                    .unwrap_or(Value::Number(0.into())),
            );
            obj.insert(
                "fs_passno".to_string(),
                parts
                    .get(5)
                    .and_then(|s| convert_to_int(s))
                    .map(Value::from)
                    .unwrap_or(Value::Number(0.into())),
            );
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fstab_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/fstab.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/fstab.json"
        ))
        .unwrap();
        let parser = FstabParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    got["fs_spec"], exp["fs_spec"],
                    "fs_spec mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["fs_file"], exp["fs_file"],
                    "fs_file mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["fs_vfstype"], exp["fs_vfstype"],
                    "fs_vfstype mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["fs_freq"], exp["fs_freq"],
                    "fs_freq mismatch at row {}",
                    i
                );
                assert_eq!(
                    got["fs_passno"], exp["fs_passno"],
                    "fs_passno mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_fstab_empty() {
        let parser = FstabParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

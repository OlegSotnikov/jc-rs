//! Parser for `file` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct FileParser;

static INFO: ParserInfo = ParserInfo {
    name: "file",
    argument: "--file",
    version: "1.5.0",
    description: "Converts `file` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["file"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FILE_PARSER: FileParser = FileParser;

inventory::submit! {
    ParserEntry::new(&FILE_PARSER)
}

impl Parser for FileParser {
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

            // Special case for gzip files where description contains ': ' delimiter
            let parts = if line.contains("gzip compressed data, last modified: ") {
                line.splitn(2, ": ").collect::<Vec<_>>()
            } else {
                // Use rsplit to correctly handle filenames containing ': '
                line.rsplitn(2, ": ").collect::<Vec<_>>()
            };

            if parts.len() == 2 {
                let (filename, filetype) = if line.contains("gzip compressed data, last modified: ")
                {
                    (parts[0].trim(), parts[1].trim())
                } else {
                    // rsplitn reverses the order
                    (parts[1].trim(), parts[0].trim())
                };

                let mut entry = Map::new();
                entry.insert("filename".to_string(), Value::String(filename.to_string()));
                entry.insert("type".to_string(), Value::String(filetype.to_string()));
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
    fn test_file_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/file.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/file.json"
        ))
        .unwrap();
        let parser = FileParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len());
            for (got, exp) in arr.iter().zip(expected.iter()) {
                assert_eq!(got["filename"], exp["filename"], "filename mismatch");
                assert_eq!(got["type"], exp["type"], "type mismatch");
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_file_ubuntu() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/file.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/file.json"
        ))
        .unwrap();
        let parser = FileParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len());
            for (got, exp) in arr.iter().zip(expected.iter()) {
                assert_eq!(got["filename"], exp["filename"]);
                assert_eq!(got["type"], exp["type"]);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_file_empty() {
        let parser = FileParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

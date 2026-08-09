//! POSIX path list string parser for colon-separated PATH-style lists.

use super::path::parse_path_str;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::Value;

struct PathListParser;

static PATH_LIST_INFO: ParserInfo = ParserInfo {
    name: "path_list",
    argument: "--path-list",
    version: "1.0.0",
    description: "POSIX path list string parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

impl Parser for PathListParser {
    fn info(&self) -> &'static ParserInfo {
        &PATH_LIST_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // Detect separator: Windows paths use ';', POSIX uses ':'
        let delimiter = if input.contains('\\') { ';' } else { ':' };

        let entries: Vec<serde_json::Map<String, Value>> = input
            .split(delimiter)
            .filter(|s| !s.trim().is_empty())
            .map(|s| parse_path_str(s.trim()))
            .collect();

        Ok(ParseOutput::Array(entries))
    }
}

static PATH_LIST_PARSER_INSTANCE: PathListParser = PathListParser;

inventory::submit! {
    ParserEntry::new(&PATH_LIST_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;
    use std::fs;

    fn parse_to_array(input: &str) -> Vec<serde_json::Value> {
        let parser = PathListParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => arr.into_iter().map(serde_json::Value::Object).collect(),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_path_list_basic() {
        let arr = parse_to_array("/abc/def/gh.txt:/xyz/uvw/ab.app");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["path"], "/abc/def/gh.txt");
        assert_eq!(arr[0]["extension"], "txt");
        assert_eq!(arr[1]["path"], "/xyz/uvw/ab.app");
        assert_eq!(arr[1]["extension"], "app");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_list_one_fixture() {
        let out_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path_list--one.out"
        );
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path_list--one.json"
        );
        if let (Ok(input), Ok(expected_json)) =
            (fs::read_to_string(out_path), fs::read_to_string(json_path))
        {
            let arr = parse_to_array(input.trim());
            let expected: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
            let result = serde_json::Value::Array(arr);
            assert_eq!(result, expected);
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_list_two_fixture() {
        let out_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path_list--two.out"
        );
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path_list--two.json"
        );
        if let (Ok(input), Ok(expected_json)) =
            (fs::read_to_string(out_path), fs::read_to_string(json_path))
        {
            let arr = parse_to_array(input.trim());
            let expected: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
            let result = serde_json::Value::Array(arr);
            assert_eq!(result, expected);
        }
    }
}

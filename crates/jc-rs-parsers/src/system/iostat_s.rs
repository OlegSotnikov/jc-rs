//! Streaming parser for `iostat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::iostat::parse_iostat;

pub struct IostatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "iostat_s",
    argument: "--iostat-s",
    version: "1.1.0",
    description: "Streaming `iostat` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static IOSTAT_S_PARSER: IostatSParser = IostatSParser;

inventory::submit! {
    ParserEntry::new(&IOSTAT_S_PARSER)
}

impl Parser for IostatSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_iostat(input);
        Ok(ParseOutput::Array(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iostat_s_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iostat-streaming.json"
        ))
        .unwrap();
        let parser = IostatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_iostat_s_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/iostat-streaming.json"
        ))
        .unwrap();
        let parser = IostatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

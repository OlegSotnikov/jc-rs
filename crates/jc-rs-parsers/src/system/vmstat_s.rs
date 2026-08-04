//! Streaming parser for `vmstat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::vmstat::parse_vmstat;

pub struct VmstatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "vmstat_s",
    argument: "--vmstat-s",
    version: "1.3.0",
    description: "Streaming `vmstat` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static VMSTAT_S_PARSER: VmstatSParser = VmstatSParser;

inventory::submit! {
    ParserEntry::new(&VMSTAT_S_PARSER)
}

impl Parser for VmstatSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_vmstat(input);
        Ok(ParseOutput::Array(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmstat_s_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat-streaming.json"
        ))
        .unwrap();
        let parser = VmstatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_vmstat_s_a_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat-a-streaming.json"
        ))
        .unwrap();
        let parser = VmstatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

//! Streaming parser for `top -b` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::top::TopSession;

pub struct TopSParser;

static INFO: ParserInfo = ParserInfo {
    name: "top_s",
    argument: "--top-s",
    version: "1.3.0",
    description: "Streaming `top -b` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static TOP_S_PARSER: TopSParser = TopSParser;

inventory::submit! {
    ParserEntry::new(&TOP_S_PARSER)
}

impl Parser for TopSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for TopSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(TopSession::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_s_centos_b_n3() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/top-b-n3.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/top-b-n3.json"
        ))
        .unwrap();
        let parser = TopSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

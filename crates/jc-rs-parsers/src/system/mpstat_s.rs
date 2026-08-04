//! Streaming parser for `mpstat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::mpstat::MpstatSession;

pub struct MpstatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "mpstat_s",
    argument: "--mpstat-s",
    version: "1.1.0",
    description: "Streaming `mpstat` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static MPSTAT_S_PARSER: MpstatSParser = MpstatSParser;

inventory::submit! {
    ParserEntry::new(&MPSTAT_S_PARSER)
}

impl Parser for MpstatSParser {
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

impl StreamingParser for MpstatSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(MpstatSession::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpstat_s_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/mpstat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/mpstat-streaming.json"
        ))
        .unwrap();
        let parser = MpstatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

//! Streaming parser for `pidstat -H` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::pidstat::PidstatSession;

pub struct PidstatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "pidstat_s",
    argument: "--pidstat-s",
    version: "1.1.0",
    description: "Streaming `pidstat -H` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static PIDSTAT_S_PARSER: PidstatSParser = PidstatSParser;

inventory::submit! {
    ParserEntry::new(&PIDSTAT_S_PARSER)
}

impl Parser for PidstatSParser {
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

impl StreamingParser for PidstatSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(PidstatSession::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pidstat_s_hl_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/pidstat-hl.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/pidstat-hl-streaming.json"
        ))
        .unwrap();
        let parser = PidstatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

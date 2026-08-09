//! TSV file streaming parser.

use super::delimited::DelimitedSession;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct TsvSParser;

static INFO: ParserInfo = ParserInfo {
    name: "tsv_s",
    argument: "--tsv-s",
    version: "1.0.0",
    description: "TSV file streaming parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static PARSER: TsvSParser = TsvSParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for TsvSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for TsvSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(DelimitedSession::new(true, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_s_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/tsv-dpkg-query.tsv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/tsv-dpkg-query-streaming.json"
        ))
        .unwrap();
        let result = TsvSParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

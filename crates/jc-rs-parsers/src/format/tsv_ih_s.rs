//! TSV streaming parser for files with no header row.

use super::delimited::{DelimitedSession, parse_delimited};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct TsvIhSParser;

static INFO: ParserInfo = ParserInfo {
    name: "tsv_ih_s",
    argument: "--tsv-ih-s",
    version: "1.0.0",
    description: "TSV file streaming parser without implicit header row",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Universal],
    tags: &[Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static PARSER: TsvIhSParser = TsvIhSParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for TsvIhSParser {
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

impl StreamingParser for TsvIhSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(DelimitedSession::new(true, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_ih_s_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/tsv_ih-dpkg-query.tsv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/tsv_ih-dpkg-query-streaming.json"
        ))
        .unwrap();
        let result = TsvIhSParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

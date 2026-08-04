//! CSV streaming parser for files with no header row.

use super::delimited::{DelimitedSession, parse_delimited};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct CsvIhSParser;

static INFO: ParserInfo = ParserInfo {
    name: "csv_ih_s",
    argument: "--csv-ih-s",
    version: "1.0.0",
    description: "CSV file streaming parser without implicit header row",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Universal],
    tags: &[Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static PARSER: CsvIhSParser = CsvIhSParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for CsvIhSParser {
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

impl StreamingParser for CsvIhSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(DelimitedSession::new(false, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_ih_s_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/csv_ih-homes.csv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/csv_ih-homes-streaming.json"
        ))
        .unwrap();
        let result = CsvIhSParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

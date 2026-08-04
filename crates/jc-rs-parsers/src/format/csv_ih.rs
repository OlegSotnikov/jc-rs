//! CSV file parser for files with no header row.

use super::delimited::{DelimitedSession, parse_delimited};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct CsvIhParser;

static INFO: ParserInfo = ParserInfo {
    name: "csv_ih",
    argument: "--csv-ih",
    version: "1.0.0",
    description: "CSV file parser without implicit header row",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PARSER: CsvIhParser = CsvIhParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for CsvIhParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        parse_delimited(input, false, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_ih_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/csv_ih-homes.csv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/csv_ih-homes.json"
        ))
        .unwrap();
        let result = CsvIhParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

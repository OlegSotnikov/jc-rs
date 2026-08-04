//! TSV file parser.

use super::delimited::{DelimitedSession, parse_delimited};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct TsvParser;

static INFO: ParserInfo = ParserInfo {
    name: "tsv",
    argument: "--tsv",
    version: "1.0.0",
    description: "TSV file parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PARSER: TsvParser = TsvParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for TsvParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        parse_delimited(input, true, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/tsv-dpkg-query.tsv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/tsv-dpkg-query.json"
        ))
        .unwrap();
        let result = TsvParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

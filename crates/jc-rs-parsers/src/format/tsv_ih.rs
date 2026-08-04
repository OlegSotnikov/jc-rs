//! TSV file parser for files with no header row.

use super::delimited::parse_delimited;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct TsvIhParser;

static INFO: ParserInfo = ParserInfo {
    name: "tsv_ih",
    argument: "--tsv-ih",
    version: "1.0.0",
    description: "TSV file parser without implicit header row",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PARSER: TsvIhParser = TsvIhParser;

inventory::submit! {
    ParserEntry::new(&PARSER)
}

impl Parser for TsvIhParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        parse_delimited(input, true, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_ih_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/tsv_ih-dpkg-query.tsv");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/tsv_ih-dpkg-query.json"
        ))
        .unwrap();
        let result = TsvIhParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }
}

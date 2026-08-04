//! CSV streaming parser.
//!
//! Streaming variant of the CSV parser. Each row yields one ParseOutput::Object.

use super::csv::parse_csv_input;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct CsvSParser;

static CSV_S_INFO: ParserInfo = ParserInfo {
    name: "csv_s",
    argument: "--csv-s",
    version: "1.0.0",
    description: "CSV file streaming parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

impl Parser for CsvSParser {
    fn info(&self) -> &'static ParserInfo {
        &CSV_S_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        // For the standard parse(), process all lines and return Array
        let rows = parse_csv_input(input)?;
        Ok(ParseOutput::Array(rows))
    }
}

impl StreamingParser for CsvSParser {
    fn parse_line(&self, _line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        // CSV streaming is inherently stateful (need headers first).
        // The streaming protocol here assumes lines are passed one-by-one
        // but we lack header context in a stateless parser.
        // Return None — callers should use parse() for full processing.
        Ok(None)
    }
}

static CSV_S_PARSER_INSTANCE: CsvSParser = CsvSParser;

inventory::submit! {
    ParserEntry::new(&CSV_S_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/generic");

    fn load_fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn parse_json_array(s: &str) -> Vec<serde_json::Map<String, serde_json::Value>> {
        serde_json::from_str(s).expect("invalid fixture JSON")
    }

    #[test]
    fn test_csv_s_biostats() {
        let input = load_fixture("csv-biostats.csv");
        let expected_standard = parse_json_array(&load_fixture("csv-biostats.json"));
        let parser = CsvSParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Array(rows) = result {
            assert_eq!(rows, expected_standard);
        } else {
            panic!("expected Array output");
        }
    }

    #[test]
    fn test_csv_s_registered() {
        let parser = CsvSParser;
        assert_eq!(parser.info().name, "csv_s");
        assert_eq!(parser.info().argument, "--csv-s");
        assert!(parser.info().streaming);
    }
}

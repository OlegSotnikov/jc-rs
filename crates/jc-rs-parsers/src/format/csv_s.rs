//! CSV streaming parser.
//!
//! Streaming variant of the CSV parser: one record per row, emitted as soon as
//! the row is complete. The row splitting itself lives in `delimited`, shared
//! with the tsv and implicit-header variants.

use super::delimited::DelimitedSession;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
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

impl StreamingParser for CsvSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(DelimitedSession::new(false, false))
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

    #[test]
    fn test_csv_s_emits_row_as_soon_as_it_is_complete() {
        let mut session = CsvSParser.session();
        assert!(session.parse_line("a,b", false).unwrap().is_none()); // header
        let record = session.parse_line("1,2", false).unwrap().unwrap();
        assert_eq!(record["a"], "1");
        assert_eq!(record["b"], "2");
    }

    #[test]
    fn test_csv_s_multiline_quoted_field() {
        let mut session = CsvSParser.session();
        session.parse_line("a,b", false).unwrap();
        // The row is not complete until the quote closes.
        assert!(session.parse_line("1,\"line one", false).unwrap().is_none());
        let record = session.parse_line("line two\"", false).unwrap().unwrap();
        assert_eq!(record["b"], "line one\nline two");
    }
}

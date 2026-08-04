//! CSV streaming parser.
//!
//! Streaming variant of the CSV parser: one record per row, emitted as soon as
//! the row is complete. A row is not always a line -- a quoted field may span
//! several -- so the session buffers until the quotes balance.

use super::csv::{detect_delimiter, normalize_csv_line, strip_bom};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::Value;

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
        Box::new(CsvSession::default())
    }
}

/// State a CSV row needs: the delimiter and header names taken from the first
/// row, and whatever part of a multi-line quoted row has arrived so far.
#[derive(Default)]
struct CsvSession {
    delimiter: Option<u8>,
    headers: Vec<String>,
    pending: String,
}

impl CsvSession {
    /// A record is complete once every quote it opened has been closed.
    fn quotes_balanced(&self) -> bool {
        self.pending.bytes().filter(|&b| b == b'"').count() % 2 == 0
    }

    fn take_record(&mut self) -> Option<Record> {
        let delimiter = *self
            .delimiter
            .get_or_insert_with(|| detect_delimiter(&self.pending));
        let text = std::mem::take(&mut self.pending);
        let fields = split_record(&normalize_csv_line(&text, delimiter), delimiter);

        if self.headers.is_empty() {
            self.headers = fields;
            return None;
        }

        let mut record = Record::with_capacity(fields.len());
        for (i, field) in fields.into_iter().enumerate() {
            let key = match self.headers.get(i) {
                Some(name) => name.clone(),
                None => format!("col{i}"),
            };
            record.insert(key, Value::String(field));
        }
        Some(record)
    }
}

impl LineParser for CsvSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        let line = if self.headers.is_empty() && self.pending.is_empty() {
            strip_bom(line)
        } else {
            line
        };

        // Blank lines between records are not records (the `csv` crate skips
        // them too); inside a quoted field they are content.
        if self.pending.is_empty() && line.trim().is_empty() {
            return Ok(None);
        }

        if !self.pending.is_empty() {
            self.pending.push('\n');
        }
        self.pending.push_str(line);

        if !self.quotes_balanced() {
            return Ok(None);
        }
        Ok(self.take_record())
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        // An unterminated quote at EOF: emit what we have rather than drop it.
        if self.pending.is_empty() {
            return Ok(None);
        }
        Ok(self.take_record())
    }
}

/// Split one complete CSV record into fields, honouring RFC 4180 quoting
/// (a doubled quote inside a quoted field is a literal quote).
fn split_record(text: &str, delimiter: u8) -> Vec<String> {
    let delim = delimiter as char;
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            c if c == delim && !in_quotes => fields.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    fields.push(field);
    fields
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

    #[test]
    fn test_csv_s_doubled_quote_is_literal() {
        let fields = split_record(r#"a,"say ""hi""",c"#, b',');
        assert_eq!(fields, vec!["a", r#"say "hi""#, "c"]);
    }
}

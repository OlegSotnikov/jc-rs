//! Shared machinery for the delimited-text parsers: `csv`, `tsv`, and their
//! implicit-header and streaming variants.
//!
//! jc builds all six on one `csv.DictReader` call with two flags (a tab
//! delimiter and an implicit header), so they share one implementation here
//! too, and a fix to the row splitter lands in all of them at once.

use jc_rs_core::error::ParseError;
use jc_rs_core::traits::{LineParser, Record};
use jc_rs_core::types::ParseOutput;
use serde_json::Value;

use super::csv::{detect_delimiter, normalize_csv_line, strip_bom};

/// How much input jc's `csv_s` hands to `csv.Sniffer`. More importantly, it is
/// also the window it checks for `""` when deciding whether a doubled quote is
/// an escaped quote or two literal ones.
const SNIFF_LIMIT: usize = 1024;

/// A delimited-text parse in progress.
///
/// A row is not always a line: a quoted field may span several, so the session
/// accumulates until the quotes balance.
pub(crate) struct DelimitedSession {
    implicit_header: bool,
    delimiter: Option<u8>,
    headers: Vec<String>,
    pending: String,
    /// The leading slice of input jc would have sniffed. Only its `""` content
    /// matters, so it stops growing at [`SNIFF_LIMIT`].
    sniff: String,
    first_line: bool,
}

impl DelimitedSession {
    pub(crate) fn new(tsv: bool, implicit_header: bool) -> Self {
        Self {
            implicit_header,
            delimiter: tsv.then_some(b'\t'),
            headers: Vec::new(),
            pending: String::new(),
            sniff: String::new(),
            first_line: true,
        }
    }

    /// Pre-load the whole input into the sniff window.
    ///
    /// The batch parsers test `'""' in data` over the entire file, while the
    /// streaming ones only see the first 1024 characters. That single
    /// difference is why `csv` and `csv_s` disagree on jc's own
    /// `csv-doublequoted` fixture, and it has to be preserved, not smoothed
    /// over: jc is the authority for both.
    fn sniff_all(&mut self, input: &str) {
        self.sniff = input.to_string();
    }

    /// Whether `""` inside a quoted field means one escaped quote.
    ///
    /// Decided from the sniff window at the moment it is needed, which gives
    /// the same answer as jc without buffering 100 lines first: a `""` within
    /// the window is already in it by the time we reach it, and one beyond the
    /// window cannot change an answer the window has already fixed.
    fn doublequote(&self) -> bool {
        self.sniff.contains("\"\"")
    }

    fn feed_sniff(&mut self, line: &str) {
        if self.sniff.len() >= SNIFF_LIMIT {
            return;
        }
        if !self.sniff.is_empty() {
            self.sniff.push_str("\r\n");
        }
        self.sniff.push_str(line);
        self.sniff.truncate(
            self.sniff
                .char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= SNIFF_LIMIT)
                .unwrap_or(self.sniff.len()),
        );
    }

    fn quotes_balanced(&self) -> bool {
        self.pending.bytes().filter(|&b| b == b'"').count() % 2 == 0
    }

    fn take_record(&mut self) -> Option<Record> {
        let delimiter = *self
            .delimiter
            .get_or_insert_with(|| detect_delimiter(&self.pending));
        let doublequote = self.doublequote();
        let text = std::mem::take(&mut self.pending);
        let fields = split_record(
            &normalize_csv_line(&text, delimiter),
            delimiter,
            doublequote,
        );

        if !self.implicit_header && self.headers.is_empty() {
            self.headers = fields;
            return None;
        }

        let mut record = Record::with_capacity(fields.len());
        for (i, field) in fields.into_iter().enumerate() {
            let key = match self.headers.get(i) {
                Some(name) => name.clone(),
                // jc names implicit columns c0, c1, ...; a row longer than the
                // header falls back to the same scheme.
                None => format!("c{i}"),
            };
            record.insert(key, Value::String(field));
        }
        Some(record)
    }
}

impl LineParser for DelimitedSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        let line = if self.first_line {
            self.first_line = false;
            strip_bom(line)
        } else {
            line
        };
        self.feed_sniff(line);

        // Blank lines between records are not records; inside a quoted field
        // they are content.
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

/// Parse a complete delimited document.
pub(crate) fn parse_delimited(
    input: &str,
    tsv: bool,
    implicit_header: bool,
) -> Result<ParseOutput, ParseError> {
    if input.trim().is_empty() {
        return Err(ParseError::InvalidInput("empty input".to_string()));
    }

    let mut session = DelimitedSession::new(tsv, implicit_header);
    session.sniff_all(input);

    let mut records = Vec::new();
    for line in input.lines() {
        if let Some(record) = session.parse_line(line, true)? {
            records.push(record);
        }
    }
    if let Some(record) = session.finalize(true)? {
        records.push(record);
    }

    Ok(ParseOutput::Array(records))
}

/// Split one complete row into fields.
///
/// With `doublequote` on, `""` inside a quoted field is one literal quote
/// (RFC 4180). With it off, the first quote simply ends the quoted section and
/// everything after it is taken literally, which is what Python's `csv`
/// module does, and therefore what jc emits.
fn split_record(text: &str, delimiter: u8, doublequote: bool) -> Vec<String> {
    let delim = delimiter as char;
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && doublequote => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' if in_quotes => in_quotes = false,
            '"' if field.is_empty() => in_quotes = true,
            // A quote in the middle of an unquoted field is just a character.
            '"' => field.push('"'),
            c if c == delim && !in_quotes => fields.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    fields.push(field);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubled_quote_is_one_quote_when_doublequote_is_on() {
        let fields = split_record(r#"a,"say ""hi""",c"#, b',', true);
        assert_eq!(fields, vec!["a", r#"say "hi""#, "c"]);
    }

    #[test]
    fn doubled_quote_ends_the_quoted_section_when_doublequote_is_off() {
        // Python's reader leaves the trailing quote in place here, and jc's
        // csv_s inherits that whenever the `""` falls outside the sniff window.
        let fields = split_record(r#"2,"this is a field with "" in it""#, b',', false);
        assert_eq!(fields, vec!["2", r#"this is a field with " in it""#]);
    }

    #[test]
    fn tab_delimited_rows_split_on_tabs() {
        let out = parse_delimited("a\tb\n1\t2\n", true, false).unwrap();
        let ParseOutput::Array(rows) = out else {
            panic!("expected array")
        };
        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "2");
    }

    #[test]
    fn implicit_header_names_columns_c0_upwards_and_keeps_every_row() {
        let out = parse_delimited("a,b\n1,2\n", false, true).unwrap();
        let ParseOutput::Array(rows) = out else {
            panic!("expected array")
        };
        assert_eq!(rows.len(), 2, "no row is consumed as a header");
        assert_eq!(rows[0]["c0"], "a");
        assert_eq!(rows[1]["c1"], "2");
    }

    #[test]
    fn jagged_rows_keep_only_the_columns_they_have() {
        let out = parse_delimited("\"one\", \"two\"\n\"one\"\n", false, true).unwrap();
        let ParseOutput::Array(rows) = out else {
            panic!("expected array")
        };
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[1].len(), 1);
        assert_eq!(rows[1]["c0"], "one");
    }
}

//! CSV file parser.
//!
//! Parses CSV (comma-separated), TSV (tab-separated), and pipe-delimited files.
//! First row is treated as headers. All values are returned as strings.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct CsvParser;

static CSV_INFO: ParserInfo = ParserInfo {
    name: "csv",
    argument: "--csv",
    version: "1.0.0",
    description: "CSV file parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Strip UTF-8 BOM from input if present.
pub(crate) fn strip_bom(input: &str) -> &str {
    input.strip_prefix('\u{FEFF}').unwrap_or(input)
}

/// Detect the delimiter by checking the first line for common delimiters.
pub(crate) fn detect_delimiter(input: &str) -> u8 {
    let first_line = input.lines().next().unwrap_or("");
    // Count candidates; prefer pipe if found (since pipe files always have it),
    // then tab, then comma.
    let pipe_count = first_line.chars().filter(|&c| c == '|').count();
    let tab_count = first_line.chars().filter(|&c| c == '\t').count();
    let comma_count = first_line.chars().filter(|&c| c == ',').count();

    if pipe_count > 0 && pipe_count >= comma_count && pipe_count >= tab_count {
        b'|'
    } else if tab_count > 0 && tab_count >= comma_count {
        b'\t'
    } else {
        b','
    }
}

/// Normalize CSV input so quoted fields are recognized properly.
/// The csv spec requires quotes to start immediately after the delimiter.
/// Some CSV files have spaces between the delimiter and the quote — we normalize
/// those by stripping leading whitespace from unquoted fields only.
pub(crate) fn normalize_csv_line(line: &str, delimiter: u8) -> String {
    let delim_char = delimiter as char;
    let mut result = String::with_capacity(line.len());
    let mut in_quotes = false;
    let mut field_start = true;

    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            field_start = false;
            result.push(ch);
        } else if ch == delim_char && !in_quotes {
            result.push(ch);
            field_start = true;
        } else if field_start && ch == ' ' {
            // Skip leading whitespace in unquoted fields
            continue;
        } else {
            field_start = false;
            result.push(ch);
        }
    }
    result
}

pub fn parse_csv_input(
    input: &str,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, ParseError> {
    let input = strip_bom(input);

    if input.trim().is_empty() {
        return Err(ParseError::InvalidInput("empty input".to_string()));
    }

    let delimiter = detect_delimiter(input);

    // Normalize lines so quoted fields start immediately after delimiter
    let normalized: String = input
        .lines()
        .map(|line| normalize_csv_line(line, delimiter))
        .collect::<Vec<_>>()
        .join("\n");

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::None)
        .flexible(true)
        .from_reader(normalized.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| ParseError::Generic(format!("CSV header error: {e}")))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| ParseError::Generic(format!("CSV record error: {e}")))?;
        let mut map = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
            map.insert(key, serde_json::Value::String(field.to_string()));
        }
        rows.push(map);
    }

    Ok(rows)
}

impl Parser for CsvParser {
    fn info(&self) -> &'static ParserInfo {
        &CSV_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_csv_input(input)?;
        Ok(ParseOutput::Array(rows))
    }
}

static CSV_PARSER_INSTANCE: CsvParser = CsvParser;

inventory::submit! {
    ParserEntry::new(&CSV_PARSER_INSTANCE)
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
    fn test_csv_comma_biostats() {
        let input = load_fixture("csv-biostats.csv");
        let expected = parse_json_array(&load_fixture("csv-biostats.json"));
        let parser = CsvParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Array(rows) = result {
            assert_eq!(rows, expected);
        } else {
            panic!("expected Array output");
        }
    }

    #[test]
    fn test_csv_pipe() {
        let input = load_fixture("csv-homes-pipe.csv");
        let expected = parse_json_array(&load_fixture("csv-homes-pipe.json"));
        let parser = CsvParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Array(rows) = result {
            assert_eq!(rows, expected);
        } else {
            panic!("expected Array output");
        }
    }

    #[test]
    fn test_csv_tab_flyrna() {
        let input = load_fixture("csv-flyrna.tsv");
        let expected = parse_json_array(&load_fixture("csv-flyrna.json"));
        let parser = CsvParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Array(rows) = result {
            assert_eq!(rows, expected);
        } else {
            panic!("expected Array output");
        }
    }

    #[test]
    fn test_csv_utf8_bom() {
        let input = load_fixture("csv-utf-8-bom.csv");
        let expected = parse_json_array(&load_fixture("csv-utf-8-bom.json"));
        let parser = CsvParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Array(rows) = result {
            assert_eq!(rows, expected);
        } else {
            panic!("expected Array output");
        }
    }
}

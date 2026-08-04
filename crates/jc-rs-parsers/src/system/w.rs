//! Parser for `w` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct WParser;

static INFO: ParserInfo = ParserInfo {
    name: "w",
    argument: "--w",
    version: "1.5.0",
    description: "Converts `w` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["w"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static W_PARSER: WParser = WParser;

inventory::submit! {
    ParserEntry::new(&W_PARSER)
}

impl Parser for WParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_w(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Parse `w` output.
///
/// First line is uptime summary (skip it).
/// Second line is column headers.
/// Remaining lines are data rows.
///
/// The `from` column can be blank (no remote host), so we detect its byte
/// position in the header and insert "-" when blank.
fn parse_w(input: &str) -> Vec<Map<String, Value>> {
    let mut lines = input.lines();

    // Skip first line (uptime summary)
    lines.next();

    // Second line: column headers
    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    // Find the byte position of "FROM" header (case-insensitive)
    let header_lower = header_line.to_lowercase();
    let from_col: Option<usize> = header_lower.find("from");

    // Normalize header: lowercase, replace "login@" → "login_at"
    let header_text = header_lower.replace("login@", "login_at");
    let headers: Vec<String> = header_text
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let n_headers = headers.len();
    let has_from = headers.iter().any(|h| h == "from");

    let mut output = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        // Check if FROM column is blank in this line
        let mut insert_dash = false;
        if has_from {
            if let Some(col_pos) = from_col {
                // Check character at from_col position
                let ch = line.chars().nth(col_pos);
                insert_dash = ch.map(|c| c.is_whitespace()).unwrap_or(true);
            }
        }

        // Split line into at most n_headers parts (last captures "what" including spaces)
        // Use split_whitespace to collect tokens
        let tokens: Vec<&str> = line.split_whitespace().collect();

        // Build parts, potentially inserting "-" for blank from
        let mut parts: Vec<String> = if insert_dash {
            // Find where "from" is in headers
            let from_idx = headers.iter().position(|h| h == "from").unwrap_or(2);
            let mut p: Vec<String> = Vec::new();
            let mut ti = 0;
            for i in 0..n_headers {
                if i == from_idx {
                    p.push("-".to_string());
                } else if ti < tokens.len() {
                    if i == n_headers - 1 {
                        // Last column: take rest
                        let rest = tokens[ti..].join(" ");
                        p.push(rest);
                        ti = tokens.len();
                    } else {
                        p.push(tokens[ti].to_string());
                        ti += 1;
                    }
                }
            }
            p
        } else {
            // Normal split: n-1 whitespace splits, last captures rest
            let mut p: Vec<String> = Vec::new();
            let mut ti = 0;
            for i in 0..n_headers {
                if ti >= tokens.len() {
                    break;
                }
                if i == n_headers - 1 {
                    let rest = tokens[ti..].join(" ");
                    p.push(rest);
                    ti = tokens.len();
                } else {
                    p.push(tokens[ti].to_string());
                    ti += 1;
                }
            }
            p
        };

        let mut record = Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = parts
                .get(i)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            record.insert(header.clone(), Value::String(val));
        }

        let record = process_w_record(record);
        output.push(record);
    }

    output
}

/// Convert dash values and empty strings to null for specific fields.
fn process_w_record(mut record: Map<String, Value>) -> Map<String, Value> {
    let null_fields = ["user", "tty", "from", "login_at", "idle", "what"];
    for field in &null_fields {
        if let Some(Value::String(s)) = record.get(*field) {
            if s == "-" || s.is_empty() {
                record.insert(field.to_string(), Value::Null);
            }
        }
    }
    record
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_w_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/w.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/w.json"
        ))
        .unwrap();

        let parser = WParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_w_osx_10_11_6() {
        let input = include_str!("../../../../tests/fixtures/osx-10.11.6/w.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.11.6/w.json"
        ))
        .unwrap();

        let parser = WParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_w_osx_smoke() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/w.out");
        let parser = WParser;
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }
}

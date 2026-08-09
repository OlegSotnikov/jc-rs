//! Parser for `lsof` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::sparse_table_parse;
use serde_json::{Map, Value};

/// Strips all non-numeric characters (except `-` and `.`) and parses as integer.
/// Matches jc's `convert_to_int` behavior.
fn convert_to_int(s: &str) -> Option<i64> {
    let stripped: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-' || *c == '.')
        .collect();
    if stripped.is_empty() {
        return None;
    }
    stripped
        .parse::<i64>()
        .ok()
        .or_else(|| stripped.parse::<f64>().ok().map(|f| f as i64))
}

pub struct LsofParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsof",
    argument: "--lsof",
    version: "1.6.0",
    description: "Converts `lsof` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &["lsof"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSOF_PARSER: LsofParser = LsofParser;

inventory::submit! {
    ParserEntry::new(&LSOF_PARSER)
}

impl Parser for LsofParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Remove blank lines
        let clean: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        if clean.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Normalize header: lowercase and replace '/' with '_'
        let header = clean[0].to_lowercase().replace('/', "_");
        let rest = clean[1..].join("\n");
        let normalized = format!("{}\n{}", header, rest);

        let rows = sparse_table_parse(&normalized);

        let result = rows
            .into_iter()
            .map(|row| {
                let mut out = Map::new();

                for (k, v) in &row {
                    match v {
                        Value::Null => {
                            // Convert null to null for certain fields
                            out.insert(k.clone(), Value::Null);
                        }
                        Value::String(s) => {
                            let trimmed = s.trim();
                            // Convert integer fields
                            if k == "pid" || k == "tid" || k == "size_off" || k == "node" {
                                if trimmed.is_empty() {
                                    out.insert(k.clone(), Value::Null);
                                } else if let Some(n) = convert_to_int(trimmed) {
                                    out.insert(k.clone(), Value::Number(n.into()));
                                } else {
                                    out.insert(k.clone(), Value::Null);
                                }
                            } else {
                                out.insert(k.clone(), Value::String(trimmed.to_string()));
                            }
                        }
                        _ => {
                            out.insert(k.clone(), v.clone());
                        }
                    }
                }

                out
            })
            .collect();

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsof_basic() {
        let input = "COMMAND    PID  TID    USER   FD      TYPE DEVICE  SIZE/OFF     NODE NAME\nsystemd      1         root  cwd   unknown                           /proc/1/cwd\n";
        let parser = LsofParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(!arr.is_empty());
            assert_eq!(
                arr[0].get("command"),
                Some(&Value::String("systemd".to_string()))
            );
            assert_eq!(arr[0].get("pid"), Some(&Value::Number(1.into())));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsof_empty() {
        let parser = LsofParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

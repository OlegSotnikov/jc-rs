//! Parser for `/proc/mtrr`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcMtrrParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_mtrr",
    argument: "--proc-mtrr",
    version: "1.0.0",
    description: "Converts `/proc/mtrr` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/mtrr"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_MTRR_PARSER: ProcMtrrParser = ProcMtrrParser;

inventory::submit! { ParserEntry::new(&PROC_MTRR_PARSER) }

/// Extract integer from a string like "2048MB", "4kB", "0MB"
fn extract_size_num(s: &str) -> i64 {
    let s = s.trim();
    // Strip trailing unit
    let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().unwrap_or(0)
}

/// Extract integer from parenthetical like "(    0MB)" or "( 2048MB)"
fn extract_base_mb(s: &str) -> i64 {
    // s is like "    0MB" or " 2048MB"
    let s = s.trim();
    let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().unwrap_or(0)
}

impl Parser for ProcMtrrParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Extract register: everything before first ':'
            let Some(colon_pos) = line.find(':') else {
                continue;
            };
            let register = &line[..colon_pos];
            let rest = line[colon_pos + 1..].trim();

            // Extract base= value
            let Some(base_start) = rest.find("base=") else {
                continue;
            };
            let after_base = &rest[base_start + 5..];

            // base is hex string up to space or '('
            let base_end = after_base.find([' ', '(']).unwrap_or(after_base.len());
            let base = &after_base[..base_end];

            // Extract base_mb from parenthetical
            let base_mb = if let Some(paren_open) = after_base.find('(') {
                if let Some(paren_close) = after_base.find(')') {
                    let inner = &after_base[paren_open + 1..paren_close];
                    // inner is like "    0MB" or " 2048MB"
                    extract_base_mb(inner)
                } else {
                    0
                }
            } else {
                0
            };

            // Extract size= value
            let Some(size_start) = rest.find("size=") else {
                continue;
            };
            let after_size = &rest[size_start + 5..].trim_start();
            // size number followed by "MB" or "kB" then possible "," or ":"
            let size = extract_size_num(after_size);

            // Extract count= value
            let count = if let Some(count_start) = rest.find("count=") {
                let after_count = &rest[count_start + 6..];
                let num_str: String = after_count
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                num_str.parse::<i64>().unwrap_or(0)
            } else {
                0
            };

            // Extract type: it's the last word-like field after a ": " or ", "
            // Two formats:
            // Format 1: "count=1: write-back"  -> type is after "count=N: "
            // Format 2: "size= 256MB: write-back, count=1" -> type is between "MB: " and ", count"
            let mtrr_type = if let Some(count_pos) = rest.find("count=") {
                let after_count = &rest[count_pos..];
                if let Some(colon_in_count) = after_count.find(": ") {
                    // Format 1: type is after "count=N: "
                    after_count[colon_in_count + 2..].trim().to_string()
                } else {
                    // Format 2: type is between size unit and ", count"
                    // Find the portion after size's unit marker (MB:/kB:) and before ", count"
                    let before_count = &rest[..count_pos];
                    // type is the last segment, look for ": TYPE," or ": TYPE"
                    if let Some(last_colon) = before_count.rfind(": ") {
                        before_count[last_colon + 2..]
                            .trim()
                            .trim_end_matches(',')
                            .trim()
                            .to_string()
                    } else {
                        String::new()
                    }
                }
            } else {
                String::new()
            };

            let mut entry = Map::new();
            entry.insert("register".to_string(), Value::String(register.to_string()));
            entry.insert("type".to_string(), Value::String(mtrr_type));
            entry.insert("base".to_string(), Value::String(base.to_string()));
            entry.insert("base_mb".to_string(), Value::Number(base_mb.into()));
            entry.insert("size".to_string(), Value::Number(size.into()));
            entry.insert("count".to_string(), Value::Number(count.into()));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_mtrr() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/mtrr");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/mtrr.json"
        ))
        .unwrap();
        let parser = ProcMtrrParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

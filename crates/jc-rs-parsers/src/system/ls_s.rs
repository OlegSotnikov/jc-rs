//! Streaming parser for `ls -l` command output.
//!
//! This is a simplified streaming variant of the ls parser.
//! It requires `-l` to be used.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsSParser;

static INFO: ParserInfo = ParserInfo {
    name: "ls_s",
    argument: "--ls-s",
    version: "1.2.0",
    description: "Streaming `ls` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Aix,
    ],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static LS_S_PARSER: LsSParser = LsSParser;

inventory::submit! {
    ParserEntry::new(&LS_S_PARSER)
}

fn is_long_entry(line: &str) -> bool {
    let re_chars = "-dclpsbDCMnP?";
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0] as char;
    if !re_chars.contains(first) {
        return false;
    }
    if bytes.len() < 10 {
        return false;
    }
    bytes[1..10].iter().all(|&b| {
        let c = b as char;
        matches!(c, 'r' | 'w' | 'x' | '-' | 's' | 'S' | 't' | 'T' | '+')
    })
}

fn split_ls_long_line(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut remaining = line.trim_start();

    for i in 0..9 {
        if remaining.is_empty() {
            break;
        }
        if i == 8 {
            parts.push(remaining);
            break;
        }
        let token_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        parts.push(&remaining[..token_end]);
        remaining = remaining[token_end..].trim_start();
    }

    parts
}

impl Parser for LsSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();
        let mut parent = String::new();

        for line in input.lines() {
            // Skip total lines
            if line.starts_with("total ") {
                continue;
            }
            // Skip blank lines
            if line.trim().is_empty() {
                continue;
            }
            // Parent directory header (for -R)
            if !is_long_entry(line) && line.trim_end().ends_with(':') {
                parent = line.trim_end_matches(':').trim().to_string();
                continue;
            }
            // Skip non-long-format lines
            if !is_long_entry(line) {
                continue;
            }

            let parts = split_ls_long_line(line.trim());
            let mut entry = Map::new();

            if parts.len() >= 9 {
                let filename_field = parts[8];
                let link_parts: Vec<&str> = filename_field.splitn(2, " -> ").collect();
                entry.insert(
                    "filename".to_string(),
                    Value::String(link_parts[0].to_string()),
                );
                if link_parts.len() > 1 {
                    entry.insert(
                        "link_to".to_string(),
                        Value::String(link_parts[1].to_string()),
                    );
                }
            } else {
                continue;
            }

            if !parent.is_empty() {
                entry.insert("parent".to_string(), Value::String(parent.clone()));
            }

            entry.insert("flags".to_string(), Value::String(parts[0].to_string()));

            // links as integer
            if let Ok(n) = parts[1].parse::<i64>() {
                entry.insert("links".to_string(), Value::Number(n.into()));
            }
            entry.insert("owner".to_string(), Value::String(parts[2].to_string()));
            entry.insert("group".to_string(), Value::String(parts[3].to_string()));

            // size as integer
            if let Ok(n) = parts[4].parse::<i64>() {
                entry.insert("size".to_string(), Value::Number(n.into()));
            }

            if parts.len() > 7 {
                let date = format!("{} {} {}", parts[5], parts[6], parts[7]);
                entry.insert("date".to_string(), Value::String(date));
            }

            result.push(entry);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_s_basic() {
        let input = "total 20\ndr-xr-xr-x.  17 root root  224 Aug 15 10:56 .\n-rw-r--r--.   1 root root  100 Aug 15 10:53 file.txt\n";
        let parser = LsSParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String(".".to_string()))
            );
            assert_eq!(arr[0].get("links"), Some(&Value::Number(17.into())));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ls_s_empty() {
        let parser = LsSParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

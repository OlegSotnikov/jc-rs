//! Parser for `free` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_size_to_int;
use serde_json::{Map, Value};

pub struct FreeParser;

static INFO: ParserInfo = ParserInfo {
    name: "free",
    argument: "--free",
    version: "1.0.0",
    description: "Converts `free` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["free"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FREE_PARSER: FreeParser = FreeParser;

inventory::submit! {
    ParserEntry::new(&FREE_PARSER)
}

impl Parser for FreeParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_free(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_free(input: &str) -> Vec<Map<String, Value>> {
    let mut lines = input.lines().filter(|l| !l.trim().is_empty());

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    // Parse headers (skip the leading empty space before "total")
    let headers: Vec<String> = header_line
        .split_whitespace()
        .map(normalize_free_header)
        .collect();

    let mut output = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Collect all whitespace-separated tokens
        let mut parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        // First token is the row label (e.g. "Mem:" or "Swap:")
        let row_type = parts.remove(0).trim_end_matches(':').to_string();

        let mut record = Map::new();
        record.insert("type".to_string(), Value::String(row_type));

        for (i, header) in headers.iter().enumerate() {
            let val = parts.get(i).copied().unwrap_or("").trim().to_string();
            if val.is_empty() {
                // skip missing fields (e.g. Swap has no shared/buff_cache/available)
            } else if let Ok(n) = val.parse::<i64>() {
                record.insert(header.clone(), Value::Number(n.into()));
            } else if let Some(n) = convert_size_to_int(&val, false) {
                record.insert(header.clone(), Value::Number(n.into()));
            } else {
                record.insert(header.clone(), Value::String(val));
            }
        }

        output.push(record);
    }

    output
}

fn normalize_free_header(h: &str) -> String {
    match h.to_lowercase().as_str() {
        "total" => "total".to_string(),
        "used" => "used".to_string(),
        "free" => "free".to_string(),
        "shared" => "shared".to_string(),
        "buff/cache" => "buff_cache".to_string(),
        "available" => "available".to_string(),
        _ => h.to_lowercase().replace('/', "_"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/free.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/free.json"
        ))
        .unwrap();

        let parser = FreeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_free_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/free.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/free.json"
        ))
        .unwrap();

        let parser = FreeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

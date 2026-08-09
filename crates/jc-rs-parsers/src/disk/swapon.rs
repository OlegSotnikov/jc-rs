//! Parser for `swapon` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SwaponParser;

static INFO: ParserInfo = ParserInfo {
    name: "swapon",
    argument: "--swapon",
    version: "1.0.0",
    description: "Converts `swapon` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["swapon"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SWAPON_PARSER: SwaponParser = SwaponParser;

inventory::submit! {
    ParserEntry::new(&SWAPON_PARSER)
}

impl Parser for SwaponParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_swapon(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Parse human-readable size strings to bytes.
/// Handles: "2G" -> 2147483648, "512M" -> 536870912, "1024K" -> 1048576,
///          "512K" -> 524288, plain integers (already in bytes or KB).
fn parse_size(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return None;
    }

    // Try plain integer first
    if let Ok(n) = s.parse::<i64>() {
        return Some(n);
    }

    // Try with suffix
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix('G') {
        (n, 1024i64 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024i64 * 1024)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1024i64)
    } else {
        let n = s.strip_suffix('T')?;
        (n, 1024i64 * 1024 * 1024 * 1024)
    };

    // Parse the numeric part (may be float like "1.5G")
    if let Ok(n) = num_str.parse::<f64>() {
        Some((n * multiplier as f64) as i64)
    } else {
        None
    }
}

fn parse_swapon(input: &str) -> Vec<Map<String, Value>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() < 2 {
        return Vec::new();
    }

    // Parse header using whitespace split
    let raw_headers: Vec<&str> = lines[0].split_whitespace().collect();
    if raw_headers.is_empty() {
        return Vec::new();
    }

    let col_names: Vec<String> = raw_headers
        .iter()
        .map(|h| match h.to_lowercase().as_str() {
            "filename" | "name" => "name".to_string(),
            "type" => "type".to_string(),
            "size" => "size".to_string(),
            "used" => "used".to_string(),
            "prio" | "priority" => "priority".to_string(),
            "uuid" => "uuid".to_string(),
            "label" => "label".to_string(),
            other => other.to_lowercase(),
        })
        .collect();

    let mut results = Vec::new();

    for &line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        let values: Vec<&str> = line.split_whitespace().collect();
        let mut record = Map::new();

        for (i, key) in col_names.iter().enumerate() {
            let val = match values.get(i) {
                Some(v) => v.trim(),
                None => continue,
            };

            if val.is_empty() {
                continue;
            }

            match key.as_str() {
                "size" | "used" => {
                    if let Some(n) = parse_size(val) {
                        record.insert(key.clone(), Value::Number(n.into()));
                    } else {
                        record.insert(key.clone(), Value::String(val.to_string()));
                    }
                }
                "priority" => {
                    if let Ok(n) = val.parse::<i64>() {
                        record.insert(key.clone(), Value::Number(n.into()));
                    } else {
                        record.insert(key.clone(), Value::String(val.to_string()));
                    }
                }
                _ => {
                    record.insert(key.clone(), Value::String(val.to_string()));
                }
            }
        }

        results.push(record);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swapon_v1() {
        let input = include_str!("../../../../tests/fixtures/generic/swapon-all-v1.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/swapon-all-v1.json"
        ))
        .unwrap();

        let parser = SwaponParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_swapon_v2() {
        let input = include_str!("../../../../tests/fixtures/generic/swapon-all-v2.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/swapon-all-v2.json"
        ))
        .unwrap();

        let parser = SwaponParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("2G"), Some(2147483648));
        assert_eq!(parse_size("1024M"), Some(1073741824));
        assert_eq!(parse_size("512K"), Some(524288));
        assert_eq!(parse_size("1234"), Some(1234));
        assert_eq!(parse_size("-"), None);
    }
}

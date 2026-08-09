//! Parser for `/proc/swaps`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcSwapsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_swaps",
    argument: "--proc-swaps",
    version: "1.0.0",
    description: "Converts `/proc/swaps` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/swaps"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_SWAPS_PARSER: ProcSwapsParser = ProcSwapsParser;
inventory::submit! { ParserEntry::new(&PROC_SWAPS_PARSER) }

impl Parser for ProcSwapsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();

        // Skip the header line
        for line in input.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            let mut entry = Map::new();
            entry.insert("filename".to_string(), Value::String(parts[0].to_string()));
            entry.insert("type".to_string(), Value::String(parts[1].to_string()));
            if let Ok(v) = parts[2].parse::<i64>() {
                entry.insert("size".to_string(), Value::Number(v.into()));
            }
            if let Ok(v) = parts[3].parse::<i64>() {
                entry.insert("used".to_string(), Value::Number(v.into()));
            }
            if let Ok(v) = parts[4].parse::<i64>() {
                entry.insert("priority".to_string(), Value::Number(v.into()));
            }
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_swaps() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/swaps");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/swaps.json"
        ))
        .unwrap();
        let parser = ProcSwapsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

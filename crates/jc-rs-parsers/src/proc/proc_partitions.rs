//! Parser for `/proc/partitions`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPartitionsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_partitions",
    argument: "--proc-partitions",
    version: "1.0.0",
    description: "Converts `/proc/partitions` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/partitions"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PARTITIONS_PARSER: ProcPartitionsParser = ProcPartitionsParser;
inventory::submit! { ParserEntry::new(&PROC_PARTITIONS_PARSER) }

impl Parser for ProcPartitionsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();

        // Skip the first 2 lines (header + blank line)
        for line in input.lines().skip(2) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let mut entry = Map::new();
            if let Ok(v) = parts[0].parse::<i64>() {
                entry.insert("major".to_string(), Value::Number(v.into()));
            }
            if let Ok(v) = parts[1].parse::<i64>() {
                entry.insert("minor".to_string(), Value::Number(v.into()));
            }
            if let Ok(v) = parts[2].parse::<i64>() {
                entry.insert("num_blocks".to_string(), Value::Number(v.into()));
            }
            entry.insert("name".to_string(), Value::String(parts[3].to_string()));
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_partitions() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/partitions");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/partitions.json"
        ))
        .unwrap();
        let parser = ProcPartitionsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

//! Parser for `/proc/meminfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcMeminfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_meminfo",
    argument: "--proc-meminfo",
    version: "1.0.0",
    description: "Converts `/proc/meminfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/meminfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_MEMINFO_PARSER: ProcMeminfoParser = ProcMeminfoParser;

inventory::submit! { ParserEntry::new(&PROC_MEMINFO_PARSER) }

impl Parser for ProcMeminfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut out: Map<String, Value> = Map::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Format: "Key: value kB" or "Key: value"
            // Replace colon then split to get key and first token of value
            let line = line.replace(':', "");
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].to_string();
                if let Ok(val) = parts[1].parse::<i64>() {
                    out.insert(key, Value::Number(val.into()));
                }
            }
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_meminfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/meminfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/meminfo.json"
        ))
        .unwrap();
        let parser = ProcMeminfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

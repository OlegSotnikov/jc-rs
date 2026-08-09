//! Parser for `/proc/vmstat`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcVmstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_vmstat",
    argument: "--proc-vmstat",
    version: "1.0.0",
    description: "Converts `/proc/vmstat` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/vmstat"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_VMSTAT_PARSER: ProcVmstatParser = ProcVmstatParser;
inventory::submit! { ParserEntry::new(&PROC_VMSTAT_PARSER) }

impl Parser for ProcVmstatParser {
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
    fn test_proc_vmstat() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/vmstat");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/vmstat.json"
        ))
        .unwrap();
        let parser = ProcVmstatParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

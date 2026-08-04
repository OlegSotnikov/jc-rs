//! Parser for `/proc/ioports`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcIoportsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_ioports",
    argument: "--proc-ioports",
    version: "1.0.0",
    description: "Converts `/proc/ioports` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/ioports"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_IOPORTS_PARSER: ProcIoportsParser = ProcIoportsParser;

inventory::submit! { ParserEntry::new(&PROC_IOPORTS_PARSER) }

impl Parser for ProcIoportsParser {
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

            // Split on " : " to separate range from device
            let Some((range_part, device)) = line.split_once(" : ") else {
                continue;
            };

            let range_part = range_part.trim();
            let device = device.trim();

            // Split range on "-"
            let Some((start, end)) = range_part.split_once('-') else {
                continue;
            };

            let mut entry = Map::new();
            entry.insert("start".to_string(), Value::String(start.to_string()));
            entry.insert("end".to_string(), Value::String(end.to_string()));
            entry.insert("device".to_string(), Value::String(device.to_string()));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_ioports() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/ioports");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/ioports.json"
        ))
        .unwrap();
        let parser = ProcIoportsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

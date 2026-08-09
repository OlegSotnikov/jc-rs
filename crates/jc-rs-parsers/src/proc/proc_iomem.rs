//! Parser for `/proc/iomem`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcIomemParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_iomem",
    argument: "--proc-iomem",
    version: "1.0.0",
    description: "Converts `/proc/iomem` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/iomem"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_IOMEM_PARSER: ProcIomemParser = ProcIomemParser;

inventory::submit! { ParserEntry::new(&PROC_IOMEM_PARSER) }

impl Parser for ProcIomemParser {
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
    fn test_proc_iomem() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/iomem");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/iomem.json"
        ))
        .unwrap();
        let parser = ProcIomemParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

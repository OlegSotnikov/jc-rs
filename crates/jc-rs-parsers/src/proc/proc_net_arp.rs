//! Parser for `/proc/net/arp`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetArpParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_arp",
    argument: "--proc-net-arp",
    version: "1.0.0",
    description: "Converts `/proc/net/arp` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/arp"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetArpParser = ProcNetArpParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetArpParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let headers = [
            "IP_address",
            "HW_type",
            "Flags",
            "HW_address",
            "Mask",
            "Device",
        ];
        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut map = Map::new();
            for (i, &h) in headers.iter().enumerate() {
                let val = parts.get(i).copied().unwrap_or("");
                map.insert(h.to_string(), Value::String(val.to_string()));
            }
            results.push(map);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_arp() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_arp");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_arp.json"
        ))
        .unwrap();
        let result = ProcNetArpParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

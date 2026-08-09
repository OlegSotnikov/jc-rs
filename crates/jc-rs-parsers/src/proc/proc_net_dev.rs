//! Parser for `/proc/net/dev`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetDevParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_dev",
    argument: "--proc-net-dev",
    version: "1.0.0",
    description: "Converts `/proc/net/dev` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/dev"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetDevParser = ProcNetDevParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

const FIELDS: &[&str] = &[
    "r_bytes",
    "r_packets",
    "r_errs",
    "r_drop",
    "r_fifo",
    "r_frame",
    "r_compressed",
    "r_multicast",
    "t_bytes",
    "t_packets",
    "t_errs",
    "t_drop",
    "t_fifo",
    "t_colls",
    "t_carrier",
    "t_compressed",
];

impl Parser for ProcNetDevParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines().skip(2) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(colon) = line.find(':') {
                let iface = line[..colon].trim();
                let rest = &line[colon + 1..];
                let nums: Vec<&str> = rest.split_whitespace().collect();

                let mut map = Map::new();
                map.insert("interface".to_string(), Value::String(iface.to_string()));
                for (i, &field) in FIELDS.iter().enumerate() {
                    let v = nums.get(i).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    map.insert(field.to_string(), Value::Number(v.into()));
                }
                results.push(map);
            }
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_dev() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_dev");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_dev.json"
        ))
        .unwrap();
        let result = ProcNetDevParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

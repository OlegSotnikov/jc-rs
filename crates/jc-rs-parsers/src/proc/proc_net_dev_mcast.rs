//! Parser for `/proc/net/dev_mcast`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetDevMcastParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_dev_mcast",
    argument: "--proc-net-dev-mcast",
    version: "1.0.0",
    description: "Converts `/proc/net/dev_mcast` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/dev_mcast"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetDevMcastParser = ProcNetDevMcastParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetDevMcastParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            let mut map = Map::new();
            map.insert(
                "index".to_string(),
                parts[0]
                    .parse::<i64>()
                    .map(|v| Value::Number(v.into()))
                    .unwrap_or(Value::Null),
            );
            map.insert("interface".to_string(), Value::String(parts[1].to_string()));
            map.insert(
                "dmi_u".to_string(),
                parts[2]
                    .parse::<i64>()
                    .map(|v| Value::Number(v.into()))
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "dmi_g".to_string(),
                parts[3]
                    .parse::<i64>()
                    .map(|v| Value::Number(v.into()))
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "dmi_address".to_string(),
                Value::String(parts[4].to_string()),
            );
            results.push(map);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_dev_mcast() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_dev_mcast");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_dev_mcast.json"
        ))
        .unwrap();
        let result = ProcNetDevMcastParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

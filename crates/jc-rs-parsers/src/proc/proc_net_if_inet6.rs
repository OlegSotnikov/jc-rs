//! Parser for `/proc/net/if_inet6`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetIfInet6Parser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_if_inet6",
    argument: "--proc-net-if-inet6",
    version: "1.0.0",
    description: "Converts `/proc/net/if_inet6` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/if_inet6"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetIfInet6Parser = ProcNetIfInet6Parser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetIfInet6Parser {
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
            if parts.len() < 6 {
                continue;
            }

            let mut map = Map::new();
            map.insert("address".to_string(), Value::String(parts[0].to_string()));
            map.insert("index".to_string(), Value::String(parts[1].to_string()));
            map.insert("prefix".to_string(), Value::String(parts[2].to_string()));
            map.insert("scope".to_string(), Value::String(parts[3].to_string()));
            map.insert("flags".to_string(), Value::String(parts[4].to_string()));
            map.insert("name".to_string(), Value::String(parts[5].to_string()));
            results.push(map);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_if_inet6() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_if_inet6");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_if_inet6.json"
        ))
        .unwrap();
        let result = ProcNetIfInet6Parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

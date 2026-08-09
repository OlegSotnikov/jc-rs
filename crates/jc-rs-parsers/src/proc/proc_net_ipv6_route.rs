//! Parser for `/proc/net/ipv6_route`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetIpv6RouteParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_ipv6_route",
    argument: "--proc-net-ipv6-route",
    version: "1.0.0",
    description: "Converts `/proc/net/ipv6_route` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/ipv6_route"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetIpv6RouteParser = ProcNetIpv6RouteParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetIpv6RouteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let headers = [
            "dest_net",
            "dest_prefix",
            "source_net",
            "source_prefix",
            "next_hop",
            "metric",
            "ref_count",
            "use_count",
            "flags",
            "device",
        ];

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }
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
    fn test_proc_net_ipv6_route() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_ipv6_route");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_ipv6_route.json"
        ))
        .unwrap();
        let result = ProcNetIpv6RouteParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

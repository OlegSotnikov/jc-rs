//! Parser for `/proc/net/route`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetRouteParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_route",
    argument: "--proc-net-route",
    version: "1.0.0",
    description: "Converts `/proc/net/route` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/route"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetRouteParser = ProcNetRouteParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

/// Fields that should be parsed as integers (by column name).
fn is_int_field(name: &str) -> bool {
    matches!(
        name,
        "RefCnt" | "Use" | "Metric" | "MTU" | "Window" | "IRTT"
    )
}

/// Fields that should remain as strings (hex values kept as-is).
fn is_str_field(name: &str) -> bool {
    matches!(name, "Iface" | "Destination" | "Gateway" | "Flags" | "Mask")
}

impl Parser for ProcNetRouteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines();

        // First line is the header (tab-separated)
        let header_line = match lines.next() {
            Some(h) => h,
            None => return Ok(ParseOutput::Array(vec![])),
        };
        let headers: Vec<&str> = header_line.split_whitespace().collect();

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut map = Map::new();
            for (i, &h) in headers.iter().enumerate() {
                let val = parts.get(i).copied().unwrap_or("");
                if is_int_field(h) {
                    let n: i64 = val.parse().unwrap_or(0);
                    map.insert(h.to_string(), Value::Number(n.into()));
                } else if is_str_field(h) {
                    map.insert(h.to_string(), Value::String(val.to_string()));
                } else {
                    // Default: try int, fallback to string
                    if let Ok(n) = val.parse::<i64>() {
                        map.insert(h.to_string(), Value::Number(n.into()));
                    } else {
                        map.insert(h.to_string(), Value::String(val.to_string()));
                    }
                }
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
    fn test_proc_net_route() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_route");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_route.json"
        ))
        .unwrap();
        let result = ProcNetRouteParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

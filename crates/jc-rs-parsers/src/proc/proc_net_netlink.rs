//! Parser for `/proc/net/netlink`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetNetlinkParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_netlink",
    argument: "--proc-net-netlink",
    version: "1.0.0",
    description: "Converts `/proc/net/netlink` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/netlink"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetNetlinkParser = ProcNetNetlinkParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

/// Fields that should be parsed as integers (by column name).
fn is_int_field(name: &str) -> bool {
    matches!(
        name,
        "Eth" | "Pid" | "Rmem" | "Wmem" | "Dump" | "Locks" | "Drops" | "Inode"
    )
}

impl Parser for ProcNetNetlinkParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines();

        // First line is the header
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
                } else {
                    map.insert(h.to_string(), Value::String(val.to_string()));
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
    fn test_proc_net_netlink() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_netlink");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_netlink.json"
        ))
        .unwrap();
        let result = ProcNetNetlinkParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

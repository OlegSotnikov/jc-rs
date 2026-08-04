//! Parser for `/proc/net/packet`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetPacketParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_packet",
    argument: "--proc-net-packet",
    version: "1.0.0",
    description: "Converts `/proc/net/packet` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/packet"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetPacketParser = ProcNetPacketParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetPacketParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut lines = input.lines();

        // Skip header line
        if lines.next().is_none() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // Take first data line only
        let data_line = loop {
            match lines.next() {
                None => return Ok(ParseOutput::Object(Map::new())),
                Some(l) if l.trim().is_empty() => continue,
                Some(l) => break l,
            }
        };

        let parts: Vec<&str> = data_line.split_whitespace().collect();
        if parts.len() < 9 {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut map = Map::new();
        map.insert("sk".to_string(), Value::String(parts[0].to_string()));
        map.insert(
            "RefCnt".to_string(),
            Value::Number(parts[1].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert(
            "Type".to_string(),
            Value::Number(parts[2].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert("Proto".to_string(), Value::String(parts[3].to_string()));
        map.insert(
            "Iface".to_string(),
            Value::Number(parts[4].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert(
            "R".to_string(),
            Value::Number(parts[5].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert(
            "Rmem".to_string(),
            Value::Number(parts[6].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert(
            "User".to_string(),
            Value::Number(parts[7].parse::<i64>().unwrap_or(0).into()),
        );
        map.insert(
            "Inode".to_string(),
            Value::Number(parts[8].parse::<i64>().unwrap_or(0).into()),
        );

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_packet() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_packet");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_packet.json"
        ))
        .unwrap();
        let result = ProcNetPacketParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

//! Parser for `/proc/buddyinfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcBuddyinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_buddyinfo",
    argument: "--proc-buddyinfo",
    version: "1.0.0",
    description: "Converts `/proc/buddyinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/buddyinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_BUDDYINFO_PARSER: ProcBuddyinfoParser = ProcBuddyinfoParser;
inventory::submit! { ParserEntry::new(&PROC_BUDDYINFO_PARSER) }

impl Parser for ProcBuddyinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Format: "Node N, zone ZONE n n n ..."
            // Split on "," to get "Node N" and "zone ZONE n n n..."
            let Some(comma_pos) = line.find(',') else {
                continue;
            };

            let node_part = &line[..comma_pos];
            let rest = &line[comma_pos + 1..];

            // Parse node number
            let node_parts: Vec<&str> = node_part.split_whitespace().collect();
            if node_parts.len() < 2 {
                continue;
            }
            let node: i64 = match node_parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Parse "zone ZONE n n n..."
            let rest_parts: Vec<&str> = rest.split_whitespace().collect();
            if rest_parts.len() < 2 {
                continue;
            }
            // rest_parts[0] == "zone", rest_parts[1] == zone name, rest_parts[2..] == chunk counts
            let zone = rest_parts[1].to_string();
            let free_chunks: Vec<Value> = rest_parts[2..]
                .iter()
                .filter_map(|s| s.parse::<i64>().ok())
                .map(|v| Value::Number(v.into()))
                .collect();

            let mut entry = Map::new();
            entry.insert("node".to_string(), Value::Number(node.into()));
            entry.insert("zone".to_string(), Value::String(zone));
            entry.insert("free_chunks".to_string(), Value::Array(free_chunks));
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_buddyinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/buddyinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/buddyinfo.json"
        ))
        .unwrap();
        let parser = ProcBuddyinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

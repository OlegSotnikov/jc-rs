//! Parser for `/proc/net/igmp`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetIgmpParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_igmp",
    argument: "--proc-net-igmp",
    version: "1.0.0",
    description: "Converts `/proc/net/igmp` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/igmp"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetIgmpParser = ProcNetIgmpParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetIgmpParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut current_device: Option<Map<String, Value>> = None;
        let mut current_groups: Vec<Value> = Vec::new();

        // skip the header line
        for line in input.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            // group lines start with whitespace/tab
            if line.starts_with('\t') || line.starts_with(' ') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let mut group = Map::new();
                    group.insert("address".to_string(), Value::String(parts[0].to_string()));
                    let users: i64 = parts[1].parse().unwrap_or(0);
                    group.insert("users".to_string(), Value::Number(users.into()));
                    group.insert("timer".to_string(), Value::String(parts[2].to_string()));
                    let reporter: i64 = parts[3].parse().unwrap_or(0);
                    group.insert("reporter".to_string(), Value::Number(reporter.into()));
                    current_groups.push(Value::Object(group));
                }
            } else {
                // device header line: index device : count querier
                // flush the previous device
                if let Some(mut dev) = current_device.take() {
                    dev.insert(
                        "groups".to_string(),
                        Value::Array(std::mem::take(&mut current_groups)),
                    );
                    results.push(dev);
                }
                current_groups.clear();

                let parts: Vec<&str> = line.split_whitespace().collect();
                // parts: [index, device, ":", count, querier]
                if parts.len() >= 5 {
                    let index: i64 = parts[0].parse().unwrap_or(0);
                    let device = parts[1].to_string();
                    // parts[2] is ":"
                    let count: i64 = parts[3].parse().unwrap_or(0);
                    let querier = parts[4].to_string();

                    let mut dev = Map::new();
                    dev.insert("index".to_string(), Value::Number(index.into()));
                    dev.insert("device".to_string(), Value::String(device));
                    dev.insert("count".to_string(), Value::Number(count.into()));
                    dev.insert("querier".to_string(), Value::String(querier));
                    current_device = Some(dev);
                }
            }
        }

        // flush the last device
        if let Some(mut dev) = current_device.take() {
            dev.insert(
                "groups".to_string(),
                Value::Array(std::mem::take(&mut current_groups)),
            );
            results.push(dev);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_igmp() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_igmp");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_igmp.json"
        ))
        .unwrap();
        let result = ProcNetIgmpParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_net_igmp_more() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_igmp_more");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_igmp_more.json"
        ))
        .unwrap();
        let result = ProcNetIgmpParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

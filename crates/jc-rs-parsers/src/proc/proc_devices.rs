//! Parser for `/proc/devices`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcDevicesParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_devices",
    argument: "--proc-devices",
    version: "1.0.0",
    description: "Converts `/proc/devices` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/devices"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_DEVICES_PARSER: ProcDevicesParser = ProcDevicesParser;

inventory::submit! { ParserEntry::new(&PROC_DEVICES_PARSER) }

impl Parser for ProcDevicesParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut character = Map::new();
        let mut block = Map::new();
        let mut current_section: Option<&str> = None;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed == "Character devices:" {
                current_section = Some("character");
                continue;
            }
            if trimmed == "Block devices:" {
                current_section = Some("block");
                continue;
            }

            let section = match current_section {
                Some(s) => s,
                None => continue,
            };

            // Format: " num name"
            let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                continue;
            }
            let major = parts[0].trim().to_string();
            let name = parts[1].trim().to_string();

            let target = if section == "character" {
                &mut character
            } else {
                &mut block
            };

            target
                .entry(&major)
                .and_modify(|v| {
                    if let Value::Array(arr) = v {
                        arr.push(Value::String(name.clone()));
                    }
                })
                .or_insert_with(|| Value::Array(vec![Value::String(name)]));
        }

        let mut out = Map::new();
        out.insert("character".to_string(), Value::Object(character));
        out.insert("block".to_string(), Value::Object(block));

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_devices() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/devices");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/devices.json"
        ))
        .unwrap();
        let parser = ProcDevicesParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

//! Parser for `udevadm info` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct UdevadmParser;

static INFO: ParserInfo = ParserInfo {
    name: "udevadm",
    argument: "--udevadm",
    version: "1.0.0",
    description: "Converts `udevadm info` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["udevadm info"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UDEVADM_PARSER: UdevadmParser = UdevadmParser;

inventory::submit! {
    ParserEntry::new(&UDEVADM_PARSER)
}

impl Parser for UdevadmParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_udevadm(input);
        Ok(ParseOutput::Object(out))
    }
}

fn parse_udevadm(input: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut s_list: Vec<Value> = Vec::new();
    let mut e_map: Map<String, Value> = Map::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split on first whitespace: prefix and value
        let (prefix, value) = match line.split_once(' ') {
            Some((p, v)) => (p, v.trim()),
            None => continue,
        };

        match prefix {
            "P:" => {
                out.insert("P".to_string(), Value::String(value.to_string()));
            }
            "N:" => {
                out.insert("N".to_string(), Value::String(value.to_string()));
            }
            "L:" => {
                if let Ok(n) = value.parse::<i64>() {
                    out.insert("L".to_string(), Value::Number(n.into()));
                } else {
                    out.insert("L".to_string(), Value::String(value.to_string()));
                }
            }
            "S:" => {
                s_list.push(Value::String(value.to_string()));
            }
            "E:" => {
                if let Some(eq_pos) = value.find('=') {
                    let k = value[..eq_pos].to_string();
                    let v = value[eq_pos + 1..].to_string();
                    e_map.insert(k, Value::String(v));
                }
            }
            _ => {
                // Other single-letter prefixes like Q:, M: etc.
                let key = prefix.trim_end_matches(':').to_string();
                out.insert(key, Value::String(value.to_string()));
            }
        }
    }

    if !s_list.is_empty() {
        out.insert("S".to_string(), Value::Array(s_list));
    }

    if !e_map.is_empty() {
        out.insert("E".to_string(), Value::Object(e_map));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udevadm_basic() {
        let input = "P: /devices/pci0000:00/0000:00:10.0/host32/target32:0:0/32:0:0:0/block/sda\n\
                     N: sda\n\
                     L: 0\n\
                     S: disk/by-path/pci-0000:00:10.0-scsi-0:0:0:0\n\
                     E: DEVPATH=/devices/pci0000:00\n\
                     E: DEVNAME=/dev/sda\n\
                     E: DEVTYPE=disk";

        let parser = UdevadmParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("P"),
                Some(&Value::String(
                    "/devices/pci0000:00/0000:00:10.0/host32/target32:0:0/32:0:0:0/block/sda"
                        .to_string()
                ))
            );
            assert_eq!(obj.get("N"), Some(&Value::String("sda".to_string())));
            assert_eq!(obj.get("L"), Some(&Value::Number(0.into())));
            if let Some(Value::Array(s)) = obj.get("S") {
                assert_eq!(s.len(), 1);
                assert_eq!(
                    s[0],
                    Value::String("disk/by-path/pci-0000:00:10.0-scsi-0:0:0:0".to_string())
                );
            } else {
                panic!("Expected S array");
            }
            if let Some(Value::Object(e)) = obj.get("E") {
                assert_eq!(e.get("DEVTYPE"), Some(&Value::String("disk".to_string())));
            } else {
                panic!("Expected E object");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_udevadm_empty() {
        let parser = UdevadmParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}

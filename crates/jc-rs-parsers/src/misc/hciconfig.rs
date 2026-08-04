//! Parser for `hciconfig` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct HciconfigParser;

static INFO: ParserInfo = ParserInfo {
    name: "hciconfig",
    argument: "--hciconfig",
    version: "1.4.0",
    description: "`hciconfig` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["hciconfig"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HCICONFIG_PARSER: HciconfigParser = HciconfigParser;

inventory::submit! {
    ParserEntry::new(&HCICONFIG_PARSER)
}

const INT_KEYS: &[&str] = &[
    "acl_mtu",
    "acl_mtu_packets",
    "sco_mtu",
    "sco_mtu_packets",
    "rx_bytes",
    "rx_acl",
    "rx_sco",
    "rx_events",
    "rx_errors",
    "tx_bytes",
    "tx_acl",
    "tx_sco",
    "tx_commands",
    "tx_errors",
];

impl Parser for HciconfigParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result: Vec<Map<String, Value>> = Vec::new();
        let mut device: Option<Map<String, Value>> = None;
        let mut line_count = 0usize;

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            line_count += 1;

            // New device block starts with a non-space character
            if !line.starts_with(|c: char| c.is_whitespace()) {
                if let Some(d) = device.take() {
                    result.push(d);
                }
                line_count = 1;

                // hci0:   Type: Primary  Bus: USB
                let clean = line.replace(':', " ");
                let parts: Vec<&str> = clean.split_whitespace().collect();
                let mut d = Map::new();
                if parts.len() >= 5 {
                    d.insert("device".to_string(), Value::String(parts[0].to_string()));
                    d.insert("type".to_string(), Value::String(parts[2].to_string()));
                    d.insert("bus".to_string(), Value::String(parts[4].to_string()));
                }
                device = Some(d);
                continue;
            }

            let trimmed = line.trim();

            if let Some(ref mut d) = device {
                if trimmed.starts_with("BD Address:") {
                    // BD Address: 00:50:56:E7:46:1A  ACL MTU: 8192:128  SCO MTU: 64:128
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 9 {
                        d.insert(
                            "bd_address".to_string(),
                            Value::String(parts[2].to_string()),
                        );
                        let acl: Vec<&str> = parts[5].splitn(2, ':').collect();
                        let sco: Vec<&str> = parts[8].splitn(2, ':').collect();
                        let acl_mtu = acl.first().copied().unwrap_or("0");
                        let acl_pkt = acl.get(1).copied().unwrap_or("0");
                        let sco_mtu = sco.first().copied().unwrap_or("0");
                        let sco_pkt = sco.get(1).copied().unwrap_or("0");
                        d.insert(
                            "acl_mtu".to_string(),
                            convert_to_int(acl_mtu)
                                .map(Value::from)
                                .unwrap_or(Value::Null),
                        );
                        d.insert(
                            "acl_mtu_packets".to_string(),
                            convert_to_int(acl_pkt)
                                .map(Value::from)
                                .unwrap_or(Value::Null),
                        );
                        d.insert(
                            "sco_mtu".to_string(),
                            convert_to_int(sco_mtu)
                                .map(Value::from)
                                .unwrap_or(Value::Null),
                        );
                        d.insert(
                            "sco_mtu_packets".to_string(),
                            convert_to_int(sco_pkt)
                                .map(Value::from)
                                .unwrap_or(Value::Null),
                        );
                    }
                } else if line_count == 3 {
                    // State line: "UP RUNNING" or "DOWN" etc.
                    let states: Vec<Value> = trimmed
                        .split_whitespace()
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    d.insert("state".to_string(), Value::Array(states));
                } else if trimmed.starts_with("RX bytes:") {
                    // RX bytes:1307 acl:0 sco:0 events:51 errors:0
                    let clean = trimmed.replace(':', " ");
                    let parts: Vec<&str> = clean.split_whitespace().collect();
                    if parts.len() >= 10 {
                        d.insert("rx_bytes".to_string(), int_val(parts[2]));
                        d.insert("rx_acl".to_string(), int_val(parts[4]));
                        d.insert("rx_sco".to_string(), int_val(parts[6]));
                        d.insert("rx_events".to_string(), int_val(parts[8]));
                        d.insert("rx_errors".to_string(), int_val(parts[10]));
                    }
                } else if trimmed.starts_with("TX bytes:") {
                    // TX bytes:1200 acl:0 sco:0 commands:51 errors:0
                    let clean = trimmed.replace(':', " ");
                    let parts: Vec<&str> = clean.split_whitespace().collect();
                    if parts.len() >= 10 {
                        d.insert("tx_bytes".to_string(), int_val(parts[2]));
                        d.insert("tx_acl".to_string(), int_val(parts[4]));
                        d.insert("tx_sco".to_string(), int_val(parts[6]));
                        d.insert("tx_commands".to_string(), int_val(parts[8]));
                        d.insert("tx_errors".to_string(), int_val(parts[10]));
                    }
                } else if trimmed.starts_with("Features:") {
                    let features: Vec<Value> = trimmed
                        .split_whitespace()
                        .skip(1)
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    d.insert("features".to_string(), Value::Array(features));
                } else if trimmed.starts_with("Packet type:") {
                    let types: Vec<Value> = trimmed
                        .split_whitespace()
                        .skip(2)
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    d.insert("packet_type".to_string(), Value::Array(types));
                } else if trimmed.starts_with("Link policy:") {
                    let policies: Vec<Value> = trimmed
                        .split_whitespace()
                        .skip(2)
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    d.insert("link_policy".to_string(), Value::Array(policies));
                } else if trimmed.starts_with("Link mode:") {
                    let modes: Vec<Value> = trimmed
                        .split_whitespace()
                        .skip(2)
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    d.insert("link_mode".to_string(), Value::Array(modes));
                } else if trimmed.starts_with("Name:") {
                    // Name: 'kbrazil-ubuntu'
                    let name_part = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                    // Strip surrounding single quotes
                    let name = name_part.trim_matches('\'');
                    d.insert("name".to_string(), Value::String(name.to_string()));
                } else if trimmed.starts_with("Class:") {
                    let class = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                    d.insert("class".to_string(), Value::String(class.to_string()));
                } else if trimmed.starts_with("Service Classes:") {
                    let raw: Vec<&str> = trimmed.split_whitespace().skip(2).collect();
                    let svc: Vec<Value> =
                        raw.iter().map(|s| Value::String(s.to_string())).collect();
                    // "Unspecified" → null in processed output
                    if svc.len() == 1 && raw[0] == "Unspecified" {
                        d.insert("service_classes".to_string(), Value::Null);
                    } else {
                        d.insert("service_classes".to_string(), Value::Array(svc));
                    }
                } else if trimmed.starts_with("Device Class:") {
                    let dev_class = trimmed
                        .split_whitespace()
                        .nth(2)
                        .unwrap_or("")
                        .trim_end_matches(',');
                    d.insert(
                        "device_class".to_string(),
                        Value::String(dev_class.to_string()),
                    );
                } else if trimmed.starts_with("HCI Version:") {
                    // HCI Version: 4.0 (0x6)  Revision: 0x22bb
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let version = format!("{} {}", parts[2], parts[3]);
                        d.insert("hci_version".to_string(), Value::String(version));
                        d.insert(
                            "hci_revision".to_string(),
                            Value::String(parts[5].to_string()),
                        );
                    }
                } else if trimmed.starts_with("LMP Version:") {
                    // LMP Version: 4.0 (0x6)  Subversion: 0x22bb
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let version = format!("{} {}", parts[2], parts[3]);
                        d.insert("lmp_version".to_string(), Value::String(version));
                        d.insert(
                            "lmp_subversion".to_string(),
                            Value::String(parts[5].to_string()),
                        );
                    }
                } else if trimmed.starts_with("Manufacturer:") {
                    let mfr = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                    d.insert("manufacturer".to_string(), Value::String(mfr.to_string()));
                }
            }
        }

        if let Some(d) = device.take() {
            result.push(d);
        }

        Ok(ParseOutput::Array(result))
    }
}

fn int_val(s: &str) -> Value {
    convert_to_int(s).map(Value::from).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = HciconfigParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();

        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    serde_json::Value::Object(got.clone()),
                    *exp,
                    "mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_hciconfig_ubuntu() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-20.04/hciconfig.out"),
            include_str!("../../../../tests/fixtures/ubuntu-20.04/hciconfig.json"),
        );
    }

    #[test]
    fn test_hciconfig_a_ubuntu() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-20.04/hciconfig-a.out"),
            include_str!("../../../../tests/fixtures/ubuntu-20.04/hciconfig-a.json"),
        );
    }

    #[test]
    fn test_hciconfig_empty() {
        let parser = HciconfigParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

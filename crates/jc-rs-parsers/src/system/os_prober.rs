//! Parser for `os-prober` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct OsProberParser;

static INFO: ParserInfo = ParserInfo {
    name: "os_prober",
    argument: "--os-prober",
    version: "1.2.0",
    description: "Converts `os-prober` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["os-prober"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static OS_PROBER_PARSER: OsProberParser = OsProberParser;

inventory::submit! {
    ParserEntry::new(&OS_PROBER_PARSER)
}

impl Parser for OsProberParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_os_prober(input);
        Ok(ParseOutput::Object(out))
    }
}

fn parse_os_prober(input: &str) -> Map<String, Value> {
    let mut out = Map::new();

    let data = input.trim();
    if data.is_empty() {
        return out;
    }

    // Format: partition:name:short_name:type
    let parts: Vec<&str> = data.splitn(4, ':').collect();
    if parts.len() < 4 {
        return out;
    }

    let mut partition = parts[0].trim().to_string();
    let name = parts[1].trim().to_string();
    let short_name = parts[2].trim().to_string();
    let type_ = parts[3].trim().to_string();

    // Check for EFI partition@boot-manager format
    if let Some(at_pos) = partition.find('@') {
        let efi_bootmgr = partition[at_pos + 1..].to_string();
        partition = partition[..at_pos].to_string();
        out.insert("partition".to_string(), Value::String(partition));
        out.insert("efi_bootmgr".to_string(), Value::String(efi_bootmgr));
    } else {
        out.insert("partition".to_string(), Value::String(partition));
    }

    out.insert("name".to_string(), Value::String(name));
    out.insert("short_name".to_string(), Value::String(short_name));
    out.insert("type".to_string(), Value::String(type_));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_prober_basic() {
        let input = "/dev/sda1:Windows 10:Windows:chain\n";

        let parser = OsProberParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("partition"),
                Some(&Value::String("/dev/sda1".to_string()))
            );
            assert_eq!(
                obj.get("name"),
                Some(&Value::String("Windows 10".to_string()))
            );
            assert_eq!(
                obj.get("short_name"),
                Some(&Value::String("Windows".to_string()))
            );
            assert_eq!(obj.get("type"), Some(&Value::String("chain".to_string())));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_os_prober_efi() {
        let input = "/dev/sda1@/EFI/Boot/bootx64.efi:Windows 10:Windows:chain\n";

        let parser = OsProberParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("partition"),
                Some(&Value::String("/dev/sda1".to_string()))
            );
            assert!(obj.get("efi_bootmgr").is_some());
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_os_prober_empty() {
        let parser = OsProberParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}

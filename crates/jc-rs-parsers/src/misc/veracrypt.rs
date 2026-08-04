//! Parser for `veracrypt` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct VeracryptParser;

static INFO: ParserInfo = ParserInfo {
    name: "veracrypt",
    argument: "--veracrypt",
    version: "1.0.0",
    description: "Converts `veracrypt` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["veracrypt"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static VERACRYPT_PARSER: VeracryptParser = VeracryptParser;

inventory::submit! {
    ParserEntry::new(&VERACRYPT_PARSER)
}

/// Try to parse a simple line-format entry: "N: path device mountpoint"
fn try_parse_line_format(line: &str) -> Option<Map<String, Value>> {
    // Pattern: "<slot>: <path> <device> <mountpoint>"
    let parts: Vec<&str> = line.splitn(2, ": ").collect();
    if parts.len() != 2 {
        return None;
    }
    let slot_str = parts[0].trim();
    if !slot_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let slot: i64 = slot_str.parse().ok()?;

    let rest = parts[1].trim();
    let tokens: Vec<&str> = rest.splitn(3, ' ').collect();
    if tokens.len() < 3 {
        return None;
    }

    let mut obj = Map::new();
    obj.insert("slot".to_string(), Value::Number(slot.into()));
    obj.insert("path".to_string(), Value::String(tokens[0].to_string()));
    obj.insert("device".to_string(), Value::String(tokens[1].to_string()));
    obj.insert(
        "mountpoint".to_string(),
        Value::String(tokens[2].to_string()),
    );
    Some(obj)
}

fn parse_verbose(lines: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    let mut current: Map<String, Value> = Map::new();

    for &line in lines {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                result.push(current.clone());
                current = Map::new();
            }
            continue;
        }

        if let Some(val) = line.strip_prefix("Slot: ") {
            if let Ok(n) = val.trim().parse::<i64>() {
                current.insert("slot".to_string(), Value::Number(n.into()));
            }
        } else if let Some(val) = line.strip_prefix("Volume: ") {
            current.insert("path".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("Virtual Device: ") {
            current.insert("device".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("Mount Directory: ") {
            current.insert(
                "mountpoint".to_string(),
                Value::String(val.trim().to_string()),
            );
        } else if let Some(val) = line.strip_prefix("Size: ") {
            current.insert("size".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("Type: ") {
            current.insert("type".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("Read-Only: ") {
            current.insert(
                "readonly".to_string(),
                Value::String(val.trim().to_string()),
            );
        } else if let Some(val) = line.strip_prefix("Hidden Volume Protected: ") {
            current.insert(
                "hidden_protected".to_string(),
                Value::String(val.trim().to_string()),
            );
        } else if let Some(val) = line.strip_prefix("Encryption Algorithm: ") {
            current.insert(
                "encryption_algo".to_string(),
                Value::String(val.trim().to_string()),
            );
        } else if let Some(val) = line.strip_prefix("Primary Key Size: ") {
            current.insert("pk_size".to_string(), Value::String(val.trim().to_string()));
        } else if line.starts_with("Secondary Key Size") {
            // "Secondary Key Size (XTS Mode): 256 bits"
            if let Some(colon_pos) = line.find(": ") {
                let val = &line[colon_pos + 2..];
                current.insert("sk_size".to_string(), Value::String(val.trim().to_string()));
            }
        } else if let Some(val) = line.strip_prefix("Block Size: ") {
            current.insert(
                "block_size".to_string(),
                Value::String(val.trim().to_string()),
            );
        } else if let Some(val) = line.strip_prefix("Mode of Operation: ") {
            current.insert("mode".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("PKCS-5 PRF: ") {
            current.insert("prf".to_string(), Value::String(val.trim().to_string()));
        } else if let Some(val) = line.strip_prefix("Volume Format Version: ") {
            if let Ok(n) = val.trim().parse::<i64>() {
                current.insert("format_version".to_string(), Value::Number(n.into()));
            }
        } else if let Some(val) = line.strip_prefix("Embedded Backup Header: ") {
            current.insert(
                "backup_header".to_string(),
                Value::String(val.trim().to_string()),
            );
        }
        // Unknown fields are silently ignored (matches Python behavior)
    }

    // Flush last entry
    if !current.is_empty() {
        result.push(current);
    }

    result
}

impl Parser for VeracryptParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Check which format: line format "N: path device mountpoint" or verbose "Slot: N"
        let first_non_empty = lines.iter().find(|l| !l.trim().is_empty()).copied();
        if first_non_empty.is_none() {
            return Ok(ParseOutput::Array(vec![]));
        }
        let first_line = first_non_empty.unwrap();

        // Try line format first
        if try_parse_line_format(first_line).is_some() {
            let result: Vec<Map<String, Value>> = lines
                .iter()
                .filter_map(|l| {
                    let l = l.trim();
                    if l.is_empty() {
                        None
                    } else {
                        try_parse_line_format(l)
                    }
                })
                .collect();
            return Ok(ParseOutput::Array(result));
        }

        // Verbose format
        let result = parse_verbose(&lines);
        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_veracrypt_verbose() {
        let input =
            include_str!("../../../../tests/fixtures/generic/veracrypt_verbose_list_volumes.out");
        let parser = VeracryptParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0]["slot"], Value::Number(1.into()));
                assert_eq!(arr[0]["path"], Value::String("/dev/sdb1".to_string()));
                assert_eq!(
                    arr[0]["device"],
                    Value::String("/dev/mapper/veracrypt1".to_string())
                );
                assert_eq!(
                    arr[0]["mountpoint"],
                    Value::String("/home/bob/mount/encrypt/sdb1".to_string())
                );
                assert_eq!(arr[0]["size"], Value::String("498 MiB".to_string()));
                assert_eq!(arr[0]["type"], Value::String("Normal".to_string()));
                assert_eq!(arr[0]["readonly"], Value::String("No".to_string()));
                assert_eq!(arr[0]["encryption_algo"], Value::String("AES".to_string()));
                assert_eq!(arr[0]["format_version"], Value::Number(2.into()));
                assert_eq!(arr[0]["backup_header"], Value::String("Yes".to_string()));

                assert_eq!(arr[1]["slot"], Value::Number(2.into()));
                assert_eq!(arr[1]["path"], Value::String("/dev/sdb2".to_string()));
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_veracrypt_verbose_unknown_fields() {
        // Same structure but with unknown fields (e.g. Label:) that should be ignored
        let input = include_str!(
            "../../../../tests/fixtures/generic/veracrypt_verbose_list_volumes_unknown_fields.out"
        );
        let parser = VeracryptParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0]["slot"], Value::Number(1.into()));
                assert_eq!(arr[1]["slot"], Value::Number(2.into()));
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_veracrypt_empty() {
        let parser = VeracryptParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}

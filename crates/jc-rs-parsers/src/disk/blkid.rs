//! Parser for `blkid` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct BlkidParser;

static INFO: ParserInfo = ParserInfo {
    name: "blkid",
    argument: "--blkid",
    version: "1.0.0",
    description: "Converts `blkid` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["blkid"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static BLKID_PARSER: BlkidParser = BlkidParser;

inventory::submit! {
    ParserEntry::new(&BLKID_PARSER)
}

impl Parser for BlkidParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_blkid(input);
        Ok(ParseOutput::Array(rows))
    }
}

const INT_FIELDS: &[&str] = &[
    "part_entry_number",
    "part_entry_offset",
    "part_entry_size",
    "id_part_entry_number",
    "id_part_entry_offset",
    "id_part_entry_size",
    "minimum_io_size",
    "physical_sector_size",
    "logical_sector_size",
    "id_iolimit_minimum_io_size",
    "id_iolimit_physical_sector_size",
    "id_iolimit_logical_sector_size",
];

fn is_normal_mode(input: &str) -> bool {
    // Normal mode: lines start with a device path like "/dev/xxx:" or "DEVNAME=xxx"
    // Key/value mode: lines like "KEY=VALUE" separated by blank lines
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Normal mode lines start with /dev or similar and have ": " after device
        if trimmed.contains(": ") || (trimmed.ends_with(':') && trimmed.starts_with('/')) {
            return true;
        }
        // Check for device prefix with KEY="VALUE" pairs
        if let Some(colon_pos) = trimmed.find(':') {
            let after = &trimmed[colon_pos + 1..];
            if after.trim().contains('=') && after.contains('"') {
                return true;
            }
        }
        break;
    }
    false
}

fn parse_blkid(input: &str) -> Vec<Map<String, Value>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if is_normal_mode(trimmed) {
        parse_normal_mode(trimmed)
    } else {
        parse_kv_mode(trimmed)
    }
}

fn parse_normal_mode(input: &str) -> Vec<Map<String, Value>> {
    let mut results = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut record = Map::new();

        // Find the device name: everything before the first ": " or first ":"
        let (device, rest) = if let Some(pos) = line.find(": ") {
            (line[..pos].to_string(), &line[pos + 2..])
        } else if line.ends_with(':') {
            (line[..line.len() - 1].to_string(), "")
        } else {
            // Try splitting on first space after a colon
            if let Some(pos) = line.find(':') {
                (line[..pos].to_string(), line[pos + 1..].trim())
            } else {
                continue;
            }
        };

        record.insert("device".to_string(), Value::String(device));

        // Parse KEY="VALUE" or KEY=VALUE pairs
        parse_kv_pairs(rest, &mut record);

        results.push(record);
    }

    results
}

fn parse_kv_mode(input: &str) -> Vec<Map<String, Value>> {
    let mut results = Vec::new();
    let mut current = Map::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                // Rename devname -> device
                if let Some(val) = current.remove("devname") {
                    current.insert("device".to_string(), val);
                }
                results.push(current);
                current = Map::new();
            }
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].to_lowercase();
            let val = line[eq_pos + 1..].trim().trim_matches('"').to_string();

            let key = if key == "devname" {
                "device".to_string()
            } else {
                key
            };

            if INT_FIELDS.contains(&key.as_str()) {
                if let Ok(n) = val.parse::<i64>() {
                    current.insert(key, Value::Number(n.into()));
                } else {
                    current.insert(key, Value::String(val));
                }
            } else {
                current.insert(key, Value::String(val));
            }
        }
    }

    if !current.is_empty() {
        if let Some(val) = current.remove("devname") {
            current.insert("device".to_string(), val);
        }
        results.push(current);
    }

    results
}

fn parse_kv_pairs(input: &str, record: &mut Map<String, Value>) {
    // Parse KEY="VALUE" pairs. Values may contain spaces inside quotes.
    let input = input.trim();
    if input.is_empty() {
        return;
    }

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Find '='
        let key_start = i;
        while i < len && chars[i] != '=' {
            i += 1;
        }
        if i >= len {
            break;
        }
        let key: String = chars[key_start..i].iter().collect();
        let key = key.to_lowercase();
        i += 1; // skip '='

        // Parse value (possibly quoted)
        let value = if i < len && chars[i] == '"' {
            i += 1; // skip opening quote
            let val_start = i;
            while i < len && chars[i] != '"' {
                i += 1;
            }
            let val: String = chars[val_start..i].iter().collect();
            if i < len {
                i += 1; // skip closing quote
            }
            val
        } else {
            let val_start = i;
            while i < len && !chars[i].is_whitespace() {
                i += 1;
            }
            chars[val_start..i].iter().collect()
        };

        let key = if key == "devname" {
            "device".to_string()
        } else {
            key
        };

        if INT_FIELDS.contains(&key.as_str()) {
            if let Ok(n) = value.parse::<i64>() {
                record.insert(key, Value::Number(n.into()));
            } else {
                record.insert(key, Value::String(value));
            }
        } else {
            record.insert(key, Value::String(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blkid_normal_mode() {
        let input = r#"/dev/sda1: UUID="abc-123" TYPE="xfs" PARTUUID="deadbeef"
/dev/sda2: UUID="def-456" TYPE="ext4""#;

        let parser = BlkidParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["device"], Value::String("/dev/sda1".into()));
            assert_eq!(arr[0]["uuid"], Value::String("abc-123".into()));
            assert_eq!(arr[0]["type"], Value::String("xfs".into()));
            assert_eq!(arr[1]["device"], Value::String("/dev/sda2".into()));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_blkid_kv_mode() {
        let input =
            "DEVNAME=/dev/sda1\nUUID=abc-123\nTYPE=xfs\n\nDEVNAME=/dev/sda2\nUUID=def-456\n";

        let parser = BlkidParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["device"], Value::String("/dev/sda1".into()));
            assert_eq!(arr[0]["uuid"], Value::String("abc-123".into()));
        } else {
            panic!("expected array");
        }
    }
}

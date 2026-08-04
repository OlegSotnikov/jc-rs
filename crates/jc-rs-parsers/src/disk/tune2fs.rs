//! Parser for `tune2fs -l` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct Tune2fsParser;

static INFO: ParserInfo = ParserInfo {
    name: "tune2fs",
    argument: "--tune2fs",
    version: "1.0.0",
    description: "Converts `tune2fs -l` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["tune2fs"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TUNE2FS_PARSER: Tune2fsParser = Tune2fsParser;

inventory::submit! {
    ParserEntry::new(&TUNE2FS_PARSER)
}

impl Parser for Tune2fsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_tune2fs(input);
        Ok(ParseOutput::Object(record))
    }
}

const INT_FIELDS: &[&str] = &[
    "inode_count",
    "block_count",
    "reserved_block_count",
    "free_blocks",
    "free_inodes",
    "first_block",
    "block_size",
    "fragment_size",
    "group_descriptor_size",
    "reserved_gdt_blocks",
    "blocks_per_group",
    "fragments_per_group",
    "inodes_per_group",
    "inode_blocks_per_group",
    "flex_block_group_size",
    "mount_count",
    "maximum_mount_count",
    "first_inode",
    "inode_size",
    "required_extra_isize",
    "desired_extra_isize",
    "journal_inode",
    "overhead_clusters",
];

/// Known date fields that should get epoch conversions.
const DATE_FIELDS: &[&str] = &[
    "filesystem_created",
    "last_mount_time",
    "last_write_time",
    "last_checked",
];

fn normalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .replace('#', "number")
        .replace(' ', "_")
}

/// Try to parse a ctime-style date string into a Unix epoch.
/// Format: "Mon Apr  6 15:10:37 2020"
fn parse_date_epoch(s: &str) -> Option<i64> {
    use chrono::NaiveDateTime;
    // Try common ctime format
    let s = s.trim();
    // ctime format: "Mon Apr  6 15:10:37 2020" — normalize double spaces
    let normalized = s.split_whitespace().collect::<Vec<&str>>().join(" ");
    // tune2fs prints local time with no zone; reading it as UTC put every
    // *_epoch out by the machine's offset.
    jc_rs_utils::parse_timestamp(&normalized, &[jc_rs_utils::timestamp::formats::F1000]).naive_epoch
}

fn parse_tune2fs(input: &str) -> Map<String, Value> {
    let mut record = Map::new();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return record;
    }

    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // First line may be "tune2fs VERSION"
        if line.starts_with("tune2fs ") && !line.contains(':') {
            let version = line["tune2fs ".len()..].trim().to_string();
            record.insert("version".to_string(), Value::String(version));
            continue;
        }

        // Split on first ':'
        if let Some(colon_pos) = line.find(':') {
            let raw_key = &line[..colon_pos];
            let val = line[colon_pos + 1..].trim();
            let key = normalize_key(raw_key);

            if key == "filesystem_features" {
                // Split on whitespace into array
                let features: Vec<Value> = val
                    .split_whitespace()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                record.insert(key, Value::Array(features));
            } else if INT_FIELDS.contains(&key.as_str()) {
                if let Ok(n) = val.parse::<i64>() {
                    record.insert(key, Value::Number(n.into()));
                } else {
                    record.insert(key, Value::String(val.to_string()));
                }
            } else {
                record.insert(key, Value::String(val.to_string()));
            }
        }
    }

    // Add epoch fields for date fields
    let date_keys: Vec<String> = DATE_FIELDS.iter().map(|s| s.to_string()).collect();
    for date_key in &date_keys {
        if let Some(Value::String(date_str)) = record.get(date_key) {
            let date_str = date_str.clone();
            let epoch = parse_date_epoch(&date_str);
            record.insert(
                format!("{}_epoch", date_key),
                match epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
            // Also add _epoch_utc (null for now, since we don't know the timezone)
            record.insert(format!("{}_epoch_utc", date_key), Value::Null);
        }
    }

    record
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tune2fs() {
        let input = include_str!("../../../../tests/fixtures/generic/tune2fs-l.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/tune2fs-l.json"
        ))
        .unwrap();

        let parser = Tune2fsParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_normalize_key() {
        assert_eq!(
            normalize_key("Filesystem revision #"),
            "filesystem_revision_number"
        );
        assert_eq!(normalize_key("Inode count"), "inode_count");
        assert_eq!(normalize_key("Reserved GDT blocks"), "reserved_gdt_blocks");
    }
}

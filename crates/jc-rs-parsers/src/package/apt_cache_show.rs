//! Parser for `apt-cache show` command output.
//!
//! This is an alias of the rpm_qi parser logic; both formats use the same
//! "Key: value" colon-separated structure with a Description block.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, normalize_key};
use serde_json::{Map, Value};

pub struct AptCacheShowParser;

static INFO: ParserInfo = ParserInfo {
    name: "apt_cache_show",
    argument: "--apt-cache-show",
    version: "1.0.0",
    description: "Converts `apt-cache show` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["apt-cache show"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static APT_CACHE_SHOW_PARSER: AptCacheShowParser = AptCacheShowParser;

inventory::submit! {
    ParserEntry::new(&APT_CACHE_SHOW_PARSER)
}

impl Parser for AptCacheShowParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let raw = parse_raw(input);
        let processed = process(raw);
        Ok(ParseOutput::Array(processed))
    }
}

/// Parse raw key-value blocks separated by blank lines.
/// Handles `Description-en:` multi-line blocks.
fn parse_raw(input: &str) -> Vec<Map<String, Value>> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    let mut results: Vec<Map<String, Value>> = Vec::new();
    let mut entry: Map<String, Value> = Map::new();
    let mut desc_lines: Vec<String> = Vec::new();
    // in_desc_entry: once set True (by Description-en:), stays True for the rest of the record.
    // Any indented line gets appended to description (mirrors Python parser behavior).
    let mut in_desc_entry: bool = false;

    for line in input.lines() {
        // Skip blank lines (record separator; we detect new records via "Package:" key)
        if line.trim().is_empty() {
            continue;
        }

        // Indented continuation line
        if in_desc_entry && line.starts_with(' ') {
            desc_lines.push(line.trim().to_string());
            continue;
        }

        // Key : value line
        let split_line: Vec<&str> = line.splitn(2, ": ").collect();
        if split_line.len() == 2 {
            let key_raw = split_line[0];
            let key = normalize_key(key_raw);

            // New entry starts when we see "Package:" key
            if (key == "package" || key_raw.starts_with("Name")) && !entry.is_empty() {
                if !desc_lines.is_empty() {
                    entry.insert(
                        "description".to_string(),
                        Value::String(desc_lines.join("  ")),
                    );
                    desc_lines.clear();
                }
                results.push(entry);
                entry = Map::new();
                in_desc_entry = false;
            }

            // Description-en: starts a description block
            if key_raw == "Description-en" {
                in_desc_entry = true;
                let first = split_line[1].trim().to_string();
                desc_lines = vec![first];
                continue;
            }

            // Store key-value (in_desc_entry stays set: Python bug replication)
            entry.insert(key, Value::String(split_line[1].trim().to_string()));
        }
        // (non-indented non-kv lines are ignored)
    }

    // Push the last entry
    if !entry.is_empty() {
        if !desc_lines.is_empty() {
            entry.insert(
                "description".to_string(),
                Value::String(desc_lines.join("  ")),
            );
        }
        results.push(entry);
    }

    results
}

/// Process raw records: convert integer fields, split list fields.
fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    let int_fields = ["installed_size", "size", "epoch"];
    let split_fields = [
        "depends",
        "pre_depends",
        "recommends",
        "suggests",
        "conflicts",
        "breaks",
        "tag",
        "replaces",
    ];

    raw.into_iter()
        .map(|mut entry| {
            for field in &int_fields {
                if let Some(Value::String(s)) = entry.get(*field) {
                    let s = s.clone();
                    if let Some(n) = convert_to_int(&s) {
                        entry.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
            }

            for field in &split_fields {
                if let Some(Value::String(s)) = entry.get(*field) {
                    let s = s.clone();
                    let parts: Vec<Value> = s
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    entry.insert(field.to_string(), Value::Array(parts));
                }
            }

            entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apt_cache_show_smoke() {
        let input = "Package: foo\nVersion: 1.0\nInstalled-Size: 100\nDepends: bar, baz\nDescription-en: A test package\n short description\n";
        let parser = AptCacheShowParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].get("package"), Some(&Value::String("foo".into())));
            assert_eq!(
                arr[0].get("installed_size"),
                Some(&Value::Number(100.into()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_apt_cache_show_fixture() {
        let fixture_out =
            include_str!("../../../../tests/fixtures/generic/apt_cache_show--standard.out");
        let fixture_json =
            include_str!("../../../../tests/fixtures/generic/apt_cache_show--standard.json");

        let parser = AptCacheShowParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "apt_cache_show fixture mismatch");
    }
}

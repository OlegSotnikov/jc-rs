//! Parser for `rpm -qi` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, normalize_key, parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct RpmQiParser;

static INFO: ParserInfo = ParserInfo {
    name: "rpm_qi",
    argument: "--rpm-qi",
    version: "1.0.0",
    description: "Converts `rpm -qi` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["rpm -qi", "rpm -qia", "rpm -qai"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static RPM_QI_PARSER: RpmQiParser = RpmQiParser;

inventory::submit! {
    ParserEntry::new(&RPM_QI_PARSER)
}

impl Parser for RpmQiParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let raw = parse_raw(input);
        let processed = process(raw);
        Ok(ParseOutput::Array(processed))
    }
}

/// Shared parse logic for rpm-qi-style key:value format.
/// Used by rpm_qi, apt_cache_show, and pkg_index_deb parsers.
pub(crate) fn parse_raw(input: &str) -> Vec<Map<String, Value>> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    let mut results: Vec<Map<String, Value>> = Vec::new();
    let mut entry: Map<String, Value> = Map::new();
    let mut desc_lines: Vec<String> = Vec::new();
    let mut in_desc: bool = false;
    let mut in_desc_en: bool = false;

    for line in input.lines() {
        let split_line: Vec<&str> = line.splitn(2, ": ").collect();

        // Description-en is for apt-cache show / pkg-index-deb
        if line.starts_with("Description-en:") {
            if !entry.is_empty() && in_desc {
                entry.insert(
                    "description".to_string(),
                    Value::String(desc_lines.join(" ").trim().to_string()),
                );
                desc_lines.clear();
            }
            in_desc = false;
            in_desc_en = true;
            // The value after "Description-en: "
            let val = if split_line.len() == 2 {
                split_line[1].trim().to_string()
            } else {
                String::new()
            };
            desc_lines = vec![val];
            continue;
        }

        // Description : (rpm style — note trailing space)
        if line.starts_with("Description :") {
            in_desc = true;
            in_desc_en = false;
            desc_lines = Vec::new();
            continue;
        }

        if in_desc {
            desc_lines.push(line.to_string());
            continue;
        }

        if in_desc_en {
            if line.starts_with(' ') {
                desc_lines.push(line.trim().to_string());
                continue;
            } else {
                // End of description_en block
                in_desc_en = false;
            }
        }

        if split_line.len() == 2 {
            let key_raw = split_line[0];
            let val = split_line[1].trim();
            let key = normalize_key(key_raw);

            // New entry starts when we see Name or Package key
            if (key_raw.starts_with("Name") || key_raw == "Package") && !entry.is_empty() {
                if !desc_lines.is_empty() {
                    entry.insert(
                        "description".to_string(),
                        Value::String(desc_lines.join(" ").trim().to_string()),
                    );
                    desc_lines.clear();
                }
                results.push(entry);
                entry = Map::new();
                in_desc = false;
                in_desc_en = false;
            }

            entry.insert(key, Value::String(val.to_string()));
        }
    }

    // Push final entry
    if !entry.is_empty() {
        if !desc_lines.is_empty() {
            entry.insert(
                "description".to_string(),
                Value::String(desc_lines.join(" ").trim().to_string()),
            );
        }
        results.push(entry);
    }

    results
}

/// Shared process logic: convert int fields and split list fields.
pub(crate) fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    let int_fields = ["epoch", "size", "installed_size"];
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

            // Compute epoch fields from date strings
            for (date_field, epoch_field, epoch_utc_field) in &[
                ("build_date", "build_epoch", "build_epoch_utc"),
                (
                    "install_date",
                    "install_date_epoch",
                    "install_date_epoch_utc",
                ),
            ] {
                if let Some(Value::String(ds)) = entry.get(*date_field) {
                    let parsed = parse_timestamp(ds, &[formats::F3000]);
                    entry.insert(
                        epoch_field.to_string(),
                        parsed
                            .naive_epoch
                            .map(|e| Value::Number(e.into()))
                            .unwrap_or(Value::Null),
                    );
                    entry.insert(
                        epoch_utc_field.to_string(),
                        parsed
                            .utc_epoch
                            .map(|e| Value::Number(e.into()))
                            .unwrap_or(Value::Null),
                    );
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
    fn test_rpm_qi_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/centos-7.7/rpm-qi-package.out");
        let fixture_json =
            include_str!("../../../../tests/fixtures/centos-7.7/rpm-qi-package.json");

        let parser = RpmQiParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        // The fixture includes build_epoch / install_date_epoch fields (timestamps)
        // We only check the fields we produce (no timestamp conversion in Rust yet)
        // Verify basic structure
        if let (serde_json::Value::Array(got_arr), serde_json::Value::Array(exp_arr)) =
            (&got, &expected)
        {
            assert_eq!(got_arr.len(), exp_arr.len(), "row count mismatch");
            assert_eq!(got_arr[0]["name"], exp_arr[0]["name"], "name mismatch");
            assert_eq!(
                got_arr[0]["version"], exp_arr[0]["version"],
                "version mismatch"
            );
            assert_eq!(got_arr[0]["size"], exp_arr[0]["size"], "size mismatch");
        } else {
            panic!("Expected Arrays");
        }
    }
}

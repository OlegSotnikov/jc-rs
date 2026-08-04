//! Parser for `pacman` command output (-Si, -Qi, etc.)

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_size_to_int, normalize_key, parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct PacmanParser;

static INFO: ParserInfo = ParserInfo {
    name: "pacman",
    argument: "--pacman",
    version: "1.0.0",
    description: "Converts `pacman` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &["pacman", "yay"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PACMAN_PARSER: PacmanParser = PacmanParser;

inventory::submit! {
    ParserEntry::new(&PACMAN_PARSER)
}

// Fields that accumulate multiple lines into a Vec<String>
const MULTILINE_FIELDS: &[&str] = &["required_by", "optional_deps", "backup_files"];

impl Parser for PacmanParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut entry: Map<String, Value> = Map::new();
        // raw multiline state: (key, list of lines)
        let mut multiline_key = String::new();
        let mut multiline_list: Vec<String> = Vec::new();

        let flush_ml = |entry: &mut Map<String, Value>, key: &str, list: &mut Vec<String>| {
            if !key.is_empty() && !list.is_empty() {
                // Store raw line list as JSON Array of strings (only if non-empty, matching jc)
                let arr: Vec<Value> = list.iter().map(|s| Value::String(s.clone())).collect();
                entry.insert(key.to_string(), Value::Array(arr));
                list.clear();
            } else {
                list.clear();
            }
        };

        for line in input.lines() {
            let line_trimmed = line.trim_end();
            if line_trimmed.trim().is_empty() {
                continue;
            }

            // A key : value line uses " : " as separator
            if let Some(pos) = line_trimmed.find(" : ") {
                let key_raw = line_trimmed[..pos].trim();
                let val_raw = line_trimmed[pos + 3..].trim();
                let key = normalize_key(key_raw);

                // Flush current multiline before starting new key
                if !multiline_key.is_empty() {
                    flush_ml(&mut entry, &multiline_key, &mut multiline_list);
                    multiline_key = String::new();
                }

                // New entry: "Name" or "Repository" key with existing data
                if (key == "name" || key == "repository") && entry.len() > 2 {
                    results.push(entry);
                    entry = Map::new();
                }

                if MULTILINE_FIELDS.contains(&key.as_str()) {
                    multiline_list = Vec::new();
                    if val_raw != "None" {
                        multiline_list.push(val_raw.to_string());
                    }
                    multiline_key = key;
                } else {
                    let value = if val_raw == "None" {
                        Value::Null
                    } else {
                        Value::String(val_raw.to_string())
                    };
                    entry.insert(key, value);
                }
            } else if !multiline_key.is_empty() {
                // Continuation line for multiline field
                multiline_list.push(line_trimmed.trim().to_string());
            }
        }

        // Push last entry
        if !entry.is_empty() {
            if !multiline_key.is_empty() {
                flush_ml(&mut entry, &multiline_key, &mut multiline_list);
            }
            results.push(entry);
        }

        let processed = process(results);
        Ok(ParseOutput::Array(processed))
    }
}

fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    // Fields where None→[] and value is space-split
    let split_fields = [
        "licenses",
        "groups",
        "provides",
        "depends_on",
        "conflicts_with",
        "replaces",
        "optional_for",
    ];
    // Fields with double-space separator (string → split on "  ")
    let two_space_fields = ["licenses", "validated_by"];
    let size_fields = ["download_size", "installed_size"];
    // Fields where each item's words are split (flatten all lines' words)
    // Note: validated_by is handled by two_space_fields (string), NOT here (array)
    let word_split_multiline = [
        "required_by",
        "groups",
        "provides",
        "depends_on",
        "conflicts_with",
        "replaces",
    ];

    raw.into_iter()
        .map(|mut entry| {
            // Process simple split fields (None → [], string → split by spaces)
            // Skip fields that are in two_space_fields — those will be handled below
            // (matches Python behavior: two_space_fields uses original val and overwrites split_fields)
            for field in &split_fields {
                if two_space_fields.contains(field) {
                    // Will be processed by two_space_fields below
                    // Just handle None → [] here
                    if let Some(Value::Null) = entry.get(*field) {
                        entry.insert(field.to_string(), Value::Array(Vec::new()));
                    }
                    continue;
                }
                match entry.get(*field).cloned() {
                    Some(Value::Null) => {
                        entry.insert(field.to_string(), Value::Array(Vec::new()));
                    }
                    Some(Value::String(s)) => {
                        let parts: Vec<Value> = s
                            .split_whitespace()
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        entry.insert(field.to_string(), Value::Array(parts));
                    }
                    _ => {}
                }
            }

            // Two-space fields: split by double space
            for field in &two_space_fields {
                if let Some(Value::String(s)) = entry.get(*field).cloned() {
                    let parts: Vec<Value> = s
                        .split("  ")
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    entry.insert(field.to_string(), Value::Array(parts));
                }
            }

            // Multiline word-split fields: each stored line's words get flattened
            for field in &word_split_multiline {
                if let Some(Value::Array(lines)) = entry.get(*field).cloned() {
                    let flat: Vec<Value> = lines
                        .iter()
                        .filter_map(|v| v.as_str())
                        .flat_map(|s| s.split_whitespace())
                        .map(|w| Value::String(w.to_string()))
                        .collect();
                    entry.insert(field.to_string(), Value::Array(flat));
                }
            }

            // optional_deps: each line is "name: description"
            if let Some(Value::Array(items)) = entry.get("optional_deps").cloned() {
                let parsed: Vec<Value> = items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|s| {
                        let mut parts = s.splitn(2, ": ");
                        let name = parts.next().unwrap_or("").trim().to_string();
                        let desc = parts.next().unwrap_or("").trim().to_string();
                        let mut obj: Map<String, Value> = Map::new();
                        obj.insert("name".to_string(), Value::String(name));
                        obj.insert("description".to_string(), Value::String(desc));
                        Value::Object(obj)
                    })
                    .collect();
                entry.insert("optional_deps".to_string(), Value::Array(parsed));
            }

            // Convert size fields and add _bytes (only if non-zero, matching jc's `if bts:`)
            for field in &size_fields {
                if let Some(Value::String(s)) = entry.get(*field).cloned()
                    && let Some(bytes) = convert_size_to_int(&s, true)
                    && bytes != 0
                {
                    let bytes_key = format!("{}_bytes", field);
                    entry.insert(bytes_key, Value::Number(bytes.into()));
                }
            }

            // Compute epoch fields from date strings (jc only adds _epoch, not _epoch_utc)
            for (date_field, epoch_field) in &[
                ("build_date", "build_date_epoch"),
                ("install_date", "install_date_epoch"),
            ] {
                if let Some(Value::String(ds)) = entry.get(*date_field) {
                    let parsed = parse_timestamp(ds, &[formats::F3000, formats::F3100]);
                    entry.insert(
                        epoch_field.to_string(),
                        parsed
                            .naive_epoch
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
    fn test_pacman_si_fixture() {
        let fixture_out =
            include_str!("../../../../tests/fixtures/generic/pacman--si-graphicsmagick.out");
        let fixture_json =
            include_str!("../../../../tests/fixtures/generic/pacman--si-graphicsmagick.json");

        let parser = PacmanParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();

        if let (serde_json::Value::Array(got_arr), serde_json::Value::Array(exp_arr)) =
            (&got, &expected)
        {
            assert_eq!(got_arr.len(), exp_arr.len(), "row count mismatch");
            assert_eq!(got_arr[0]["name"], exp_arr[0]["name"]);
            assert_eq!(got_arr[0]["version"], exp_arr[0]["version"]);
            assert_eq!(got_arr[0]["licenses"], exp_arr[0]["licenses"]);
            assert_eq!(got_arr[0]["depends_on"], exp_arr[0]["depends_on"]);
            assert_eq!(got_arr[0]["optional_deps"], exp_arr[0]["optional_deps"]);
            assert_eq!(
                got_arr[0]["download_size_bytes"],
                exp_arr[0]["download_size_bytes"]
            );
            assert_eq!(
                got_arr[0]["installed_size_bytes"],
                exp_arr[0]["installed_size_bytes"]
            );
        } else {
            panic!("Expected Arrays");
        }
    }
}

//! Parser for `zpool status` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct ZpoolStatusParser;

static INFO: ParserInfo = ParserInfo {
    name: "zpool_status",
    argument: "--zpool-status",
    version: "1.2.0",
    description: "Converts `zpool status` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["zpool status"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ZPOOL_STATUS_PARSER: ZpoolStatusParser = ZpoolStatusParser;

inventory::submit! {
    ParserEntry::new(&ZPOOL_STATUS_PARSER)
}

/// Build config list from a config block string (indented lines).
fn build_config_list(config_str: &str) -> Vec<Map<String, Value>> {
    let mut config_list = Vec::new();

    for line in config_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // Skip header line
        if line.trim().ends_with("READ WRITE CKSUM") {
            continue;
        }

        let stripped = line.trim();
        let _parts: Vec<&str> = stripped
            .splitn(6, char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();

        // Re-split properly (splitn on whitespace doesn't work well with multiple spaces)
        let parts = split_max_n(stripped, 6);

        let mut obj = Map::new();
        obj.insert(
            "name".to_string(),
            parts
                .first()
                .map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "state".to_string(),
            parts
                .get(1)
                .map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null),
        );

        // read, write, checksum -> integer or null
        for (i, field) in ["read", "write", "checksum"].iter().enumerate() {
            let val = parts.get(i + 2);
            let json_val = match val {
                Some(s) => convert_to_int(s).map(Value::from).unwrap_or(Value::Null),
                None => Value::Null,
            };
            obj.insert(field.to_string(), json_val);
        }

        // Optional errors field (6th token)
        if parts.len() == 6 {
            obj.insert("errors".to_string(), Value::String(parts[5].to_string()));
        }

        config_list.push(obj);
    }

    config_list
}

/// Split string into at most n whitespace-separated tokens.
fn split_max_n(s: &str, n: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = s.trim();
    for i in 0..n {
        if remaining.is_empty() {
            break;
        }
        if i == n - 1 {
            result.push(remaining.to_string());
            break;
        }
        // Find next whitespace boundary
        let end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        result.push(remaining[..end].to_string());
        remaining = remaining[end..].trim_start();
    }
    result
}

/// Parse a single pool string block into a pool object.
fn parse_pool_block(pool_str: &str) -> Option<Map<String, Value>> {
    let mut obj: Map<String, Value> = Map::new();
    let mut config_lines: Vec<String> = Vec::new();
    let mut in_config = false;
    let mut current_key: Option<String> = None;
    let mut current_val_lines: Vec<String> = Vec::new();

    fn flush_kv(obj: &mut Map<String, Value>, key: &Option<String>, val_lines: &[String]) {
        if let Some(k) = key {
            let val = if val_lines.len() == 1 {
                val_lines[0].trim().to_string()
            } else {
                val_lines
                    .iter()
                    .map(|l| l.trim())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            obj.insert(k.clone(), Value::String(val));
        }
    }

    for line in pool_str.lines() {
        // Config continuation lines (8+ spaces or tab-indented)
        if in_config {
            // Check if a new top-level key starts
            if !line.starts_with("        ") && !line.starts_with('\t') && !line.trim().is_empty() {
                // Check if this is a new key: value pattern at top level
                let stripped = line.trim();
                if let Some(colon_pos) = stripped.find(':') {
                    let candidate_key = stripped[..colon_pos].trim();
                    if !candidate_key.contains(' ') && !candidate_key.is_empty() {
                        // New key - end config section
                        in_config = false;
                        // Flush config
                        if !config_lines.is_empty() {
                            obj.insert(
                                "config".to_string(),
                                Value::Array(
                                    build_config_list(&config_lines.join("\n"))
                                        .into_iter()
                                        .map(Value::Object)
                                        .collect(),
                                ),
                            );
                            config_lines.clear();
                        }
                        // Start new key
                        let val_str = stripped[colon_pos + 1..].trim().to_string();
                        current_key = Some(candidate_key.to_string());
                        current_val_lines = if val_str.is_empty() {
                            vec![]
                        } else {
                            vec![val_str]
                        };
                        continue;
                    }
                }
            }
            if in_config {
                config_lines.push(line.to_string());
                continue;
            }
        }

        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }

        // Detect if this is a heavily-indented line (continuation of previous value)
        if line.starts_with("        ") || line.starts_with('\t') {
            if let Some(ref _k) = current_key {
                current_val_lines.push(line.to_string());
            }
            continue;
        }

        // Try to parse as "key: value" (possibly with leading spaces stripped)
        if let Some(colon_pos) = stripped.find(':') {
            let candidate_key = stripped[..colon_pos].trim();
            if !candidate_key.contains(' ') && !candidate_key.is_empty() {
                // Flush previous key
                flush_kv(&mut obj, &current_key, &current_val_lines);
                let val_str = stripped[colon_pos + 1..].trim().to_string();
                current_key = Some(candidate_key.to_string());
                current_val_lines = if val_str.is_empty() {
                    vec![]
                } else {
                    vec![val_str]
                };

                if candidate_key == "config" {
                    in_config = true;
                    current_key = None;
                    current_val_lines = vec![];
                }
                continue;
            }
        }

        // Continuation line
        if let Some(ref _k) = current_key {
            current_val_lines.push(stripped.to_string());
        }
    }

    // Flush last key
    flush_kv(&mut obj, &current_key, &current_val_lines);

    // Flush config if still open
    if in_config && !config_lines.is_empty() {
        obj.insert(
            "config".to_string(),
            Value::Array(
                build_config_list(&config_lines.join("\n"))
                    .into_iter()
                    .map(Value::Object)
                    .collect(),
            ),
        );
    }

    if obj.is_empty() { None } else { Some(obj) }
}

impl Parser for ZpoolStatusParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();
        let mut pool_str = String::new();

        for line in input.lines() {
            if line.is_empty() {
                continue;
            }

            if line.trim_start().starts_with("pool: ") {
                // Start of a new pool block
                if !pool_str.is_empty() {
                    if let Some(pool_obj) = parse_pool_block(&pool_str) {
                        result.push(pool_obj);
                    }
                    pool_str.clear();
                }
                // Append the pool line (strip leading whitespace for normalization)
                pool_str.push_str(&format!(
                    "pool: {}\n",
                    line.trim_start()["pool: ".len()..].trim()
                ));
                continue;
            }

            // Config section: heavily indented lines stay as-is
            if line.starts_with("        ") || line.starts_with('\t') {
                pool_str.push_str(line);
                pool_str.push('\n');
                continue;
            }

            // Lines starting with '/' get 2-space indent
            if line.starts_with('/') {
                pool_str.push_str("  ");
                pool_str.push_str(line);
                pool_str.push('\n');
                continue;
            }

            // Other lines: strip whitespace
            let stripped = line.trim();
            if !stripped.is_empty() {
                pool_str.push_str(stripped);
                pool_str.push('\n');
            }
        }

        // Flush last pool
        if !pool_str.is_empty()
            && let Some(pool_obj) = parse_pool_block(&pool_str)
        {
            result.push(pool_obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_pools(arr: &[Map<String, Value>], expected: &[serde_json::Value]) {
        assert_eq!(arr.len(), expected.len(), "pool count mismatch");
        for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
            for field in &["pool", "state", "errors"] {
                let g = got.get(*field).unwrap_or(&Value::Null);
                let e = exp.get(*field).unwrap_or(&Value::Null);
                if e != &Value::Null {
                    assert_eq!(g, e, "pool {} field '{}' mismatch", i, field);
                }
            }

            // Check config
            if let Some(exp_config) = exp.get("config").and_then(|v| v.as_array()) {
                let got_config = got.get("config").and_then(|v| v.as_array());
                assert!(got_config.is_some(), "pool {} missing config", i);
                let got_config = got_config.unwrap();
                assert_eq!(
                    got_config.len(),
                    exp_config.len(),
                    "pool {} config length mismatch",
                    i
                );
                for (j, (gc, ec)) in got_config.iter().zip(exp_config.iter()).enumerate() {
                    for field in &["name", "state", "read", "write", "checksum"] {
                        assert_eq!(
                            gc.get(*field).unwrap_or(&Value::Null),
                            ec.get(*field).unwrap_or(&Value::Null),
                            "pool {} config[{}] field '{}' mismatch",
                            i,
                            j,
                            field
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_zpool_status_v() {
        let input = include_str!("../../../../tests/fixtures/generic/zpool-status-v.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/zpool-status-v.json"
        ))
        .unwrap();
        let parser = ZpoolStatusParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => check_pools(&arr, &expected),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zpool_status_spares() {
        let input = include_str!("../../../../tests/fixtures/generic/zpool-status-spares.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/zpool-status-spares.json"
        ))
        .unwrap();
        let parser = ZpoolStatusParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => check_pools(&arr, &expected),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zpool_status_empty() {
        let parser = ZpoolStatusParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}

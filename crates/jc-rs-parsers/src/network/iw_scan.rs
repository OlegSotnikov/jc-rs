//! Parser for `iw dev <device> scan` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct IwScanParser;

static INFO: ParserInfo = ParserInfo {
    name: "iw_scan",
    argument: "--iw-scan",
    version: "1.0.0",
    description: "Converts `iw dev <device> scan` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IW_SCAN_PARSER: IwScanParser = IwScanParser;
inventory::submit! { ParserEntry::new(&IW_SCAN_PARSER) }

fn try_convert(s: &str) -> Value {
    // Try int
    if let Ok(n) = s.parse::<i64>() {
        return Value::Number(n.into());
    }
    // Try float
    if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(f) {
                return Value::Number(n);
            }
        }
    }
    Value::String(s.to_string())
}

fn normalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace('/', "_")
        .replace(':', "")
        .replace('(', "")
        .replace(')', "")
        .replace('.', "")
}

impl Parser for IwScanParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result: Vec<Map<String, Value>> = Vec::new();
        let mut current: Option<Map<String, Value>> = None;
        let mut last_key: Option<String> = None;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // BSS line starts a new BSS entry
            if line.starts_with("BSS ") {
                if let Some(bss) = current.take() {
                    result.push(bss);
                }
                let mut obj = Map::new();
                // BSS aa:bb:cc:dd:ee:ff(on wlan0)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let bssid = parts[1].split('(').next().unwrap_or(parts[1]);
                    obj.insert("bssid".to_string(), Value::String(bssid.to_string()));

                    // Extract interface if present
                    if let Some(start) = parts[1].find("on ") {
                        let iface = &parts[1][start + 3..].trim_end_matches(')');
                        obj.insert("interface".to_string(), Value::String(iface.to_string()));
                    }
                }
                current = Some(obj);
                last_key = None;
                continue;
            }

            if let Some(ref mut obj) = current {
                // Check if this is a key: value line
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = &trimmed[..colon_pos];
                    let value = trimmed[colon_pos + 1..].trim();

                    let key_n = normalize_key(key);
                    if key_n.is_empty() {
                        continue;
                    }

                    // Handle special cases
                    if key_n == "supported_rates" || key_n == "extended_supported_rates" {
                        // Parse as array of floats
                        let rates: Vec<Value> = value
                            .split_whitespace()
                            .filter_map(|s| s.parse::<f64>().ok())
                            .filter_map(|f| serde_json::Number::from_f64(f).map(Value::Number))
                            .collect();
                        obj.insert(key_n.clone(), Value::Array(rates));
                        last_key = Some(key_n);
                    } else {
                        obj.insert(key_n.clone(), try_convert(value));
                        last_key = Some(key_n);
                    }
                } else if !trimmed.is_empty() {
                    // Continuation line - append to last key or create new entry
                    // Many iw scan sub-fields are indented continuation lines
                    // Example: "\t\t * something" - treat as key-value
                    let clean = trimmed.trim_start_matches('*').trim();
                    if let Some(colon_pos) = clean.find(':') {
                        let key = &clean[..colon_pos];
                        let value = clean[colon_pos + 1..].trim();
                        let key_n = normalize_key(key);
                        if !key_n.is_empty() {
                            obj.insert(key_n.clone(), try_convert(value));
                            last_key = Some(key_n);
                        }
                    } else if !clean.is_empty() {
                        // Just a value line - might be sub-key
                        let key_n = normalize_key(clean);
                        if !key_n.is_empty() && !obj.contains_key(&key_n) {
                            obj.insert(key_n.clone(), Value::String(clean.to_string()));
                        }
                    }
                }
            }
        }

        if let Some(bss) = current {
            result.push(bss);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_iw_scan_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iw-scan0.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iw-scan0.json"
        ))
        .unwrap();
        let result = IwScanParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iw_scan_empty() {
        let result = IwScanParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_iw_scan_registered() {
        assert!(jc_rs_core::registry::find_parser("iw_scan").is_some());
    }
}

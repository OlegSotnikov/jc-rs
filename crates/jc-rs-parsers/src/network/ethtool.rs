//! Parser for `ethtool` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct EthtoolParser;

static INFO: ParserInfo = ParserInfo {
    name: "ethtool",
    argument: "--ethtool",
    version: "1.2.0",
    description: "Converts `ethtool` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["ethtool"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ETHTOOL_PARSER: EthtoolParser = EthtoolParser;
inventory::submit! { ParserEntry::new(&ETHTOOL_PARSER) }

/// Convert speed string like "1000Mb/s" to bps using jc's convert_size_to_int logic.
fn speed_to_bps(s: &str) -> Option<i64> {
    let s = s.trim();
    if s == "Unknown!" || s == "unknown" || s.is_empty() {
        return None;
    }
    let (num_str, unit) = {
        let pos = s.find(|c: char| c.is_alphabetic())?;
        (&s[..pos], &s[pos..])
    };
    let num: f64 = num_str.parse().ok()?;
    let multiplier: f64 = match unit.to_lowercase().as_str() {
        "mb/s" | "mbit/s" | "mbps" | "mb" => 1_000_000.0,
        "gb/s" | "gbit/s" | "gbps" | "gb" => 1_000_000_000.0,
        "kb/s" | "kbit/s" | "kbps" | "kb" => 1_000.0,
        "tb/s" | "tbit/s" | "tbps" | "tb" => 1_000_000_000_000.0,
        _ => return None,
    };
    Some((num * multiplier) as i64)
}

/// Match jc's convert_to_bool: only 'y', 'yes', 'true', '*' are truthy.
/// Numbers are bool'd (0 → false, nonzero → true).
/// Everything else is false.
fn jc_convert_to_bool(s: &str) -> bool {
    let stripped = s.trim();
    // Try to parse as float first (jc strips non-numeric chars, but for these fields
    // we just try direct parse)
    if let Ok(f) = stripped.parse::<f64>() {
        return f != 0.0;
    }
    matches!(stripped.to_lowercase().as_str(), "y" | "yes" | "true" | "*")
}

fn normalize_key(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace('/', "_")
        .replace('(', "")
        .replace(')', "")
        .replace(':', "")
        .replace('.', "")
        .replace('[', "")
        .replace(']', "")
}

/// Parse "Settings for ..." format (default ethtool output)
fn parse_default(input: &str) -> Map<String, Value> {
    let mut raw_output = Map::new();

    let mut supported_ports: Vec<String> = Vec::new();
    let mut supported_link_modes: Vec<String> = Vec::new();
    let mut supported_fec_modes: Vec<String> = Vec::new();
    let mut advertised_link_modes: Vec<String> = Vec::new();
    let mut link_partner_advertised_link_modes: Vec<String> = Vec::new();
    let mut advertised_fec_modes: Vec<String> = Vec::new();
    let mut current_message_level: Vec<String> = Vec::new();

    // Track which list keys had "Not reported" as their value
    let mut not_reported: std::collections::HashSet<&str> = std::collections::HashSet::new();

    let mut mode = String::new(); // current continuation mode

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // "Settings for eth0:"
        if line.trim().starts_with("Settings for ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].trim_end_matches(':');
                raw_output.insert("name".to_string(), Value::String(name.to_string()));
            }
            continue;
        }

        // Expand tabs for indent detection (jc uses 4 spaces per tab)
        let data_line = line.replace('\t', "    ");

        // Check if this is a continuation line (9+ spaces indent, no colon before content)
        if !data_line.starts_with("         ") {
            // 9 spaces
            mode.clear();
        }

        // Handle explicit field lines
        if line.contains("Supported ports:") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            // Strip brackets: "[ TP MII ]" → "TP MII"
            let inner = val.trim_start_matches('[').trim_end_matches(']').trim();
            supported_ports.extend(inner.split_whitespace().map(|s| s.to_string()));
            continue;
        }

        if line.contains("Supported link modes:") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            if val.to_lowercase() == "not reported" {
                not_reported.insert("supported_link_modes");
            } else {
                supported_link_modes.extend(val.split_whitespace().map(|s| s.to_string()));
            }
            mode = "supported_link_modes".to_string();
            continue;
        }

        if line.contains("Supported FEC modes:") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            if val.to_lowercase() == "not reported" {
                not_reported.insert("supported_fec_modes");
            } else {
                supported_fec_modes.extend(val.split_whitespace().map(|s| s.to_string()));
            }
            mode = "supported_fec_modes".to_string();
            continue;
        }

        if line.contains("Advertised link modes:") && !line.contains("Link partner") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            if val.to_lowercase() == "not reported" {
                not_reported.insert("advertised_link_modes");
            } else {
                advertised_link_modes.extend(val.split_whitespace().map(|s| s.to_string()));
            }
            mode = "advertised_link_modes".to_string();
            continue;
        }

        if line.contains("Link partner advertised link modes:") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            if val.to_lowercase() == "not reported" {
                not_reported.insert("link_partner_advertised_link_modes");
            } else {
                link_partner_advertised_link_modes
                    .extend(val.split_whitespace().map(|s| s.to_string()));
            }
            mode = "link_partner_advertised_link_modes".to_string();
            continue;
        }

        if line.contains("Advertised FEC modes:") && !line.contains("Link partner") {
            let (_, val) = line.split_once(':').unwrap();
            let val = val.trim();
            if val.to_lowercase() == "not reported" {
                not_reported.insert("advertised_fec_modes");
            } else {
                advertised_fec_modes.extend(val.split_whitespace().map(|s| s.to_string()));
            }
            mode = "advertised_fec_modes".to_string();
            continue;
        }

        if line.contains("Current message level:") {
            let (_, val) = line.split_once(':').unwrap();
            current_message_level.push(val.trim().to_string());
            mode = "current_message_level".to_string();
            continue;
        }

        // Handle continuation lines
        match mode.as_str() {
            "supported_link_modes" => {
                supported_link_modes.extend(line.trim().split_whitespace().map(|s| s.to_string()));
                continue;
            }
            "supported_fec_modes" => {
                supported_fec_modes.extend(line.trim().split_whitespace().map(|s| s.to_string()));
                continue;
            }
            "advertised_link_modes" => {
                advertised_link_modes.extend(line.trim().split_whitespace().map(|s| s.to_string()));
                continue;
            }
            "link_partner_advertised_link_modes" => {
                link_partner_advertised_link_modes
                    .extend(line.trim().split_whitespace().map(|s| s.to_string()));
                continue;
            }
            "advertised_fec_modes" => {
                advertised_fec_modes.extend(line.trim().split_whitespace().map(|s| s.to_string()));
                continue;
            }
            "current_message_level" => {
                current_message_level.push(line.trim().to_string());
                continue;
            }
            _ => {}
        }

        // Generic key: value line
        if let Some((key, val)) = line.trim().split_once(':') {
            let key_n = normalize_key(key);
            let val = val.trim();
            raw_output.insert(key_n, Value::String(val.to_string()));
        }
    }

    // Insert list values
    let list_vals: Vec<(&Vec<String>, &str)> = vec![
        (&supported_ports, "supported_ports"),
        (&supported_link_modes, "supported_link_modes"),
        (&supported_fec_modes, "supported_fec_modes"),
        (&advertised_link_modes, "advertised_link_modes"),
        (
            &link_partner_advertised_link_modes,
            "link_partner_advertised_link_modes",
        ),
        (&advertised_fec_modes, "advertised_fec_modes"),
        (&current_message_level, "current_message_level"),
    ];

    for (list, key) in list_vals {
        // Check if raw_output had "Not reported" stored as string
        if let Some(Value::String(s)) = raw_output.get(key as &str) {
            if s.to_lowercase() == "not reported" {
                raw_output.insert(key.to_string(), Value::Array(vec![]));
                continue;
            }
        }
        if not_reported.contains(key) {
            raw_output.insert(key.to_string(), Value::Array(vec![]));
        } else if !list.is_empty() {
            raw_output.insert(
                key.to_string(),
                Value::Array(list.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
    }

    raw_output
}

/// Post-processing: convert booleans, speed, and unit values (matching jc's _process)
fn process(obj: &mut Map<String, Value>) {
    let bool_keys = [
        "supports_auto_negotiation",
        "advertised_auto_negotiation",
        "auto_negotiation",
        "link_detected",
        "advertised_pause_frame_use",
    ];

    // Convert speed to speed_bps
    if let Some(Value::String(speed)) = obj.get("speed") {
        if let Some(bps) = speed_to_bps(speed) {
            obj.insert("speed_bps".to_string(), Value::Number(bps.into()));
        }
    }

    // Convert boolean fields
    for key in &bool_keys {
        if let Some(Value::String(val)) = obj.get(*key) {
            let b = jc_convert_to_bool(val);
            obj.insert(key.to_string(), Value::Bool(b));
        }
    }

    // Convert unit values (degrees, power, voltage, current)
    let degrees_re =
        regex::Regex::new(r"(?P<deg_c>.*?) degrees C / (?P<deg_f>.*?) degrees F").unwrap();
    let power_re = regex::Regex::new(r"(?P<pow_mw>.*?) mW / (?P<pow_dbm>.*?) dBm").unwrap();

    let keys: Vec<String> = obj.keys().cloned().collect();
    for key in keys {
        let val = match obj.get(&key) {
            Some(Value::String(s)) => s.clone(),
            _ => continue,
        };

        // degrees C / degrees F
        if let Some(caps) = degrees_re.captures(&val) {
            if let (Ok(c), Ok(f)) = (
                caps["deg_c"].trim().parse::<f64>(),
                caps["deg_f"].trim().parse::<f64>(),
            ) {
                obj.remove(&key);
                if let Some(n) = serde_json::Number::from_f64(c) {
                    obj.insert(format!("{}_celsius", key), Value::Number(n));
                }
                if let Some(n) = serde_json::Number::from_f64(f) {
                    obj.insert(format!("{}_farenheit", key), Value::Number(n));
                }
                continue;
            }
        }

        // mW / dBm
        if let Some(caps) = power_re.captures(&val) {
            if let (Ok(mw), Ok(dbm)) = (
                caps["pow_mw"].trim().parse::<f64>(),
                caps["pow_dbm"].trim().parse::<f64>(),
            ) {
                obj.remove(&key);
                if let Some(n) = serde_json::Number::from_f64(mw) {
                    obj.insert(format!("{}_mw", key), Value::Number(n));
                }
                if let Some(n) = serde_json::Number::from_f64(dbm) {
                    obj.insert(format!("{}_dbm", key), Value::Number(n));
                }
                continue;
            }
        }

        // Voltage: ends with " V"
        if val.ends_with(" V") {
            if let Some(f) = convert_to_float(&val) {
                obj.remove(&key);
                if let Some(n) = serde_json::Number::from_f64(f) {
                    obj.insert(format!("{}_v", key), Value::Number(n));
                }
                continue;
            }
        }

        // Current: ends with " mA"
        if val.ends_with(" mA") {
            if let Some(f) = convert_to_float(&val) {
                obj.remove(&key);
                if let Some(n) = serde_json::Number::from_f64(f) {
                    obj.insert(format!("{}_ma", key), Value::Number(n));
                }
                continue;
            }
        }
    }
}

/// Mimic jc.utils.convert_to_float: strip non-numeric chars and parse as f64
fn convert_to_float(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-' || *c == '.')
        .collect();
    cleaned.parse::<f64>().ok()
}

/// Parse module information format ("Identifier" lines)
fn parse_module_info(input: &str) -> Map<String, Value> {
    let mut raw_output = Map::new();
    let mut previous_key = String::new();
    let mut multi_value: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (key, val) = match trimmed.split_once(':') {
            Some((k, v)) => (normalize_key(k), v.trim().to_string()),
            None => continue,
        };
        if key.is_empty() {
            continue;
        }

        if key == previous_key {
            multi_value.push(val);
            continue;
        }

        // Flush previous
        if !previous_key.is_empty() {
            if multi_value.len() > 1 {
                raw_output.insert(
                    previous_key.clone(),
                    Value::Array(
                        multi_value
                            .iter()
                            .map(|s| Value::String(s.clone()))
                            .collect(),
                    ),
                );
            } else if multi_value.len() == 1 {
                raw_output.insert(previous_key.clone(), Value::String(multi_value[0].clone()));
            } else {
                raw_output.insert(previous_key.clone(), Value::String(String::new()));
            }
        }

        multi_value.clear();
        multi_value.push(val);
        previous_key = key;
    }

    // Flush last key
    if !previous_key.is_empty() {
        if multi_value.len() > 1 {
            raw_output.insert(
                previous_key,
                Value::Array(
                    multi_value
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        } else if multi_value.len() == 1 {
            raw_output.insert(previous_key, Value::String(multi_value[0].clone()));
        } else {
            raw_output.insert(previous_key, Value::String(String::new()));
        }
    }

    raw_output
}

impl Parser for EthtoolParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let first_meaningful = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

        let mut obj = if first_meaningful.trim().starts_with("Identifier") {
            parse_module_info(input)
        } else {
            parse_default(input)
        };

        process(&mut obj);

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ethtool_default1_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/ethtool--default1.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/ethtool--default1.json"
        ))
        .unwrap();
        let result = EthtoolParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ethtool_module_info_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/ethtool--module-info.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/ethtool--module-info.json"
        ))
        .unwrap();
        let result = EthtoolParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ethtool_empty() {
        let result = EthtoolParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_ethtool_registered() {
        assert!(jc_rs_core::registry::find_parser("ethtool").is_some());
    }
}

//! Parser for `nmcli` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct NmcliParser;

static INFO: ParserInfo = ParserInfo {
    name: "nmcli",
    argument: "--nmcli",
    version: "1.2.0",
    description: "Converts `nmcli` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["nmcli"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static NMCLI_PARSER: NmcliParser = NmcliParser;
inventory::submit! { ParserEntry::new(&NMCLI_PARSER) }

fn normalize_key(key: &str) -> String {
    key.replace([' ', '.', '['], "_")
        .replace(']', "")
        .replace('-', "_")
        .replace("GENERAL_", "")
        .to_lowercase()
}

fn normalize_value(value: &str) -> Value {
    let v = value.trim();
    if v == "--" {
        return Value::Null;
    }
    // Strip surrounding quotes
    let v = if v.starts_with('"') && v.ends_with('"') && v.len() > 1 {
        &v[1..v.len() - 1]
    } else {
        v
    };
    Value::String(v.to_string())
}

fn try_numeric(v: &Value) -> Value {
    if let Some(s) = v.as_str() {
        // Try int first
        if let Ok(n) = s.parse::<i64>() {
            return Value::Number(n.into());
        }
        // Try float (only if has dot)
        if s.contains('.')
            && let Ok(f) = s.parse::<f64>()
            && let Some(n) = serde_json::Number::from_f64(f)
        {
            return Value::Number(n);
        }
    }
    v.clone()
}

fn split_routes(value: &str) -> Map<String, Value> {
    // "dst = 192.168.71.0/24, nh = 0.0.0.0, mt = 100"
    let mut obj = Map::new();
    for part in value.split(',') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 {
            let k = kv[0].trim().to_string();
            let v = kv[1].trim().to_string();
            let val = try_numeric(&Value::String(v));
            obj.insert(k, val);
        }
    }
    obj
}

fn split_options(value: &str) -> Map<String, Value> {
    // "ip_address = 192.168.71.180"
    let mut obj = Map::new();
    let kv: Vec<&str> = value.splitn(2, '=').collect();
    if kv.len() == 2 {
        obj.insert("name".to_string(), Value::String(kv[0].trim().to_string()));
        let v = kv[1].trim().to_string();
        obj.insert("value".to_string(), try_numeric(&Value::String(v)));
    }
    obj
}

/// Extract text from trailing parenthetical: "100 (connected)" → Some("connected")
fn extract_paren_text(s: &str) -> Option<&str> {
    if s.contains('(') && s.ends_with(')') {
        // Must match \w+ inside the final parens
        let paren_start = s.rfind('(')?;
        let inner = &s[paren_start + 1..s.len() - 1];
        // Only if inner is word chars (no spaces or special chars) - matches Python's \w+ regex
        if inner.chars().all(|c| c.is_alphanumeric() || c == '_') && !inner.is_empty() {
            return Some(inner);
        }
    }
    None
}

/// Strip trailing parenthetical: "-1 (default)" → "-1"
fn remove_paren_text(s: &str) -> &str {
    if let Some(pos) = s.rfind('(')
        && s.ends_with(')')
    {
        let trimmed = s[..pos].trim_end();
        return trimmed;
    }
    s
}

/// Insert key+value into item, with parenthetical text handling
fn insert_kv(item: &mut Map<String, Value>, key_n: String, value_n: Value) {
    // Check for parenthetical text in the value
    if let Some(s) = value_n.as_str()
        && let Some(text) = extract_paren_text(s)
    {
        let text = text.to_string();
        // Strip paren from base value and try numeric
        let base = remove_paren_text(s);
        let base_val = try_numeric(&Value::String(base.to_string()));
        item.insert(key_n.clone(), base_val);
        item.insert(key_n + "_text", Value::String(text));
        return;
    }
    item.insert(key_n, value_n);
}

/// Parse device show format: "GENERAL.DEVICE: ens33" key:value pairs
fn parse_device_show(data: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut item: Map<String, Value> = Map::new();
    let mut current_item = String::new();

    for line in data.lines().filter(|l| !l.trim().is_empty()) {
        let kv: Vec<&str> = line.splitn(2, ':').collect();
        if kv.len() < 2 {
            continue;
        }
        let key = kv[0].trim();
        let value = kv[1].trim();

        let key_n = normalize_key(key);
        let value_n = normalize_value(value);

        if !item.is_empty() && key_n.contains("device") {
            if let Some(s) = value_n.as_str()
                && s != current_item
            {
                raw_output.push(item.clone());
                item = Map::new();
                current_item = s.to_string();
            }
        } else if item.is_empty()
            && let Some(s) = value_n.as_str()
        {
            current_item = s.to_string();
        }

        let final_value = if key_n.contains("_route_")
            && key_n
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            if let Some(s) = value_n.as_str() {
                Value::Object(split_routes(s))
            } else {
                value_n
            }
        } else if key_n.contains("_option_")
            && key_n
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            if let Some(s) = value_n.as_str() {
                Value::Object(split_options(s))
            } else {
                value_n
            }
        } else {
            value_n
        };

        insert_kv(&mut item, key_n, final_value);
    }

    if !item.is_empty() {
        raw_output.push(item);
    }

    raw_output
}

/// Parse connection show -x format: "connection.id: ..." key:value pairs with team.config JSON
fn parse_connection_show_x(data: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut item: Map<String, Value> = Map::new();
    let mut in_team_config = false;
    let mut team_config_value: Vec<String> = Vec::new();
    let mut team_config_key = String::new();

    for line in data.lines().filter(|l| !l.trim().is_empty()) {
        // Handle team.config and team-port.config multi-line JSON
        if line.starts_with("team.config:") || line.starts_with("team-port.config:") {
            let kv: Vec<&str> = line.splitn(2, ':').collect();
            if kv.len() < 2 {
                continue;
            }
            let key = kv[0].trim();
            let value = kv[1].trim();
            team_config_key = normalize_key(key);
            team_config_value.clear();

            if value == "--" {
                item.insert(team_config_key.clone(), Value::Null);
                in_team_config = false;
            } else {
                in_team_config = true;
                team_config_value.push(value.to_string());
                item.insert(team_config_key.clone(), Value::Object(Map::new()));
            }
            continue;
        }

        // If inside team config JSON, accumulate until we hit a team. line
        let starts_with_team = line.starts_with("team.") || line.starts_with("team-port.");
        if !starts_with_team && in_team_config {
            team_config_value.push(line.trim().to_string());
            continue;
        }

        // Flush accumulated team config JSON
        if in_team_config && !team_config_value.is_empty() {
            let json_str = team_config_value.join("");
            if let Ok(v) = serde_json::from_str::<Value>(&json_str) {
                item.insert(team_config_key.clone(), v);
            }
            team_config_value.clear();
            in_team_config = false;
        }
        if !starts_with_team && !in_team_config && team_config_value.is_empty() {
            // already flushed
        }
        in_team_config = false;

        let kv: Vec<&str> = line.splitn(2, ':').collect();
        if kv.len() < 2 {
            continue;
        }
        let key = kv[0].trim();
        let value = kv[1].trim();
        let key_n = normalize_key(key);
        let value_n = normalize_value(value);

        let final_value = if key_n.contains("_route_")
            && key_n
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            if let Some(s) = value_n.as_str() {
                Value::Object(split_routes(s))
            } else {
                value_n
            }
        } else if key_n.contains("_option_")
            && key_n
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            if let Some(s) = value_n.as_str() {
                Value::Object(split_options(s))
            } else {
                value_n
            }
        } else {
            value_n
        };

        insert_kv(&mut item, key_n, final_value);
    }

    // Flush any remaining team config
    if in_team_config && !team_config_value.is_empty() {
        let json_str = team_config_value.join("");
        if let Ok(v) = serde_json::from_str::<Value>(&json_str) {
            item.insert(team_config_key, v);
        }
    }

    if !item.is_empty() {
        raw_output.push(item);
    }

    raw_output
}

/// Parse general permissions format: pivot table where each permission becomes a key
fn parse_general_permissions(data: &str) -> Vec<Map<String, Value>> {
    let mut output_dict: Map<String, Value> = Map::new();

    for line in data.lines().filter(|l| !l.trim().is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0];
        let value = parts[1];
        let key_n = normalize_key(key);
        // Skip the header row
        if key_n == "permission" {
            continue;
        }
        output_dict.insert(key_n, Value::String(value.to_string()));
    }

    vec![output_dict]
}

/// Parse general/connection/device table format (sparse table)
fn parse_table_format(data: &str) -> Vec<Map<String, Value>> {
    let lines: Vec<&str> = data.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let header_line = lines[0];
    let header_bytes = header_line.as_bytes();
    let mut col_starts: Vec<usize> = Vec::new();
    let mut in_word = false;
    for (i, &b) in header_bytes.iter().enumerate() {
        if b != b' ' && !in_word {
            col_starts.push(i);
            in_word = true;
        } else if b == b' ' {
            in_word = false;
        }
    }

    let headers: Vec<String> = col_starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = if i + 1 < col_starts.len() {
                col_starts[i + 1]
            } else {
                header_line.len()
            };
            let end = end.min(header_line.len());
            let start = start.min(header_line.len());
            header_line[start..end]
                .trim()
                .to_lowercase()
                .replace([' ', '-'], "_")
        })
        .collect();

    let mut result = Vec::new();
    for line in &lines[1..] {
        if line.is_empty() {
            continue;
        }
        let mut obj = Map::new();
        for (i, &start) in col_starts.iter().enumerate() {
            let end = if i + 1 < col_starts.len() {
                col_starts[i + 1]
            } else {
                line.len()
            };
            let start = start.min(line.len());
            let end = end.min(line.len());
            let val = line[start..end].trim();
            let key = &headers[i];
            if val == "--" {
                obj.insert(key.clone(), Value::Null);
            } else {
                obj.insert(key.clone(), Value::String(val.to_string()));
            }
        }
        result.push(obj);
    }

    result
}

fn apply_numeric_conversion(items: &mut [Map<String, Value>]) {
    for item in items.iter_mut() {
        let keys: Vec<String> = item.keys().cloned().collect();
        for key in keys {
            if let Some(v) = item.get(&key)
                && let Some(s) = v.as_str()
            {
                let converted = try_numeric(&Value::String(s.to_string()));
                item.insert(key, converted);
            }
        }
    }
}

impl Parser for NmcliParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Detect format by checking the beginning of the data (matches Python logic)
        let first_line = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

        let mut result = if first_line.starts_with("GENERAL.DEVICE") {
            parse_device_show(input)
        } else if first_line.starts_with("connection.id:") {
            parse_connection_show_x(input)
        } else if first_line.starts_with("PERMISSION ") {
            parse_general_permissions(input)
        } else {
            parse_table_format(input)
        };

        apply_numeric_conversion(&mut result);

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_nmcli_device_show_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/nmcli-device-show.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/nmcli-device-show.json"
        ))
        .unwrap();
        let result = NmcliParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_nmcli_device_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/nmcli-device.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/nmcli-device.json"
        ))
        .unwrap();
        let result = NmcliParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_nmcli_empty() {
        let result = NmcliParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_nmcli_registered() {
        assert!(jc_rs_core::registry::find_parser("nmcli").is_some());
    }
}

//! Parser for `upower` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct UpowerParser;

static INFO: ParserInfo = ParserInfo {
    name: "upower",
    argument: "--upower",
    version: "1.4.0",
    description: "Converts `upower` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["upower"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UPOWER_PARSER: UpowerParser = UpowerParser;

inventory::submit! {
    ParserEntry::new(&UPOWER_PARSER)
}

impl Parser for UpowerParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_upower(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn normalize_key(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
        .replace('(', "")
        .replace(')', "")
}

fn parse_upower(input: &str) -> Vec<Map<String, Value>> {
    let mut output: Vec<Map<String, Value>> = Vec::new();
    let mut device_obj: Option<Map<String, Value>> = None;
    let mut history_key: String = String::new();
    let mut history_list: Vec<Value> = Vec::new();

    // Detect -i format (no Device: header, starts with key-value directly)
    let first_meaningful = input.lines().find(|l| !l.trim().is_empty());
    let is_info_format = first_meaningful
        .map(|l| !l.starts_with("Device:") && !l.starts_with("Daemon:") && l.contains(':'))
        .unwrap_or(false);
    if is_info_format {
        device_obj = Some(Map::new());
    }

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Top-level device or daemon header
        if line.starts_with("Device:") || line.starts_with("Daemon:") {
            // Flush previous device
            if let Some(mut dev) = device_obj.take() {
                if !history_key.is_empty() && !history_list.is_empty() {
                    dev.insert(history_key.clone(), Value::Array(history_list.clone()));
                    history_list.clear();
                    history_key.clear();
                }
                output.push(dev);
            }

            if line.starts_with("Device:") {
                let device_name = line["Device:".len()..].trim().to_string();
                let mut dev = Map::new();
                dev.insert("type".to_string(), Value::String("Device".to_string()));
                dev.insert("device_name".to_string(), Value::String(device_name));
                device_obj = Some(dev);
            } else {
                // Daemon
                let mut dev = Map::new();
                dev.insert("type".to_string(), Value::String("Daemon".to_string()));
                device_obj = Some(dev);
            }
            continue;
        }

        // History detail lines (4-space indent, no colon)
        if line.starts_with("    ") && !line.contains(':') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let mut hist_obj = Map::new();
                hist_obj.insert("time".to_string(), Value::String(parts[0].to_string()));
                hist_obj.insert(
                    "percent_charged".to_string(),
                    Value::String(parts[1].to_string()),
                );
                hist_obj.insert("status".to_string(), Value::String(parts[2].to_string()));
                history_list.push(Value::Object(hist_obj));

                if let Some(ref mut dev) = device_obj {
                    dev.insert(history_key.clone(), Value::Array(history_list.clone()));
                }
            }
            continue;
        }

        // History section headers (2-space indent, no colon in key part)
        if line.starts_with("  History (charge):") || line.starts_with("  History (rate):") {
            // Flush previous history
            if let Some(ref mut dev) = device_obj {
                if !history_key.is_empty() && !history_list.is_empty() {
                    dev.insert(history_key.clone(), Value::Array(history_list.clone()));
                }
            }
            history_list.clear();

            if line.contains("(charge)") {
                history_key = "history_charge".to_string();
            } else {
                history_key = "history_rate".to_string();
            }

            if let Some(ref mut dev) = device_obj {
                dev.insert(history_key.clone(), Value::Array(Vec::new()));
            }
            continue;
        }

        // 4-space indent with colon: detail lines
        if line.starts_with("    ") && line.contains(':') {
            if let Some(colon_pos) = line.find(':') {
                let key = normalize_key(&line[..colon_pos]);
                let val = line[colon_pos + 1..].trim().to_string();

                if let Some(ref mut dev) = device_obj {
                    if let Some(Value::Object(detail)) = dev.get_mut("detail") {
                        detail.insert(key, Value::String(val));
                    }
                }
            }
            continue;
        }

        // 2-space indent with colon: top-level detail key-value
        if line.starts_with("  ") && line.contains(':') {
            if let Some(colon_pos) = line.find(':') {
                let key = normalize_key(&line[..colon_pos]);
                let val = line[colon_pos + 1..].trim().to_string();

                if let Some(ref mut dev) = device_obj {
                    dev.insert(key, Value::String(val));
                }
            }
            continue;
        }

        // 2-space indent without colon: detail type header (e.g. "  battery")
        if line.starts_with("  ") && !line.contains(':') {
            let detail_type = trimmed.to_string();
            if let Some(ref mut dev) = device_obj {
                let mut detail = Map::new();
                detail.insert("type".to_string(), Value::String(detail_type));
                dev.insert("detail".to_string(), Value::Object(detail));
            }
            continue;
        }
    }

    // Flush last device
    if let Some(mut dev) = device_obj {
        if !history_key.is_empty() && !history_list.is_empty() {
            dev.insert(history_key, Value::Array(history_list));
        }
        output.push(dev);
    }

    // Post-process: convert bool and numeric fields
    for entry in &mut output {
        process_device(entry);
    }

    output
}

fn convert_to_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "yes" | "true" | "1")
}

fn process_device(dev: &mut Map<String, Value>) {
    let bool_keys = [
        "power_supply",
        "has_history",
        "has_statistics",
        "on_battery",
        "lid_is_closed",
        "lid_is_present",
    ];

    // Process updated field: extract time string and seconds_ago
    if let Some(Value::String(updated)) = dev.get("updated").cloned() {
        // Format: "Thu 11 Mar 2021 06:28:08 PM UTC (441975 seconds ago)"
        let cleaned = updated.replace('(', "").replace(')', "");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        if parts.len() >= 3 {
            // Last 3 tokens are: <N> seconds ago
            let n = parts.len();
            if n >= 3 && parts[n - 1] == "ago" && parts[n - 2] == "ago".to_string().as_str() {
                // fallthrough
            }
            // Find "seconds ago" at end
            let seconds_ago_str = if parts.len() >= 3 {
                let last3 = &parts[parts.len() - 3..];
                if last3[1] == "seconds" && last3[2] == "ago" {
                    Some(last3[0])
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(secs_str) = seconds_ago_str {
                if let Ok(secs) = secs_str.parse::<i64>() {
                    dev.insert(
                        "updated_seconds_ago".to_string(),
                        Value::Number(secs.into()),
                    );
                }
                // The time string is everything before the "(N seconds ago)" part
                let time_str = parts[..parts.len() - 3].join(" ");
                dev.insert("updated".to_string(), Value::String(time_str.clone()));
                // Compute epoch from time string
                let parsed = parse_timestamp(
                    &time_str,
                    &[
                        formats::F1000,
                        formats::F2000,
                        formats::F3000,
                        formats::F4000,
                        formats::F5000,
                        formats::F8000,
                        formats::F8100,
                    ],
                );
                dev.insert(
                    "updated_epoch".to_string(),
                    parsed
                        .naive_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
                dev.insert(
                    "updated_epoch_utc".to_string(),
                    parsed
                        .utc_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
            }
        }
    }

    // Convert top-level bool fields
    for key in &bool_keys {
        if let Some(Value::String(s)) = dev.get(*key).cloned() {
            dev.insert(key.to_string(), Value::Bool(convert_to_bool(&s)));
        }
    }

    // Process detail sub-object
    if let Some(Value::Object(detail)) = dev.get_mut("detail") {
        let detail_bool_keys = ["online", "present", "rechargeable"];

        // Convert bools
        for key in &detail_bool_keys {
            if let Some(Value::String(s)) = detail.get(*key).cloned() {
                detail.insert(key.to_string(), Value::Bool(convert_to_bool(&s)));
            }
        }

        // Convert "none" warning_level to null
        if let Some(Value::String(s)) = detail.get("warning_level").cloned() {
            if s == "none" {
                detail.insert("warning_level".to_string(), Value::Null);
            }
        }

        // Process energy fields: "22.3998 Wh" -> float + unit key
        let keys_to_process: Vec<String> = detail.keys().cloned().collect();
        let mut additions: Vec<(String, Value)> = Vec::new();

        for key in keys_to_process {
            if let Some(Value::String(val)) = detail.get(&key) {
                let parts: Vec<&str> = val.split_whitespace().collect();
                if parts.len() == 2 {
                    // "value unit"
                    if let Ok(f) = parts[0].parse::<f64>() {
                        additions.push((
                            key.clone(),
                            serde_json::Number::from_f64(f)
                                .map(Value::Number)
                                .unwrap_or(Value::String(val.clone())),
                        ));
                        additions
                            .push((key.clone() + "_unit", Value::String(parts[1].to_string())));
                        continue;
                    }
                }
                // Percentage: "42.5469%"
                if val.ends_with('%') {
                    let num_str = &val[..val.len() - 1];
                    if let Ok(f) = num_str.parse::<f64>() {
                        additions.push((
                            key.clone(),
                            serde_json::Number::from_f64(f)
                                .map(Value::Number)
                                .unwrap_or(Value::String(val.clone())),
                        ));
                    }
                }
                // Quoted values: 'value'
                if val.starts_with('\'') && val.ends_with('\'') {
                    let unquoted = val[1..val.len() - 1].to_string();
                    additions.push((key.clone(), Value::String(unquoted)));
                }
            }
        }

        for (k, v) in additions {
            detail.insert(k, v);
        }
    }

    // Process history lists
    for hist_key in &["history_charge", "history_rate"] {
        if let Some(Value::Array(hist)) = dev.get_mut(*hist_key) {
            for item in hist.iter_mut() {
                if let Value::Object(h) = item {
                    if let Some(Value::String(t)) = h.get("time").cloned() {
                        if let Ok(n) = t.parse::<i64>() {
                            h.insert("time".to_string(), Value::Number(n.into()));
                        }
                    }
                    if let Some(Value::String(p)) = h.get("percent_charged").cloned() {
                        if let Ok(f) = p.parse::<f64>() {
                            h.insert(
                                "percent_charged".to_string(),
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::String(p)),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upower_basic() {
        let input = concat!(
            "Device: /org/freedesktop/UPower/devices/battery_BAT0\n",
            "  native-path: /sys/devices/LNXSYSTM:00\n",
            "  power supply: yes\n",
            "  updated: Thu 11 Mar 2021 06:28:08 PM UTC (441975 seconds ago)\n",
            "  has history: yes\n",
            "  has statistics: yes\n",
            "  battery\n",
            "    present:             yes\n",
            "    rechargeable:        yes\n",
            "    state:               charging\n",
            "    energy:              22.3998 Wh\n",
            "    percentage:          42.5469%\n",
            "    technology:          lithium-ion",
        );

        let parser = UpowerParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            let dev = &arr[0];
            assert_eq!(dev.get("type"), Some(&Value::String("Device".to_string())));
            assert_eq!(dev.get("power_supply"), Some(&Value::Bool(true)));
            assert_eq!(dev.get("has_history"), Some(&Value::Bool(true)));

            if let Some(Value::Object(detail)) = dev.get("detail") {
                assert_eq!(detail.get("present"), Some(&Value::Bool(true)));
                assert_eq!(detail.get("rechargeable"), Some(&Value::Bool(true)));
                assert_eq!(
                    detail.get("state"),
                    Some(&Value::String("charging".to_string()))
                );
                // energy should be parsed as float with unit
                assert!(detail.get("energy").is_some());
                assert_eq!(
                    detail.get("energy_unit"),
                    Some(&Value::String("Wh".to_string()))
                );
            } else {
                panic!("Expected detail object");
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_upower_daemon() {
        let input = concat!(
            "Daemon:\n",
            "  daemon-version:  0.99.11\n",
            "  on-battery:      no\n",
            "  lid-is-closed:   no\n",
            "  lid-is-present:  yes\n",
            "  critical-action: HybridSleep",
        );

        let parser = UpowerParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            let dev = &arr[0];
            assert_eq!(dev.get("type"), Some(&Value::String("Daemon".to_string())));
            assert_eq!(dev.get("on_battery"), Some(&Value::Bool(false)));
            assert_eq!(dev.get("lid_is_present"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_upower_empty() {
        let parser = UpowerParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

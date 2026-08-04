//! Parser for `acpi` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Number, Value};

pub struct AcpiParser;

static INFO: ParserInfo = ParserInfo {
    name: "acpi",
    argument: "--acpi",
    version: "1.7.0",
    description: "Converts `acpi` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["acpi"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ACPI_PARSER: AcpiParser = AcpiParser;

inventory::submit! {
    ParserEntry::new(&ACPI_PARSER)
}

impl Parser for AcpiParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut output_line: Map<String, Value> = Map::new();
        let mut last_line_state = String::new();
        let mut trip_points: Vec<Value> = Vec::new();
        let mut messages: Vec<Value> = Vec::new();

        for line in input.lines().filter(|l| !l.trim().is_empty()) {
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() < 2 {
                continue;
            }

            let obj_type = words[0];
            let obj_id = words[1].trim_end_matches(':');
            let line_state = format!("{}{}", obj_type, obj_id);

            if line_state != last_line_state {
                if !output_line.is_empty() {
                    if !trip_points.is_empty() {
                        output_line
                            .insert("trip_points".to_string(), Value::Array(trip_points.clone()));
                    }
                    if !messages.is_empty() {
                        output_line.insert("messages".to_string(), Value::Array(messages.clone()));
                    }
                    raw_output.push(output_line.clone());
                }
                output_line = Map::new();
                trip_points = Vec::new();
                messages = Vec::new();
            }

            match obj_type {
                "Battery" => {
                    output_line.insert("type".to_string(), Value::String("Battery".to_string()));
                    output_line.insert(
                        "id".to_string(),
                        Value::Number(obj_id.parse::<i64>().unwrap_or(0).into()),
                    );

                    if line.contains("Not charging") {
                        output_line.insert(
                            "state".to_string(),
                            Value::String("Not charging".to_string()),
                        );
                        if let Some(pct) = words.last() {
                            let pct = pct.trim_end_matches(['%', ',']);
                            if let Ok(n) = pct.parse::<i64>() {
                                output_line
                                    .insert("charge_percent".to_string(), Value::Number(n.into()));
                            }
                        }
                    } else if line.contains("Charging")
                        || line.contains("Discharging")
                        || line.contains("Full")
                    {
                        if words.len() > 2 {
                            let state = words[2].trim_end_matches(',');
                            output_line
                                .insert("state".to_string(), Value::String(state.to_string()));
                        }
                        if words.len() > 3 {
                            let pct = words[3].trim_end_matches(['%', ',']);
                            if let Ok(n) = pct.parse::<i64>() {
                                output_line
                                    .insert("charge_percent".to_string(), Value::Number(n.into()));
                            }
                        }
                        if words.len() > 4 {
                            let time = words[4];
                            if !line.contains("will never fully discharge")
                                && !line.contains("rate information unavailable")
                            {
                                if line.contains("Charging") {
                                    output_line.insert(
                                        "until_charged".to_string(),
                                        Value::String(time.to_string()),
                                    );
                                    // Parse hours/minutes/seconds
                                    if let Some(parts) = parse_time(time) {
                                        output_line.insert(
                                            "until_charged_hours".to_string(),
                                            Value::Number(parts.0.into()),
                                        );
                                        output_line.insert(
                                            "until_charged_minutes".to_string(),
                                            Value::Number(parts.1.into()),
                                        );
                                        output_line.insert(
                                            "until_charged_seconds".to_string(),
                                            Value::Number(parts.2.into()),
                                        );
                                        let total = parts.0 * 3600 + parts.1 * 60 + parts.2;
                                        output_line.insert(
                                            "until_charged_total_seconds".to_string(),
                                            Value::Number(total.into()),
                                        );
                                    }
                                } else if line.contains("Discharging") {
                                    output_line.insert(
                                        "charge_remaining".to_string(),
                                        Value::String(time.to_string()),
                                    );
                                    if let Some(parts) = parse_time(time) {
                                        output_line.insert(
                                            "charge_remaining_hours".to_string(),
                                            Value::Number(parts.0.into()),
                                        );
                                        output_line.insert(
                                            "charge_remaining_minutes".to_string(),
                                            Value::Number(parts.1.into()),
                                        );
                                        output_line.insert(
                                            "charge_remaining_seconds".to_string(),
                                            Value::Number(parts.2.into()),
                                        );
                                        let total = parts.0 * 3600 + parts.1 * 60 + parts.2;
                                        output_line.insert(
                                            "charge_remaining_total_seconds".to_string(),
                                            Value::Number(total.into()),
                                        );
                                    }
                                }
                            }
                        }
                    } else if line.contains("design capacity") {
                        // "Battery 0: design capacity 2110 mAh, last full capacity 2271 mAh = 100%"
                        if words.len() > 4
                            && let Ok(n) = words[4].parse::<i64>()
                        {
                            output_line
                                .insert("design_capacity_mah".to_string(), Value::Number(n.into()));
                        }
                        if words.len() > 9
                            && let Ok(n) = words[9].parse::<i64>()
                        {
                            output_line
                                .insert("last_full_capacity".to_string(), Value::Number(n.into()));
                        }
                        if let Some(last) = words.last() {
                            let pct = last.trim_end_matches('%');
                            if let Ok(n) = pct.parse::<i64>() {
                                output_line.insert(
                                    "last_full_capacity_percent".to_string(),
                                    Value::Number(n.into()),
                                );
                            }
                        }
                    }
                }

                "Adapter" => {
                    output_line.insert("type".to_string(), Value::String("Adapter".to_string()));
                    output_line.insert(
                        "id".to_string(),
                        Value::Number(obj_id.parse::<i64>().unwrap_or(0).into()),
                    );
                    output_line
                        .insert("on-line".to_string(), Value::Bool(line.contains("on-line")));
                }

                "Thermal" => {
                    output_line.insert("type".to_string(), Value::String("Thermal".to_string()));
                    output_line.insert(
                        "id".to_string(),
                        Value::Number(obj_id.parse::<i64>().unwrap_or(0).into()),
                    );

                    if line.contains("trip point") {
                        // "Thermal 0: trip point 0 switches to mode critical at temperature 127.0 degrees C"
                        let mut tp = Map::new();
                        if words.len() > 4
                            && let Ok(n) = words[4].parse::<i64>()
                        {
                            tp.insert("id".to_string(), Value::Number(n.into()));
                        }
                        if words.len() > 8 {
                            tp.insert(
                                "switches_to_mode".to_string(),
                                Value::String(words[8].to_string()),
                            );
                        }
                        if words.len() > 11 {
                            let temp_str = words[11];
                            if let Ok(f) = temp_str.parse::<f64>()
                                && let Some(n) = Number::from_f64(f)
                            {
                                tp.insert("temperature".to_string(), Value::Number(n));
                            }
                        }
                        if let Some(last) = words.last() {
                            tp.insert(
                                "temperature_unit".to_string(),
                                Value::String(last.to_string()),
                            );
                        }
                        trip_points.push(Value::Object(tp));
                    } else {
                        // "Thermal 0: ok, 46.0 degrees C"
                        if words.len() > 2 {
                            let mode = words[2].trim_end_matches(',');
                            output_line.insert("mode".to_string(), Value::String(mode.to_string()));
                        }
                        if words.len() > 3 {
                            let temp_str = words[3];
                            if let Ok(f) = temp_str.parse::<f64>()
                                && let Some(n) = Number::from_f64(f)
                            {
                                output_line.insert("temperature".to_string(), Value::Number(n));
                            }
                        }
                        if let Some(last) = words.last() {
                            output_line.insert(
                                "temperature_unit".to_string(),
                                Value::String(last.to_string()),
                            );
                        }
                    }
                }

                "Cooling" => {
                    output_line.insert("type".to_string(), Value::String("Cooling".to_string()));
                    output_line.insert(
                        "id".to_string(),
                        Value::Number(obj_id.parse::<i64>().unwrap_or(0).into()),
                    );
                    // message is everything after "Cooling N: "
                    let prefix = format!("{} {}:", obj_type, obj_id);
                    let msg =
                        line[line.find(&prefix).map(|p| p + prefix.len()).unwrap_or(0)..].trim();
                    messages.push(Value::String(msg.to_string()));
                }

                _ => {}
            }

            last_line_state = line_state;
        }

        // Flush last entry
        if !output_line.is_empty() {
            if !trip_points.is_empty() {
                output_line.insert("trip_points".to_string(), Value::Array(trip_points));
            }
            if !messages.is_empty() {
                output_line.insert("messages".to_string(), Value::Array(messages));
            }
            raw_output.push(output_line);
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

fn parse_time(s: &str) -> Option<(i64, i64, i64)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let h = parts[0].parse::<i64>().ok()?;
        let m = parts[1].parse::<i64>().ok()?;
        let sec = parts[2].parse::<i64>().ok()?;
        Some((h, m, sec))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acpi_battery_charging() {
        let input = "Battery 0: Charging, 71%, 00:29:20 until charged\nBattery 0: design capacity 2110 mAh, last full capacity 2271 mAh = 100%\n";
        let parser = AcpiParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(!arr.is_empty());
            assert_eq!(
                arr[0].get("type"),
                Some(&Value::String("Battery".to_string()))
            );
            assert_eq!(arr[0].get("id"), Some(&Value::Number(0.into())));
            assert_eq!(
                arr[0].get("state"),
                Some(&Value::String("Charging".to_string()))
            );
            assert_eq!(
                arr[0].get("charge_percent"),
                Some(&Value::Number(71.into()))
            );
            assert_eq!(
                arr[0].get("until_charged"),
                Some(&Value::String("00:29:20".to_string()))
            );
            assert_eq!(
                arr[0].get("until_charged_hours"),
                Some(&Value::Number(0.into()))
            );
            assert_eq!(
                arr[0].get("until_charged_minutes"),
                Some(&Value::Number(29.into()))
            );
            assert_eq!(
                arr[0].get("until_charged_total_seconds"),
                Some(&Value::Number(1760.into()))
            );
            assert_eq!(
                arr[0].get("design_capacity_mah"),
                Some(&Value::Number(2110.into()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_acpi_adapter() {
        let input = "Adapter 0: on-line\n";
        let parser = AcpiParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(
                arr[0].get("type"),
                Some(&Value::String("Adapter".to_string()))
            );
            assert_eq!(arr[0].get("on-line"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_acpi_thermal() {
        let input = "Thermal 0: ok, 46.0 degrees C\nThermal 0: trip point 0 switches to mode critical at temperature 127.0 degrees C\n";
        let parser = AcpiParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(
                arr[0].get("type"),
                Some(&Value::String("Thermal".to_string()))
            );
            assert_eq!(arr[0].get("mode"), Some(&Value::String("ok".to_string())));
            assert!(arr[0].get("trip_points").is_some());
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_acpi_cooling() {
        let input = "Cooling 0: Processor 0 of 10\nCooling 1: Processor 0 of 10\n";
        let parser = AcpiParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("type"),
                Some(&Value::String("Cooling".to_string()))
            );
            assert!(arr[0].get("messages").is_some());
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_acpi_empty() {
        let parser = AcpiParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

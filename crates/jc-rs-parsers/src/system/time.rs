//! Parser for `/usr/bin/time` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int};
use serde_json::{Map, Value};

pub struct TimeParser;

static INFO: ParserInfo = ParserInfo {
    name: "time",
    argument: "--time",
    version: "1.5.0",
    description: "Converts `/usr/bin/time` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TIME_PARSER: TimeParser = TimeParser;

inventory::submit! {
    ParserEntry::new(&TIME_PARSER)
}

impl Parser for TimeParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_time(input);
        Ok(ParseOutput::Object(record))
    }
}

pub fn parse_time(input: &str) -> Map<String, Value> {
    let mut raw: Map<String, Value> = Map::new();

    if input.trim().is_empty() {
        return raw;
    }

    // Detect type from first matching line
    let mut time_type: Option<&str> = None;

    for line in input.lines().filter(|l| !l.trim().is_empty()) {
        // Type detection (first matching wins, elif semantics)
        if time_type != Some("linux_brief") && line.contains("elapsed") {
            time_type = Some("linux_brief");
        } else if time_type != Some("bsd_brief") && line.contains(" user ") {
            time_type = Some("bsd_brief");
        } else if time_type != Some("linux_long") && line.contains("Command") {
            time_type = Some("linux_long");
        } else if time_type != Some("bsd_long") && line.contains("maximum resident set size") {
            time_type = Some("bsd_long");
        } else if time_type != Some("posix") && line.starts_with("real ") {
            time_type = Some("posix");
        }

        match time_type {
            Some("linux_brief") => {
                if line.contains("elapsed") {
                    // Line 0: user/system/elapsed/cpu/avgtext/avgdata/maxresident
                    let new_line = line
                        .replace(['+', '(', ')'], " ")
                        .replace("user", " ")
                        .replace("system", " ")
                        .replace("elapsed", " ")
                        .replace("%CPU", " ")
                        .replace("avgtext", " ")
                        .replace("avgdata", " ")
                        .replace("maxresident", " ")
                        .replace('k', " ");

                    let parts: Vec<&str> = new_line.split_whitespace().collect();
                    if parts.len() >= 7 {
                        raw.insert("user_time".to_string(), Value::String(parts[0].to_string()));
                        raw.insert(
                            "system_time".to_string(),
                            Value::String(parts[1].to_string()),
                        );
                        raw.insert(
                            "elapsed_time".to_string(),
                            Value::String(parts[2].to_string()),
                        );
                        raw.insert(
                            "cpu_percent".to_string(),
                            if parts[3] == "?" {
                                Value::Null
                            } else {
                                Value::String(parts[3].to_string())
                            },
                        );
                        raw.insert(
                            "average_shared_text".to_string(),
                            Value::String(parts[4].to_string()),
                        );
                        raw.insert(
                            "average_unshared_data_size".to_string(),
                            Value::String(parts[5].to_string()),
                        );
                        raw.insert(
                            "maximum_resident_set_size".to_string(),
                            Value::String(parts[6].to_string()),
                        );
                    }
                } else {
                    // Line 1: inputs/outputs/major/minor/pagefaults/swaps
                    let new_line = line
                        .replace(['+', '(', ')'], " ")
                        .replace("inputs", " ")
                        .replace("outputs", " ")
                        .replace("major", " ")
                        .replace("minor", " ")
                        .replace("pagefaults", " ")
                        .replace("swaps", " ");

                    let parts: Vec<&str> = new_line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        raw.insert(
                            "block_input_operations".to_string(),
                            Value::String(parts[0].to_string()),
                        );
                        raw.insert(
                            "block_output_operations".to_string(),
                            Value::String(parts[1].to_string()),
                        );
                        raw.insert(
                            "major_pagefaults".to_string(),
                            Value::String(parts[2].to_string()),
                        );
                        raw.insert(
                            "minor_pagefaults".to_string(),
                            Value::String(parts[3].to_string()),
                        );
                        raw.insert("swaps".to_string(), Value::String(parts[4].to_string()));
                    }
                }
            }
            Some("posix") => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if parts[0] == "real" {
                        raw.insert("real_time".to_string(), Value::String(parts[1].to_string()));
                    } else if parts[0] == "user" {
                        raw.insert("user_time".to_string(), Value::String(parts[1].to_string()));
                    } else if parts[0] == "sys" {
                        raw.insert(
                            "system_time".to_string(),
                            Value::String(parts[1].to_string()),
                        );
                    }
                }
            }
            Some("bsd_brief") => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    raw.insert("real_time".to_string(), Value::String(parts[0].to_string()));
                    raw.insert("user_time".to_string(), Value::String(parts[2].to_string()));
                    raw.insert(
                        "system_time".to_string(),
                        Value::String(parts[4].to_string()),
                    );
                }
            }
            Some("bsd_long") => {
                let trimmed = line.trim();
                let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    let value = parts[0].to_string();
                    let key_text = parts[1].trim().replace(' ', "_");

                    // fixup key names
                    let key = match key_text.as_str() {
                        "average_shared_text" => "average_shared_text_size".to_string(),
                        other => other.to_string(),
                    };

                    raw.insert(key, Value::String(value));
                }
            }
            Some("linux_long") => {
                if let Some((key_text, value_text)) = line.split_once(": ") {
                    let mut key = key_text
                        .trim()
                        .to_lowercase()
                        .replace(' ', "_")
                        .replace(['(', ')'], "")
                        .replace(['/', ':'], "_")
                        .replace("_kbytes", "")
                        .replace("_seconds", "")
                        .replace("socket_", "")
                        .replace("_bytes", "");

                    // fixup key names
                    key = match key.as_str() {
                        "file_system_inputs" => "block_input_operations".to_string(),
                        "file_system_outputs" => "block_output_operations".to_string(),
                        "percent_of_cpu_this_job_got" => "cpu_percent".to_string(),
                        "elapsed_wall_clock_time_h_mm_ss_or_m_ss" => "elapsed_time".to_string(),
                        "major_requiring_i_o_page_faults" => "major_pagefaults".to_string(),
                        "minor_reclaiming_a_frame_page_faults" => "minor_pagefaults".to_string(),
                        other => other.to_string(),
                    };

                    let value = value_text.trim().replace('%', "");
                    raw.insert(key, Value::String(value));
                }
            }
            _ => {}
        }
    }

    // Post-process: convert types, add elapsed_time components
    process_time(raw)
}

fn process_time(raw: Map<String, Value>) -> Map<String, Value> {
    let int_list = &[
        "cpu_percent",
        "average_shared_text_size",
        "average_unshared_data_size",
        "average_unshared_stack_size",
        "average_shared_memory_size",
        "maximum_resident_set_size",
        "block_input_operations",
        "block_output_operations",
        "major_pagefaults",
        "minor_pagefaults",
        "swaps",
        "page_reclaims",
        "page_faults",
        "messages_sent",
        "messages_received",
        "signals_received",
        "voluntary_context_switches",
        "involuntary_context_switches",
        "average_stack_size",
        "average_total_size",
        "average_resident_set_size",
        "signals_delivered",
        "page_size",
        "exit_status",
    ];
    let float_list = &["real_time", "user_time", "system_time"];

    let mut out: Map<String, Value> = Map::new();

    // Strip quotes from command_being_timed
    if let Some(Value::String(s)) = raw.get("command_being_timed") {
        let stripped = if (s.starts_with('"') && s.ends_with('"'))
            || (s.starts_with('\'') && s.ends_with('\''))
        {
            s[1..s.len() - 1].to_string()
        } else {
            s.clone()
        };
        out.insert("command_being_timed".to_string(), Value::String(stripped));
    }

    // Compute elapsed_time components
    let elapsed_hours;
    let elapsed_minutes;
    let elapsed_seconds;
    let elapsed_centiseconds;
    let elapsed_total: f64;

    if let Some(Value::String(et)) = raw.get("elapsed_time") {
        let parts: Vec<&str> = et.splitn(3, ':').collect();
        let (h, m, ss) = if parts.len() == 3 {
            (
                parts[0].parse::<i64>().unwrap_or(0),
                parts[1].parse::<i64>().unwrap_or(0),
                parts[2],
            )
        } else if parts.len() == 2 {
            (0i64, parts[0].parse::<i64>().unwrap_or(0), parts[1])
        } else {
            (0i64, 0i64, et.as_str())
        };

        let (s, cs) = if let Some((sec_str, cs_str)) = ss.split_once('.') {
            (
                sec_str.parse::<i64>().unwrap_or(0),
                cs_str.parse::<i64>().unwrap_or(0),
            )
        } else {
            (ss.parse::<i64>().unwrap_or(0), 0i64)
        };

        elapsed_hours = h;
        elapsed_minutes = m;
        elapsed_seconds = s;
        elapsed_centiseconds = cs;
        elapsed_total = (h * 3600 + m * 60 + s) as f64 + cs as f64 / 100.0;
    } else {
        elapsed_hours = 0;
        elapsed_minutes = 0;
        elapsed_seconds = 0;
        elapsed_centiseconds = 0;
        elapsed_total = 0.0;
    }

    for (key, val) in &raw {
        if key == "command_being_timed" {
            continue; // Already handled above
        }
        let v = match val {
            Value::String(s) => {
                if int_list.contains(&key.as_str()) {
                    convert_to_int(s)
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null)
                } else if float_list.contains(&key.as_str()) {
                    convert_to_float(s)
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else {
                    val.clone()
                }
            }
            _ => val.clone(),
        };
        out.insert(key.clone(), v);
    }

    // Add elapsed_time components if elapsed_time exists
    if raw.contains_key("elapsed_time") {
        out.insert(
            "elapsed_time_hours".to_string(),
            Value::Number(elapsed_hours.into()),
        );
        out.insert(
            "elapsed_time_minutes".to_string(),
            Value::Number(elapsed_minutes.into()),
        );
        out.insert(
            "elapsed_time_seconds".to_string(),
            Value::Number(elapsed_seconds.into()),
        );
        out.insert(
            "elapsed_time_centiseconds".to_string(),
            Value::Number(elapsed_centiseconds.into()),
        );
        out.insert(
            "elapsed_time_total_seconds".to_string(),
            serde_json::Number::from_f64(elapsed_total)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/time.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/time.json"
        ))
        .unwrap();
        let parser = TimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_time_verbose_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/time-verbose.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/time-verbose.json"
        ))
        .unwrap();
        let parser = TimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_time_p_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/time-p.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/time-p.json"
        ))
        .unwrap();
        let parser = TimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_time_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/time.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/time.json"
        ))
        .unwrap();
        let parser = TimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_time_l_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/time-l.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/time-l.json"
        ))
        .unwrap();
        let parser = TimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

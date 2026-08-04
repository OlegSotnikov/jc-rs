//! Parser for `iostat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int, simple_table_parse};
use serde_json::{Map, Value};

pub struct IostatParser;

static INFO: ParserInfo = ParserInfo {
    name: "iostat",
    argument: "--iostat",
    version: "1.1.0",
    description: "Converts `iostat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["iostat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IOSTAT_PARSER: IostatParser = IostatParser;

inventory::submit! {
    ParserEntry::new(&IOSTAT_PARSER)
}

impl Parser for IostatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_iostat(input);
        Ok(ParseOutput::Array(rows))
    }
}

pub(crate) fn normalize_iostat_header(line: &str) -> String {
    line.replace('%', "percent_")
        .replace(['/', '-'], "_")
        .to_lowercase()
}

pub(crate) fn create_obj_list(section_lines: &str, section_name: &str) -> Vec<Map<String, Value>> {
    let raw = simple_table_parse(section_lines);
    let float_list = &[
        "percent_user",
        "percent_nice",
        "percent_system",
        "percent_iowait",
        "percent_steal",
        "percent_idle",
        "tps",
        "kb_read_s",
        "mb_read_s",
        "kb_wrtn_s",
        "mb_wrtn_s",
        "rrqm_s",
        "wrqm_s",
        "r_s",
        "w_s",
        "rmb_s",
        "rkb_s",
        "wmb_s",
        "wkb_s",
        "avgrq_sz",
        "avgqu_sz",
        "await",
        "r_await",
        "w_await",
        "svctm",
        "percent_util",
        "percent_rrqm",
        "percent_wrqm",
        "aqu_sz",
        "rareq_sz",
        "wareq_sz",
        "d_s",
        "dkb_s",
        "dmb_s",
        "drqm_s",
        "percent_drqm",
        "d_await",
        "dareq_sz",
        "f_s",
        "f_await",
        "kb_dscd_s",
        "mb_dscd_s",
    ];
    let int_list = &[
        "kb_read", "mb_read", "kb_wrtn", "mb_wrtn", "kb_dscd", "mb_dscd",
    ];

    raw.into_iter()
        .map(|row| {
            let mut out = Map::new();
            for (key, val) in &row {
                let v = match val {
                    Value::String(s) => {
                        if float_list.contains(&key.as_str()) {
                            convert_to_float(s)
                                .map(|f| {
                                    serde_json::Number::from_f64(f)
                                        .map(Value::Number)
                                        .unwrap_or(Value::Null)
                                })
                                .unwrap_or(Value::Null)
                        } else if int_list.contains(&key.as_str()) {
                            convert_to_int(s)
                                .map(|n| Value::Number(n.into()))
                                .unwrap_or(Value::Null)
                        } else {
                            val.clone()
                        }
                    }
                    _ => val.clone(),
                };
                out.insert(key.clone(), v);
            }
            out.insert("type".to_string(), Value::String(section_name.to_string()));
            out
        })
        .collect()
}

pub fn parse_iostat(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut section = "";
    let mut cpu_lines: Vec<String> = Vec::new();
    let mut device_lines: Vec<String> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(header_part) = line.strip_prefix("avg-cpu:") {
            // Flush pending sections
            if !cpu_lines.is_empty() {
                let table_str = cpu_lines.join("\n");
                raw_output.extend(create_obj_list(&table_str, "cpu"));
                cpu_lines.clear();
            }
            if !device_lines.is_empty() {
                let table_str = device_lines.join("\n");
                raw_output.extend(create_obj_list(&table_str, "device"));
                device_lines.clear();
            }

            section = "cpu";
            // Normalize header: strip "avg-cpu:" prefix, normalize
            // after "avg-cpu:"
            let normalized = normalize_iostat_header(header_part).trim().to_string();
            cpu_lines.push(normalized);
            continue;
        }

        if line.starts_with("Device") {
            // Flush pending sections
            if !cpu_lines.is_empty() {
                let table_str = cpu_lines.join("\n");
                raw_output.extend(create_obj_list(&table_str, "cpu"));
                cpu_lines.clear();
            }
            if !device_lines.is_empty() {
                let table_str = device_lines.join("\n");
                raw_output.extend(create_obj_list(&table_str, "device"));
                device_lines.clear();
            }

            section = "device";
            let normalized = normalize_iostat_header(line).replace(':', " ");
            device_lines.push(normalized);
            continue;
        }

        // Skip OS header lines (e.g., "Linux 3.10.0-...")
        if !line.starts_with(' ') && section.is_empty() {
            continue;
        }

        if section == "cpu" {
            cpu_lines.push(line.to_string());
        } else if section == "device" {
            device_lines.push(line.to_string());
        }
    }

    // Flush remaining
    if !cpu_lines.is_empty() {
        let table_str = cpu_lines.join("\n");
        raw_output.extend(create_obj_list(&table_str, "cpu"));
    }
    if !device_lines.is_empty() {
        let table_str = device_lines.join("\n");
        raw_output.extend(create_obj_list(&table_str, "device"));
    }

    raw_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iostat_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iostat.json"
        ))
        .unwrap();
        let parser = IostatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_iostat_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/iostat.json"
        ))
        .unwrap();
        let parser = IostatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_iostat_x_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iostat-x.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iostat-x.json"
        ))
        .unwrap();
        let parser = IostatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

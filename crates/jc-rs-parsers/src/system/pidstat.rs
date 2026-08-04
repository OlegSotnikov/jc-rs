//! Parser for `pidstat -H` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int, simple_table_parse};
use serde_json::{Map, Value};

pub struct PidstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "pidstat",
    argument: "--pidstat",
    version: "1.3.0",
    description: "Converts `pidstat -H` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["pidstat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PIDSTAT_PARSER: PidstatParser = PidstatParser;

inventory::submit! {
    ParserEntry::new(&PIDSTAT_PARSER)
}

impl Parser for PidstatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }
        let rows = parse_pidstat(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn normalize_pidstat_header(header: &str) -> String {
    header
        .replace('#', " ")
        .replace('-', "_")
        .replace('/', "_")
        .replace('%', "percent_")
        .to_lowercase()
}

pub fn parse_pidstat(input: &str) -> Vec<Map<String, Value>> {
    let int_list = &[
        "time",
        "uid",
        "pid",
        "cpu",
        "vsz",
        "rss",
        "stksize",
        "stkref",
        "usr_ms",
        "system_ms",
        "guest_ms",
    ];

    let float_list = &[
        "percent_usr",
        "percent_system",
        "percent_guest",
        "percent_cpu",
        "minflt_s",
        "majflt_s",
        "percent_mem",
        "kb_rd_s",
        "kb_wr_s",
        "kb_ccwr_s",
        "cswch_s",
        "nvcswch_s",
        "percent_wait",
    ];

    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut table_lines: Vec<String> = Vec::new();
    let mut header_found = false;

    let data_lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

    for line in &data_lines {
        if line.starts_with('#') {
            header_found = true;
            // Flush pending table
            if table_lines.len() > 1 {
                let table_str = table_lines.join("\n");
                let rows = simple_table_parse(&table_str);
                raw_output.extend(process_pidstat_rows(rows, int_list, float_list));
            }
            table_lines = vec![normalize_pidstat_header(line)];
            continue;
        }

        if header_found {
            table_lines.push(line.to_string());
        }
    }

    // Flush remaining
    if table_lines.len() > 1 {
        let table_str = table_lines.join("\n");
        let rows = simple_table_parse(&table_str);
        raw_output.extend(process_pidstat_rows(rows, int_list, float_list));
    }

    raw_output
}

fn process_pidstat_rows(
    rows: Vec<std::collections::HashMap<String, Value>>,
    int_list: &[&str],
    float_list: &[&str],
) -> Vec<Map<String, Value>> {
    rows.into_iter()
        .map(|row| {
            let mut out = Map::new();
            for (key, val) in row {
                let v = match &val {
                    Value::String(s) => {
                        if int_list.contains(&key.as_str()) {
                            convert_to_int(s)
                                .map(|n| Value::Number(n.into()))
                                .unwrap_or(Value::Null)
                        } else if float_list.contains(&key.as_str()) {
                            convert_to_float(s)
                                .and_then(|f| serde_json::Number::from_f64(f))
                                .map(Value::Number)
                                .unwrap_or(Value::Null)
                        } else {
                            val
                        }
                    }
                    _ => val,
                };
                out.insert(key, v);
            }
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pidstat_hl_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/pidstat-hl.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/pidstat-hl.json"
        ))
        .unwrap();
        let parser = PidstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_pidstat_ht_generic() {
        let input = include_str!("../../../../tests/fixtures/generic/pidstat-ht.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/pidstat-ht.json"
        ))
        .unwrap();
        let parser = PidstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

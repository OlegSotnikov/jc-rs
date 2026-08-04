//! Parser for `mpstat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, simple_table_parse};
use serde_json::{Map, Value};

pub struct MpstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "mpstat",
    argument: "--mpstat",
    version: "1.1.0",
    description: "Converts `mpstat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["mpstat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static MPSTAT_PARSER: MpstatParser = MpstatParser;

inventory::submit! {
    ParserEntry::new(&MPSTAT_PARSER)
}

impl Parser for MpstatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_mpstat(input);
        Ok(ParseOutput::Array(rows))
    }
}

pub fn parse_mpstat(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut header_found = false;
    let mut header_text = String::new();
    let mut header_start: usize = 0;
    let mut stat_type = "cpu";

    let float_list = &[
        "percent_usr",
        "percent_nice",
        "percent_sys",
        "percent_iowait",
        "percent_irq",
        "percent_soft",
        "percent_steal",
        "percent_guest",
        "percent_gnice",
        "percent_idle",
        "intr_s",
        "nmi_s",
        "loc_s",
        "spu_s",
        "pmi_s",
        "iwi_s",
        "rtr_s",
        "res_s",
        "cal_s",
        "tlb_s",
        "trm_s",
        "thr_s",
        "dfr_s",
        "mce_s",
        "mcp_s",
        "err_s",
        "mis_s",
        "pin_s",
        "npi_s",
        "piw_s",
        "hi_s",
        "timer_s",
        "net_tx_s",
        "net_rx_s",
        "block_s",
        "irq_poll_s",
        "block_iopoll_s",
        "tasklet_s",
        "sched_s",
        "hrtimer_s",
        "rcu_s",
    ];

    for line in input.lines().filter(|l| !l.trim().is_empty()) {
        // Check for header line (contains ' CPU ' or ' NODE ')
        if line.contains(" CPU ") || line.contains(" NODE ") {
            header_found = true;

            if line.contains("%usr") {
                stat_type = "cpu";
            } else {
                stat_type = "interrupts";
            }

            // Normalize header
            let normalized = line
                .replace('/', "_")
                .replace('%', "percent_")
                .to_lowercase();

            // Find the position of "cpu " or "node " in the original line
            header_start = if let Some(pos) = line.find("CPU ") {
                pos
            } else if let Some(pos) = line.find("NODE ") {
                pos
            } else {
                0
            };

            header_text = normalized[header_start..].to_string();
            continue;
        }

        if !header_found {
            continue;
        }

        // Data line
        if line.len() < header_start {
            continue;
        }

        // Parse the data portion of the line (from header_start position)
        let data_portion = if header_start <= line.len() {
            &line[header_start..]
        } else {
            continue;
        };

        let table_str = format!("{}\n{}", header_text, data_portion);
        let rows = simple_table_parse(&table_str);
        if rows.is_empty() {
            continue;
        }

        let mut output_line: Map<String, Value> = Map::new();

        // Convert fields
        for (key, val) in &rows[0] {
            let v = match val {
                Value::String(s) => {
                    let is_float_key = float_list.contains(&key.as_str())
                        || (key.len() > 2
                            && key.ends_with("_s")
                            && key
                                .chars()
                                .next()
                                .map(|c| c.is_ascii_digit())
                                .unwrap_or(false));
                    if is_float_key {
                        convert_to_float(s)
                            .and_then(|f| serde_json::Number::from_f64(f))
                            .map(Value::Number)
                            .unwrap_or(Value::Null)
                    } else {
                        val.clone()
                    }
                }
                _ => val.clone(),
            };
            output_line.insert(key.clone(), v);
        }

        output_line.insert("type".to_string(), Value::String(stat_type.to_string()));

        // Extract time from beginning of the line (before header_start)
        let item_time = if header_start > 0 && header_start <= line.len() {
            line[..header_start].trim().to_string()
        } else {
            String::new()
        };

        if item_time.starts_with("Average") || item_time.starts_with("average") {
            output_line.insert("average".to_string(), Value::Bool(true));
        } else if !item_time.is_empty() {
            output_line.insert("time".to_string(), Value::String(item_time));
        }

        raw_output.push(output_line);
    }

    raw_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpstat_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/mpstat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/mpstat.json"
        ))
        .unwrap();
        let parser = MpstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mpstat_a_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/mpstat-A.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/mpstat-A.json"
        ))
        .unwrap();
        let parser = MpstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mpstat_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/mpstat-A.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/mpstat-A.json"
        ))
        .unwrap();
        let parser = MpstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}

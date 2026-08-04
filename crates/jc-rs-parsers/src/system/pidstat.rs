//! Parser for `pidstat -H` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record};
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
        .replace(['-', '/'], "_")
        .replace('%', "percent_")
        .to_lowercase()
}

const INT_LIST: &[&str] = &[
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

const FLOAT_LIST: &[&str] = &[
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

/// `pidstat -h` restates its `#` header before each sample. The session keeps
/// the current header so a row can be converted the moment it lands, instead of
/// buffering a whole sample the way the batch parser used to.
#[derive(Default)]
pub(crate) struct PidstatSession {
    header: Option<String>,
}

impl LineParser for PidstatSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        if line.starts_with('#') {
            self.header = Some(normalize_pidstat_header(line));
            return Ok(None);
        }

        let Some(header) = self.header.as_deref() else {
            return Ok(None);
        };

        let table = format!("{}\n{}", header, line);
        let rows = process_pidstat_rows(simple_table_parse(&table), INT_LIST, FLOAT_LIST);
        Ok(rows.into_iter().next())
    }
}

pub fn parse_pidstat(input: &str) -> Vec<Map<String, Value>> {
    let mut session = PidstatSession::default();
    input
        .lines()
        .filter_map(|line| session.parse_line(line, true).ok().flatten())
        .collect()
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
                                .and_then(serde_json::Number::from_f64)
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

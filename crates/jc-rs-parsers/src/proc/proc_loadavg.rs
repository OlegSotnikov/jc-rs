//! Parser for `/proc/loadavg`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcLoadavgParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_loadavg",
    argument: "--proc-loadavg",
    version: "1.0.0",
    description: "Converts `/proc/loadavg` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/loadavg"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_LOADAVG_PARSER: ProcLoadavgParser = ProcLoadavgParser;

inventory::submit! { ParserEntry::new(&PROC_LOADAVG_PARSER) }

impl Parser for ProcLoadavgParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let line = input.trim();
        if line.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // Format: "0.00 0.01 0.03 2/111 2039"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(ParseError::InvalidInput(
                "Expected 5 fields in /proc/loadavg".to_string(),
            ));
        }

        let mut out = Map::new();

        if let Ok(v) = parts[0].parse::<f64>() {
            out.insert(
                "load_1m".to_string(),
                Value::Number(serde_json::Number::from_f64(v).unwrap()),
            );
        }
        if let Ok(v) = parts[1].parse::<f64>() {
            out.insert(
                "load_5m".to_string(),
                Value::Number(serde_json::Number::from_f64(v).unwrap()),
            );
        }
        if let Ok(v) = parts[2].parse::<f64>() {
            out.insert(
                "load_15m".to_string(),
                Value::Number(serde_json::Number::from_f64(v).unwrap()),
            );
        }

        // parts[3] = "2/111"
        let procs: Vec<&str> = parts[3].split('/').collect();
        if procs.len() == 2 {
            if let Ok(v) = procs[0].parse::<i64>() {
                out.insert("running".to_string(), Value::Number(v.into()));
            }
            if let Ok(v) = procs[1].parse::<i64>() {
                out.insert("available".to_string(), Value::Number(v.into()));
            }
        }

        if let Ok(v) = parts[4].parse::<i64>() {
            out.insert("last_pid".to_string(), Value::Number(v.into()));
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_loadavg() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/loadavg");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/loadavg.json"
        ))
        .unwrap();
        let parser = ProcLoadavgParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

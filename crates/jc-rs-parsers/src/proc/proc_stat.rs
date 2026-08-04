//! Parser for `/proc/stat`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcStatParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_stat",
    argument: "--proc-stat",
    version: "1.0.0",
    description: "Converts `/proc/stat` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/stat"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_STAT_PARSER: ProcStatParser = ProcStatParser;

inventory::submit! { ParserEntry::new(&PROC_STAT_PARSER) }

const CPU_FIELDS: &[&str] = &[
    "user",
    "nice",
    "system",
    "idle",
    "iowait",
    "irq",
    "softirq",
    "steal",
    "guest",
    "guest_nice",
];

impl Parser for ProcStatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut out = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("cpu") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let mut cpu_map = Map::new();
                    for (i, &field) in CPU_FIELDS.iter().enumerate() {
                        if let Some(val_str) = parts.get(i + 1)
                            && let Ok(v) = val_str.parse::<i64>()
                        {
                            cpu_map.insert(field.to_string(), Value::Number(v.into()));
                        }
                    }
                    out.insert(name.to_string(), Value::Object(cpu_map));
                }
            } else if line.starts_with("intr ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let arr: Vec<Value> = parts[1..]
                    .iter()
                    .filter_map(|s| s.parse::<i64>().ok())
                    .map(|v| Value::Number(v.into()))
                    .collect();
                out.insert("interrupts".to_string(), Value::Array(arr));
            } else if line.starts_with("ctxt ") {
                if let Some(v) = line.split_whitespace().nth(1)
                    && let Ok(n) = v.parse::<i64>()
                {
                    out.insert("context_switches".to_string(), Value::Number(n.into()));
                }
            } else if line.starts_with("btime ") {
                if let Some(v) = line.split_whitespace().nth(1)
                    && let Ok(n) = v.parse::<i64>()
                {
                    out.insert("boot_time".to_string(), Value::Number(n.into()));
                }
            } else if line.starts_with("processes ") {
                if let Some(v) = line.split_whitespace().nth(1)
                    && let Ok(n) = v.parse::<i64>()
                {
                    out.insert("processes".to_string(), Value::Number(n.into()));
                }
            } else if line.starts_with("procs_running ") {
                if let Some(v) = line.split_whitespace().nth(1)
                    && let Ok(n) = v.parse::<i64>()
                {
                    out.insert("processes_running".to_string(), Value::Number(n.into()));
                }
            } else if line.starts_with("procs_blocked ") {
                if let Some(v) = line.split_whitespace().nth(1)
                    && let Ok(n) = v.parse::<i64>()
                {
                    out.insert("processes_blocked".to_string(), Value::Number(n.into()));
                }
            } else if line.starts_with("softirq ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let arr: Vec<Value> = parts[1..]
                    .iter()
                    .filter_map(|s| s.parse::<i64>().ok())
                    .map(|v| Value::Number(v.into()))
                    .collect();
                out.insert("softirq".to_string(), Value::Array(arr));
            }
            // Ignore preempt and other unknown lines
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_stat() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/stat");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/stat.json"
        ))
        .unwrap();
        let parser = ProcStatParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_stat2() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/stat2");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/stat2.json"
        ))
        .unwrap();
        let parser = ProcStatParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

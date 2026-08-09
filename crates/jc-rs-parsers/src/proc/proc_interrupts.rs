//! Parser for `/proc/interrupts`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcInterruptsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_interrupts",
    argument: "--proc-interrupts",
    version: "1.0.0",
    description: "Converts `/proc/interrupts` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/interrupts"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_INTERRUPTS_PARSER: ProcInterruptsParser = ProcInterruptsParser;

inventory::submit! { ParserEntry::new(&PROC_INTERRUPTS_PARSER) }

impl Parser for ProcInterruptsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines();

        // First line gives CPU count
        let first_line = match lines.next() {
            Some(l) => l,
            None => return Ok(ParseOutput::Array(vec![])),
        };
        let cpu_num = first_line.split_whitespace().count();

        let mut entries = Vec::new();
        let mut last_type = String::new();

        for line in lines {
            if !line.contains(':') {
                continue;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            // Pop first element and strip trailing ':'
            let irq = tokens.remove(0).trim_end_matches(':').to_string();

            let mut entry = Map::new();
            entry.insert("irq".to_string(), Value::String(irq.clone()));
            entry.insert("cpu_num".to_string(), Value::Number(cpu_num.into()));

            if irq == "ERR" || irq == "MIS" {
                // All remaining tokens are interrupt counts; type carries from previous
                let interrupts: Vec<Value> = tokens
                    .iter()
                    .filter_map(|t| t.parse::<i64>().ok())
                    .map(|v| Value::Number(v.into()))
                    .collect();
                entry.insert("interrupts".to_string(), Value::Array(interrupts));
                entry.insert("type".to_string(), Value::String(last_type.clone()));
                entry.insert("device".to_string(), Value::Null);
            } else if irq.chars().all(|c| c.is_ascii_digit()) {
                // Numeric IRQ: pop cpu_num values as interrupts, next is type, rest is device
                let interrupts: Vec<Value> = tokens
                    .drain(..cpu_num)
                    .filter_map(|t| t.parse::<i64>().ok())
                    .map(|v| Value::Number(v.into()))
                    .collect();
                entry.insert("interrupts".to_string(), Value::Array(interrupts));

                let irq_type = if !tokens.is_empty() {
                    tokens.remove(0).to_string()
                } else {
                    String::new()
                };
                last_type = irq_type.clone();
                entry.insert("type".to_string(), Value::String(irq_type));

                if tokens.is_empty() {
                    entry.insert("device".to_string(), Value::Null);
                } else {
                    let device: Vec<Value> = tokens
                        .iter()
                        .map(|t| Value::String(t.to_string()))
                        .collect();
                    entry.insert("device".to_string(), Value::Array(device));
                }
            } else {
                // Non-numeric IRQ (NMI, LOC, etc.): pop cpu_num values, rest joined as type
                let interrupts: Vec<Value> = tokens
                    .drain(..cpu_num)
                    .filter_map(|t| t.parse::<i64>().ok())
                    .map(|v| Value::Number(v.into()))
                    .collect();
                entry.insert("interrupts".to_string(), Value::Array(interrupts));

                let irq_type = tokens.join(" ");
                last_type = irq_type.clone();
                entry.insert("type".to_string(), Value::String(irq_type));
                entry.insert("device".to_string(), Value::Null);
            }

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_interrupts() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/interrupts");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/interrupts.json"
        ))
        .unwrap();
        let parser = ProcInterruptsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

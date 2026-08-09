//! Parser for `/proc/consoles`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcConsolesParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_consoles",
    argument: "--proc-consoles",
    version: "1.0.0",
    description: "Converts `/proc/consoles` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/consoles"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_CONSOLES_PARSER: ProcConsolesParser = ProcConsolesParser;

inventory::submit! { ParserEntry::new(&PROC_CONSOLES_PARSER) }

fn op_name(c: char) -> Option<&'static str> {
    match c {
        'R' => Some("read"),
        'W' => Some("write"),
        'U' => Some("unblank"),
        _ => None,
    }
}

fn flag_name(c: char) -> Option<&'static str> {
    match c {
        'E' => Some("enabled"),
        'C' => Some("preferred"),
        'B' => Some("primary boot"),
        'p' => Some("printk buffer"),
        'b' => Some("braille device"),
        'a' => Some("safe when CPU offline"),
        _ => None,
    }
}

impl Parser for ProcConsolesParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Format: "tty0                 -WU (EC p  )    4:1"
            // Find the parens to extract flags
            let paren_open = match line.find('(') {
                Some(i) => i,
                None => continue,
            };
            let paren_close = match line.rfind(')') {
                Some(i) => i,
                None => continue,
            };

            let before_paren = line[..paren_open].trim();
            let flags_raw = &line[paren_open + 1..paren_close];
            let after_paren = line[paren_close + 1..].trim();

            // before_paren = "tty0                 -WU"
            let before_parts: Vec<&str> = before_paren.split_whitespace().collect();
            if before_parts.len() < 2 {
                continue;
            }
            let device = before_parts[0];
            let operations = before_parts[1];

            // operations_list: map non-'-' chars
            let operations_list: Vec<Value> = operations
                .chars()
                .filter_map(|c| op_name(c).map(|s| Value::String(s.to_string())))
                .collect();

            // flags_list: map non-space chars
            let flags_list: Vec<Value> = flags_raw
                .chars()
                .filter(|c| !c.is_whitespace())
                .filter_map(|c| flag_name(c).map(|s| Value::String(s.to_string())))
                .collect();

            // after_paren = "4:1"
            let maj_min: Vec<&str> = after_paren.split(':').collect();
            let major = maj_min
                .first()
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or(0);
            let minor = maj_min
                .get(1)
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or(0);

            let mut entry = Map::new();
            entry.insert("device".to_string(), Value::String(device.to_string()));
            entry.insert(
                "operations".to_string(),
                Value::String(operations.to_string()),
            );
            entry.insert("operations_list".to_string(), Value::Array(operations_list));
            entry.insert("flags".to_string(), Value::String(flags_raw.to_string()));
            entry.insert("flags_list".to_string(), Value::Array(flags_list));
            entry.insert("major".to_string(), Value::Number(major.into()));
            entry.insert("minor".to_string(), Value::Number(minor.into()));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_consoles() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/consoles");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/consoles.json"
        ))
        .unwrap();
        let parser = ProcConsolesParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_consoles2() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/consoles2");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/consoles2.json"
        ))
        .unwrap();
        let parser = ProcConsolesParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

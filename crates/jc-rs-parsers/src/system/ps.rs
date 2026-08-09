//! Parser for `ps` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{normalize_key, simple_table_parse};
use serde_json::{Map, Value};

pub struct PsParser;

static INFO: ParserInfo = ParserInfo {
    name: "ps",
    argument: "--ps",
    version: "1.4.0",
    description: "Converts `ps` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["ps"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PS_PARSER: PsParser = PsParser;

inventory::submit! {
    ParserEntry::new(&PS_PARSER)
}

impl Parser for PsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let raw = parse_raw(input)?;
        let processed = process(raw);
        Ok(ParseOutput::Array(processed))
    }
}

fn parse_raw(input: &str) -> Result<Vec<Map<String, Value>>, ParseError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Normalize headers to snake_case
    let rows = simple_table_parse(input);
    let result = rows
        .into_iter()
        .map(|row| {
            let mut normalized = Map::new();
            for (k, v) in row {
                let nk = normalize_key(&k);
                // jc renames %CPU → cpu_percent, %MEM → mem_percent
                let nk = match nk.as_str() {
                    "_cpu" => "cpu_percent".to_string(),
                    "_mem" => "mem_percent".to_string(),
                    _ => nk,
                };
                normalized.insert(nk, v);
            }
            normalized
        })
        .collect();
    Ok(result)
}

fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    raw.into_iter()
        .map(|mut row| {
            // Each field is rewritten in place. Reading it through `get` forced
            // a clone of the string just to release the borrow, and then a
            // second allocation for the key `insert` needed — two per field per
            // row, for a value that was already sitting in the map.
            // Integer fields
            for field in ["pid", "ppid", "c", "vsz", "rss"] {
                if let Some(slot) = row.get_mut(field)
                    && let Value::String(s) = slot
                    && let Ok(n) = s.trim().parse::<i64>()
                {
                    *slot = Value::Number(n.into());
                }
            }
            // Float fields
            for field in ["%cpu", "%mem", "cpu_percent", "mem_percent"] {
                if let Some(slot) = row.get_mut(field)
                    && let Value::String(s) = slot
                    && let Ok(f) = s.trim().parse::<f64>()
                    && let Some(n) = serde_json::Number::from_f64(f)
                {
                    *slot = Value::Number(n);
                }
            }
            // Null TTY fields
            for field in ["tty", "tt"] {
                if let Some(slot) = row.get_mut(field)
                    && let Value::String(s) = slot
                    && (s == "?" || s == "??")
                {
                    *slot = Value::Null;
                }
            }
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ps_smoke() {
        let input = "  PID TTY          TIME CMD\n    1 ?        00:00:11 systemd\n    2 ?        00:00:00 kthreadd";
        let parser = PsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].get("pid"), Some(&Value::Number(1.into())));
            assert_eq!(arr[0].get("tty"), Some(&Value::Null));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ps_ef_format() {
        let input = "UID        PID  PPID  C STIME TTY          TIME CMD\nroot         1     0  0 Nov01 ?        00:00:11 /usr/lib/systemd/systemd";
        let parser = PsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].get("pid"), Some(&Value::Number(1.into())));
            assert_eq!(arr[0].get("ppid"), Some(&Value::Number(0.into())));
        } else {
            panic!("Expected Array");
        }
    }
}

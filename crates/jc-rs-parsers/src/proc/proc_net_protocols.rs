//! Parser for `/proc/net/protocols`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetProtocolsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_protocols",
    argument: "--proc-net-protocols",
    version: "1.0.0",
    description: "Converts `/proc/net/protocols` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/protocols"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetProtocolsParser = ProcNetProtocolsParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

const INT_FIELDS: &[&str] = &["size", "sockets", "memory", "maxhdr"];
const BOOL_FIELDS: &[&str] = &[
    "slab", "cl", "co", "di", "ac", "io", "in", "de", "sh", "ss", "gs", "se", "re", "sp", "bi",
    "br", "ha", "uh", "gp", "em",
];

fn convert_to_bool(val: &str) -> Value {
    match val {
        "yes" | "y" => Value::Bool(true),
        "no" | "n" => Value::Bool(false),
        _ => Value::Bool(false),
    }
}

impl Parser for ProcNetProtocolsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines().filter(|l| !l.trim().is_empty());

        // First line is the header
        let header_line = match lines.next() {
            Some(h) => h,
            None => return Ok(ParseOutput::Array(vec![])),
        };

        let headers: Vec<&str> = header_line.split_whitespace().collect();
        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in lines {
            let values: Vec<&str> = line.split_whitespace().collect();
            let mut map = Map::new();

            for (i, &key) in headers.iter().enumerate() {
                let raw = values.get(i).copied().unwrap_or("");
                if INT_FIELDS.contains(&key) {
                    let v: i64 = raw.parse().unwrap_or(0);
                    map.insert(key.to_string(), Value::Number(v.into()));
                } else if BOOL_FIELDS.contains(&key) {
                    map.insert(key.to_string(), convert_to_bool(raw));
                } else {
                    map.insert(key.to_string(), Value::String(raw.to_string()));
                }
            }

            results.push(map);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_protocols() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_protocols");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_protocols.json"
        ))
        .unwrap();
        let result = ProcNetProtocolsParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

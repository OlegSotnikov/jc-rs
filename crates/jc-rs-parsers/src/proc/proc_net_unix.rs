//! Parser for `/proc/net/unix`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetUnixParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_unix",
    argument: "--proc-net-unix",
    version: "1.0.0",
    description: "Converts `/proc/net/unix` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/unix"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetUnixParser = ProcNetUnixParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetUnixParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines();

        // Skip header line
        if lines.next().is_none() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Fields: Num RefCount Protocol Flags Type St Inode [Path]
            // Extract 7 whitespace-delimited tokens, treat remainder as optional Path.
            let parts: Vec<&str> = {
                let mut v: Vec<&str> = Vec::with_capacity(8);
                let mut remaining = line;
                for _ in 0..7 {
                    remaining = remaining.trim_start();
                    if remaining.is_empty() {
                        break;
                    }
                    let end = remaining
                        .find(|c: char| c.is_whitespace())
                        .unwrap_or(remaining.len());
                    v.push(&remaining[..end]);
                    remaining = &remaining[end..];
                }
                // The rest (after 7 tokens) is the Path, trimmed
                let path = remaining.trim();
                if !path.is_empty() {
                    v.push(path);
                }
                v
            };

            if parts.len() < 7 {
                continue;
            }

            let mut map = Map::new();
            map.insert("Num".to_string(), Value::String(parts[0].to_string()));
            map.insert("RefCount".to_string(), Value::String(parts[1].to_string()));
            map.insert("Protocol".to_string(), Value::String(parts[2].to_string()));
            map.insert("Flags".to_string(), Value::String(parts[3].to_string()));
            map.insert("Type".to_string(), Value::String(parts[4].to_string()));
            map.insert("St".to_string(), Value::String(parts[5].to_string()));
            map.insert(
                "Inode".to_string(),
                Value::Number(parts[6].parse::<i64>().unwrap_or(0).into()),
            );
            if let Some(&path) = parts.get(7)
                && !path.is_empty()
            {
                map.insert("Path".to_string(), Value::String(path.to_string()));
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
    fn test_proc_net_unix() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_unix");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_unix.json"
        ))
        .unwrap();
        let result = ProcNetUnixParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

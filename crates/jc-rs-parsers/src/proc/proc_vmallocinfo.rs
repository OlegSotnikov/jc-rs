//! Parser for `/proc/vmallocinfo`
//!
//! Each line format: `0xSTART-0xEND size caller [options...] [key=value...]`
//!
//! The Python jc parser has an interesting mutation behaviour: key=value pairs
//! in a line are applied to the *previous* entry (by mutating the dict already
//! in the output list, because Python dicts are reference types). We replicate
//! that exact behaviour here so the output matches the reference fixtures.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcVmallocinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_vmallocinfo",
    argument: "--proc-vmallocinfo",
    version: "1.0.0",
    description: "Converts `/proc/vmallocinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/vmallocinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcVmallocinfoParser = ProcVmallocinfoParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcVmallocinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split into: area, size, details; replicates Python's split(maxsplit=2).
            // We find the byte offsets by scanning for whitespace-separated tokens.
            let area;
            let size_str;
            let details;
            {
                // Skip leading whitespace, find token 1 (area)
                let s = line.trim_start();
                let end1 = s.find(char::is_whitespace).unwrap_or(s.len());
                area = &s[..end1];
                // Skip whitespace, find token 2 (size)
                let s2 = s[end1..].trim_start();
                let end2 = s2.find(char::is_whitespace).unwrap_or(s2.len());
                size_str = &s2[..end2];
                // The rest is details
                details = s2[end2..].trim_start();
            }

            // Split area into start and end on the '-' between the two addresses
            // Both start with "0x", so split after the first '-' that isn't part of hex
            // The format is always "0xADDR-0xADDR"
            let (start, end) = match area.find('-') {
                Some(p) => (&area[..p], &area[p + 1..]),
                None => continue,
            };

            let size: i64 = size_str.parse().unwrap_or(0);

            let mut options: Vec<Value> = Vec::new();
            // key=value pairs to apply to the PREVIOUS entry (Python mutation behaviour)
            let mut kv_for_prev: Vec<(String, String)> = Vec::new();

            let caller;
            if details == "unpurged vm_area" {
                caller = "unpurged vm_area".to_string();
            } else {
                let detail_tokens: Vec<&str> = details.split_whitespace().collect();
                caller = detail_tokens.first().copied().unwrap_or("").to_string();
                for tok in detail_tokens.iter().skip(1) {
                    if let Some(eq_pos) = tok.find('=') {
                        let k = &tok[..eq_pos];
                        let v = &tok[eq_pos + 1..];
                        kv_for_prev.push((k.to_string(), v.to_string()));
                    } else {
                        options.push(Value::String(tok.to_string()));
                    }
                }
            }

            // Apply key=value pairs to the PREVIOUS entry (replicating Python's mutation)
            if let Some(prev) = results.last_mut() {
                for (k, v) in kv_for_prev {
                    // Try to parse as integer; fall back to string
                    if let Ok(n) = v.parse::<i64>() {
                        prev.insert(k, Value::Number(n.into()));
                    } else {
                        prev.insert(k, Value::String(v));
                    }
                }
            }

            let mut entry: Map<String, Value> = Map::new();
            entry.insert("start".to_string(), Value::String(start.to_string()));
            entry.insert("end".to_string(), Value::String(end.to_string()));
            entry.insert("size".to_string(), Value::Number(size.into()));
            if caller.is_empty() {
                entry.insert("caller".to_string(), Value::Null);
            } else {
                entry.insert("caller".to_string(), Value::String(caller));
            }
            entry.insert("options".to_string(), Value::Array(options));

            results.push(entry);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_vmallocinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/vmallocinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/vmallocinfo.json"
        ))
        .unwrap();
        let result = ProcVmallocinfoParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

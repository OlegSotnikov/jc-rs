//! Parser for `/proc/net/netstat`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcNetNetstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_netstat",
    argument: "--proc-net-netstat",
    version: "1.0.0",
    description: "Converts `/proc/net/netstat` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/netstat"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetNetstatParser = ProcNetNetstatParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcNetNetstatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        // The file comes in pairs of lines:
        //   "TcpExt: key1 key2 ..."
        //   "TcpExt: val1 val2 ..."
        // We track which row names have been seen. The first occurrence is
        // the header, the second (same name) is the data row.

        // Collect non-empty lines
        let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

        // Map from row_name -> header keys (in order)
        let mut seen: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for line in &lines {
            let colon_pos = match line.find(':') {
                Some(p) => p,
                None => continue,
            };
            let row_name = line[..colon_pos].trim().to_string();
            let rest = line[colon_pos + 1..].trim();

            // The first line for a name is its header; the second carries the
            // values under it.
            let Some(keys) = seen.get(&row_name) else {
                let keys: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
                seen.insert(row_name, keys);
                continue;
            };
            {
                let values: Vec<&str> = rest.split_whitespace().collect();

                let mut map = Map::new();
                for (i, key) in keys.iter().enumerate() {
                    let v: i64 = values.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
                    map.insert(key.clone(), Value::Number(v.into()));
                }
                map.insert("type".to_string(), Value::String(row_name.clone()));
                results.push(map);
            }
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_netstat() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_netstat");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_netstat.json"
        ))
        .unwrap();
        let result = ProcNetNetstatParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

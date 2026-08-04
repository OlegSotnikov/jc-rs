//! Parser for `/proc/pagetypeinfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPagetypeinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pagetypeinfo",
    argument: "--proc-pagetypeinfo",
    version: "1.0.0",
    description: "Converts `/proc/pagetypeinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/pagetypeinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PAGETYPEINFO_PARSER: ProcPagetypeinfoParser = ProcPagetypeinfoParser;

inventory::submit! { ParserEntry::new(&PROC_PAGETYPEINFO_PARSER) }

impl Parser for ProcPagetypeinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut page_block_order: i64 = 0;
        let mut pages_per_block: i64 = 0;
        let mut free_pages: Vec<Map<String, Value>> = Vec::new();
        let mut num_blocks_type: Vec<Map<String, Value>> = Vec::new();

        #[derive(PartialEq)]
        enum Section {
            Header,
            FreePages,
            NumBlocks,
        }

        let mut section = Section::Header;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("Page block order:") {
                page_block_order = rest.trim().parse::<i64>().unwrap_or(0);
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("Pages per block:") {
                pages_per_block = rest.trim().parse::<i64>().unwrap_or(0);
                continue;
            }

            if trimmed.starts_with("Free pages count") {
                section = Section::FreePages;
                continue;
            }

            if trimmed.starts_with("Number of blocks type") {
                section = Section::NumBlocks;
                continue;
            }

            if section == Section::FreePages {
                // Node    0, zone      DMA, type    Unmovable      0      0 ...
                let cleaned = trimmed.replace(',', " ");
                let parts: Vec<&str> = cleaned.split_whitespace().collect();
                if parts.len() < 7 {
                    continue;
                }
                let node = parts[1].parse::<i64>().unwrap_or(0);
                let zone = parts[3].to_string();
                let typ = parts[5].to_string();
                let free: Vec<Value> = parts[6..]
                    .iter()
                    .map(|s| Value::Number(s.parse::<i64>().unwrap_or(0).into()))
                    .collect();

                let mut entry = Map::new();
                entry.insert("node".to_string(), Value::Number(node.into()));
                entry.insert("zone".to_string(), Value::String(zone));
                entry.insert("type".to_string(), Value::String(typ));
                entry.insert("free".to_string(), Value::Array(free));
                free_pages.push(entry);
            } else if section == Section::NumBlocks {
                // Node 0, zone      DMA            1            7 ...
                let cleaned = trimmed.replace(',', " ");
                let parts: Vec<&str> = cleaned.split_whitespace().collect();
                if parts.len() < 9 {
                    continue;
                }
                let node = parts[1].parse::<i64>().unwrap_or(0);
                let zone = parts[3].to_string();
                let unmovable = parts[4].parse::<i64>().unwrap_or(0);
                let movable = parts[5].parse::<i64>().unwrap_or(0);
                let reclaimable = parts[6].parse::<i64>().unwrap_or(0);
                let high_atomic = parts[7].parse::<i64>().unwrap_or(0);
                let isolate = parts[8].parse::<i64>().unwrap_or(0);

                let mut entry = Map::new();
                entry.insert("node".to_string(), Value::Number(node.into()));
                entry.insert("zone".to_string(), Value::String(zone));
                entry.insert("unmovable".to_string(), Value::Number(unmovable.into()));
                entry.insert("movable".to_string(), Value::Number(movable.into()));
                entry.insert("reclaimable".to_string(), Value::Number(reclaimable.into()));
                entry.insert("high_atomic".to_string(), Value::Number(high_atomic.into()));
                entry.insert("isolate".to_string(), Value::Number(isolate.into()));
                num_blocks_type.push(entry);
            }
        }

        let mut result = Map::new();
        result.insert(
            "page_block_order".to_string(),
            Value::Number(page_block_order.into()),
        );
        result.insert(
            "pages_per_block".to_string(),
            Value::Number(pages_per_block.into()),
        );
        result.insert(
            "free_pages".to_string(),
            Value::Array(free_pages.into_iter().map(Value::Object).collect()),
        );
        result.insert(
            "num_blocks_type".to_string(),
            Value::Array(num_blocks_type.into_iter().map(Value::Object).collect()),
        );

        Ok(ParseOutput::Object(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pagetypeinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pagetypeinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pagetypeinfo.json"
        ))
        .unwrap();
        let parser = ProcPagetypeinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

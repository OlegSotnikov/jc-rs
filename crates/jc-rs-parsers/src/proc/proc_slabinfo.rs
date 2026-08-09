//! Parser for `/proc/slabinfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcSlabinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_slabinfo",
    argument: "--proc-slabinfo",
    version: "1.0.0",
    description: "Converts `/proc/slabinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/slabinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_SLABINFO_PARSER: ProcSlabinfoParser = ProcSlabinfoParser;

inventory::submit! { ParserEntry::new(&PROC_SLABINFO_PARSER) }

impl Parser for ProcSlabinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut entries = Vec::new();

        for line in input.lines().skip(2) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Replace ':' with space, then split by whitespace
            let cleaned = line.replace(':', " ");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            if parts.len() < 14 {
                continue;
            }

            let name = parts[0].to_string();
            let active_objs = parts[1].parse::<i64>().unwrap_or(0);
            let num_objs = parts[2].parse::<i64>().unwrap_or(0);
            let obj_size = parts[3].parse::<i64>().unwrap_or(0);
            let obj_per_slab = parts[4].parse::<i64>().unwrap_or(0);
            let pages_per_slab = parts[5].parse::<i64>().unwrap_or(0);
            // parts[6] = "tunables"
            let limit = parts[7].parse::<i64>().unwrap_or(0);
            let batch_count = parts[8].parse::<i64>().unwrap_or(0);
            let shared_factor = parts[9].parse::<i64>().unwrap_or(0);
            // parts[10] = "slabdata"
            let active_slabs = parts[11].parse::<i64>().unwrap_or(0);
            let num_slabs = parts[12].parse::<i64>().unwrap_or(0);
            let shared_avail = parts[13].parse::<i64>().unwrap_or(0);

            let mut tunables = Map::new();
            tunables.insert("limit".to_string(), Value::Number(limit.into()));
            tunables.insert("batch_count".to_string(), Value::Number(batch_count.into()));
            tunables.insert(
                "shared_factor".to_string(),
                Value::Number(shared_factor.into()),
            );

            let mut slabdata = Map::new();
            slabdata.insert(
                "active_slabs".to_string(),
                Value::Number(active_slabs.into()),
            );
            slabdata.insert("num_slabs".to_string(), Value::Number(num_slabs.into()));
            slabdata.insert(
                "shared_avail".to_string(),
                Value::Number(shared_avail.into()),
            );

            let mut entry = Map::new();
            entry.insert("name".to_string(), Value::String(name));
            entry.insert("active_objs".to_string(), Value::Number(active_objs.into()));
            entry.insert("num_objs".to_string(), Value::Number(num_objs.into()));
            entry.insert("obj_size".to_string(), Value::Number(obj_size.into()));
            entry.insert(
                "obj_per_slab".to_string(),
                Value::Number(obj_per_slab.into()),
            );
            entry.insert(
                "pages_per_slab".to_string(),
                Value::Number(pages_per_slab.into()),
            );
            entry.insert("tunables".to_string(), Value::Object(tunables));
            entry.insert("slabdata".to_string(), Value::Object(slabdata));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_slabinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/slabinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/slabinfo.json"
        ))
        .unwrap();
        let parser = ProcSlabinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

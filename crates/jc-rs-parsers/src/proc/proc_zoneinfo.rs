//! Parser for `/proc/zoneinfo`
//!
//! State-machine parser that produces one object per NUMA node, containing
//! per-zone sub-objects and per-node stat keys.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcZoneinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_zoneinfo",
    argument: "--proc-zoneinfo",
    version: "1.0.0",
    description: "Converts `/proc/zoneinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/zoneinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcZoneinfoParser = ProcZoneinfoParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

#[derive(Debug, PartialEq)]
enum Section {
    Stats,
    Pages,
    Pagesets,
}

impl Parser for ProcZoneinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut current_node: Option<Map<String, Value>> = None;
        let mut current_zone: Option<String> = None;
        let mut section = Section::Stats;
        // The current pageset being built
        let mut current_pageset: Option<Map<String, Value>> = None;

        for line in input.lines() {
            // Skip blank lines
            if line.trim().is_empty() {
                continue;
            }

            // Skip "per-node stats" header
            if line == "  per-node stats" {
                continue;
            }

            // "Node N, zone   ZONENAME"
            if line.starts_with("Node ") {
                // Finish previous pageset if any
                if let Some(ps) = current_pageset.take()
                    && let Some(ref zone_name) = current_zone
                    && let Some(ref mut node) = current_node
                    && let Some(zone_val) = node.get_mut(zone_name)
                    && let Value::Object(zm) = zone_val
                    && let Some(ps_arr) = zm.get_mut("pagesets")
                    && let Value::Array(arr) = ps_arr
                {
                    arr.push(Value::Object(ps));
                }

                // "Node 0, zone      DMA" — triggers a new node only when zone is "DMA"
                // Other zone lines reuse the current node
                let line_no_comma = line.replace(',', "");
                let parts: Vec<&str> = line_no_comma.split_whitespace().collect();
                // parts: ["Node", "0", "zone", "DMA"]
                if parts.len() < 4 {
                    continue;
                }
                let node_id: i64 = parts[1].parse().unwrap_or(0);
                let zone_name = parts[3].to_string();

                let is_dma = zone_name == "DMA";

                if is_dma {
                    // Save previous node
                    if let Some(node) = current_node.take() {
                        results.push(node);
                    }
                    let mut new_node: Map<String, Value> = Map::new();
                    new_node.insert("node".to_string(), Value::Number(node_id.into()));
                    new_node.insert(zone_name.clone(), Value::Object(Map::new()));
                    current_node = Some(new_node);
                } else {
                    // Same node, new zone
                    if let Some(ref mut node) = current_node {
                        node.insert(zone_name.clone(), Value::Object(Map::new()));
                    }
                }

                current_zone = Some(zone_name);
                section = Section::Stats;
                continue;
            }

            // "  pages free N"
            if line.starts_with("  pages free ") {
                section = Section::Pages;
                let free_val: i64 = line
                    .split_whitespace()
                    .last()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
                if let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
                    && let Some(zone_val) = node.get_mut(zone_name)
                    && let Value::Object(zm) = zone_val
                {
                    let mut pages_map: Map<String, Value> = Map::new();
                    pages_map.insert("free".to_string(), Value::Number(free_val.into()));
                    zm.insert("pages".to_string(), Value::Object(pages_map));
                }
                continue;
            }

            // "  pagesets"
            if line.starts_with("  pagesets") {
                section = Section::Pagesets;
                // Initialise pagesets array in the current zone
                if let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
                    && let Some(zone_val) = node.get_mut(zone_name)
                    && let Value::Object(zm) = zone_val
                {
                    zm.insert("pagesets".to_string(), Value::Array(vec![]));
                }
                current_pageset = None;
                continue;
            }

            // Stats section (per-node stats)
            if section == Section::Stats {
                let trimmed = line.trim();
                let mut parts = trimmed.splitn(2, char::is_whitespace);
                let key = match parts.next() {
                    Some(s) => s.trim(),
                    None => continue,
                };
                let val_str = match parts.next() {
                    Some(s) => s.trim(),
                    None => continue,
                };
                if let Ok(n) = val_str.parse::<i64>()
                    && let Some(ref mut node) = current_node
                {
                    node.insert(key.to_string(), Value::Number(n.into()));
                }
                continue;
            }

            // Pages section
            if section == Section::Pages {
                let trimmed = line.trim();

                // "        protection: (0, 2871, 3795, 3795, 3795)"
                if let Some(rest) = trimmed.strip_prefix("protection:") {
                    let cleaned = rest.replace(['(', ')', ','], "");
                    let prot: Vec<Value> = cleaned
                        .split_whitespace()
                        .filter_map(|x| x.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .collect();
                    if let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
                        && let Some(zone_val) = node.get_mut(zone_name)
                        && let Value::Object(zm) = zone_val
                        && let Some(pages_val) = zm.get_mut("pages")
                        && let Value::Object(pm) = pages_val
                    {
                        pm.insert("protection".to_string(), Value::Array(prot));
                    }
                    continue;
                }

                // Regular "key val" pages entry
                let mut parts = trimmed.splitn(2, char::is_whitespace);
                let key = match parts.next() {
                    Some(s) => s.trim(),
                    None => continue,
                };
                let val_str = match parts.next() {
                    Some(s) => s.trim(),
                    None => continue,
                };
                if let Ok(n) = val_str.parse::<i64>()
                    && let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
                    && let Some(zone_val) = node.get_mut(zone_name)
                    && let Value::Object(zm) = zone_val
                    && let Some(pages_val) = zm.get_mut("pages")
                    && let Value::Object(pm) = pages_val
                {
                    pm.insert(key.to_string(), Value::Number(n.into()));
                }
                continue;
            }

            // Pagesets section
            if section == Section::Pagesets {
                let trimmed = line.trim();

                // "    cpu: N"
                if trimmed.starts_with("cpu:") {
                    // Save previous pageset
                    if let Some(ps) = current_pageset.take()
                        && let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
                        && let Some(zone_val) = node.get_mut(zone_name)
                        && let Value::Object(zm) = zone_val
                        && let Some(ps_arr) = zm.get_mut("pagesets")
                        && let Value::Array(arr) = ps_arr
                    {
                        arr.push(Value::Object(ps));
                    }
                    // "cpu: N" → strip "cpu:" and parse int
                    let cpu_str = trimmed["cpu:".len()..].trim();
                    let cpu: i64 = cpu_str.parse().unwrap_or(0);
                    let mut ps: Map<String, Value> = Map::new();
                    ps.insert("cpu".to_string(), Value::Number(cpu.into()));
                    current_pageset = Some(ps);
                    continue;
                }

                // "key: val" pageset field
                if let Some(colon) = trimmed.find(':') {
                    let key = trimmed[..colon].trim().to_string();
                    let val_str = trimmed[colon + 1..].trim();
                    if let Ok(n) = val_str.parse::<i64>()
                        && let Some(ref mut ps) = current_pageset
                    {
                        ps.insert(key, Value::Number(n.into()));
                    }
                }
                continue;
            }
        }

        // Flush last pageset and node
        if let Some(ps) = current_pageset.take()
            && let (Some(zone_name), Some(node)) = (&current_zone, &mut current_node)
            && let Some(zone_val) = node.get_mut(zone_name)
            && let Value::Object(zm) = zone_val
            && let Some(ps_arr) = zm.get_mut("pagesets")
            && let Value::Array(arr) = ps_arr
        {
            arr.push(Value::Object(ps));
        }
        if let Some(node) = current_node.take() {
            results.push(node);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_zoneinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/zoneinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/zoneinfo.json"
        ))
        .unwrap();
        let result = ProcZoneinfoParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_zoneinfo2() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/zoneinfo2");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/zoneinfo2.json"
        ))
        .unwrap();
        let result = ProcZoneinfoParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}

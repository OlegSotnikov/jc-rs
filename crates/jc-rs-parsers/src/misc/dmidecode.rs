//! Parser for `dmidecode` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct DmidecodeParser;

static INFO: ParserInfo = ParserInfo {
    name: "dmidecode",
    argument: "--dmidecode",
    version: "1.5.0",
    description: "Converts `dmidecode` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["dmidecode"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DMIDECODE_PARSER: DmidecodeParser = DmidecodeParser;
inventory::submit! { ParserEntry::new(&DMIDECODE_PARSER) }

fn normalize_key(s: &str) -> String {
    s.trim().to_lowercase().replace(' ', "_")
}

impl Parser for DmidecodeParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let lines: Vec<&str> = input.lines().collect();
        let mut raw_output: Vec<Map<String, Value>> = Vec::new();

        // State machine matching jc's Python logic
        let mut item_header = false;
        let mut item_values = false;
        let mut value_list = false;

        // Current item state
        let mut item: Option<Map<String, Value>> = None;
        let mut header: Vec<String> = Vec::new();
        let mut key: Option<String> = None;
        let mut attribute: Option<String> = None;
        let mut values: Vec<String> = Vec::new();
        let mut key_data: Vec<String> = Vec::new();
        let mut item_map_values: Map<String, Value> = Map::new();

        // Helper to flush current item
        fn flush_item(
            item: &mut Option<Map<String, Value>>,
            item_map_values: &mut Map<String, Value>,
            attribute: &Option<String>,
            values: &mut Vec<String>,
            key: &Option<String>,
            key_data: &mut Vec<String>,
            raw_output: &mut Vec<Map<String, Value>>,
        ) {
            if let Some(it) = item {
                if !values.is_empty() {
                    if let Some(attr) = attribute {
                        item_map_values.insert(
                            attr.clone(),
                            Value::Array(values.iter().map(|s| Value::String(s.clone())).collect()),
                        );
                        values.clear();
                    }
                }
                if !key_data.is_empty() {
                    if let Some(k) = key {
                        item_map_values.insert(
                            format!("{}_data", k),
                            Value::Array(
                                key_data.iter().map(|s| Value::String(s.clone())).collect(),
                            ),
                        );
                        key_data.clear();
                    }
                }

                // Finalize values into item
                if item_map_values.is_empty() {
                    it.insert("values".to_string(), Value::Null);
                } else {
                    it.insert(
                        "values".to_string(),
                        Value::Object(std::mem::take(item_map_values)),
                    );
                }

                // Convert type and bytes to integers
                if let Some(Value::String(s)) = it.get("type") {
                    if let Ok(n) = s.parse::<i64>() {
                        it.insert("type".to_string(), Value::Number(n.into()));
                    }
                }
                if let Some(Value::String(s)) = it.get("bytes") {
                    if let Ok(n) = s.parse::<i64>() {
                        it.insert("bytes".to_string(), Value::Number(n.into()));
                    }
                }

                raw_output.push(std::mem::take(it));
                *item = None;
            }
        }

        // Skip initial header comment lines (everything before the first blank line)
        let mut start = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                start = i + 1;
                // Prime the state machine: next non-blank is the Handle line,
                // and item_header=true so the line after Handle becomes description.
                item_header = true;
                break;
            }
        }

        for line in &lines[start..] {
            // Blank line → new item boundary
            if line.is_empty() {
                item_header = true;
                item_values = false;
                value_list = false;

                flush_item(
                    &mut item,
                    &mut item_map_values,
                    &attribute,
                    &mut values,
                    &key,
                    &mut key_data,
                    &mut raw_output,
                );

                item = None;
                key = None;
                attribute = None;
                values = Vec::new();
                key_data = Vec::new();
                item_map_values = Map::new();
                continue;
            }

            // Handle header line: "Handle 0x0000, DMI type 0, 24 bytes"
            if line.starts_with("Handle ") && line.ends_with("bytes") {
                let h: Vec<String> = line
                    .replace(',', " ")
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                header = h;
                if header.len() >= 6 {
                    let mut it = Map::new();
                    it.insert("handle".to_string(), Value::String(header[1].clone()));
                    it.insert("type".to_string(), Value::String(header[4].clone()));
                    it.insert("bytes".to_string(), Value::String(header[5].clone()));
                    item = Some(it);
                }
                continue;
            }

            // Description line (first non-tab line after header)
            if item_header {
                item_header = false;
                item_values = true;
                value_list = false;

                if let Some(ref mut it) = item {
                    it.insert("description".to_string(), Value::String(line.to_string()));
                }
                continue;
            }

            // New item if not in header and line doesn't start with tab
            // (multiple descriptions in same handle)
            if !item_header && !line.starts_with('\t') {
                item_header = false;
                item_values = true;
                value_list = false;

                // Flush current item first
                flush_item(
                    &mut item,
                    &mut item_map_values,
                    &attribute,
                    &mut values,
                    &key,
                    &mut key_data,
                    &mut raw_output,
                );

                key = None;
                attribute = None;
                values = Vec::new();
                key_data = Vec::new();
                item_map_values = Map::new();

                // Create new item using existing header
                let mut it = Map::new();
                if header.len() >= 6 {
                    it.insert("handle".to_string(), Value::String(header[1].clone()));
                    it.insert("type".to_string(), Value::String(header[4].clone()));
                    it.insert("bytes".to_string(), Value::String(header[5].clone()));
                }
                it.insert("description".to_string(), Value::String(line.to_string()));
                item = Some(it);
                continue;
            }

            // Key-value pair: starts with \t but not \t\t, has ":", doesn't end with ":"
            if item_values
                && line.starts_with('\t')
                && !line.starts_with("\t\t")
                && !line.trim().ends_with(':')
                && line.contains(':')
            {
                let parts = line.splitn(2, ':').collect::<Vec<_>>();
                if parts.len() == 2 {
                    item_header = false;
                    item_values = true;
                    value_list = false;

                    // Flush previous list/data
                    if !values.is_empty() {
                        if let Some(ref attr) = attribute {
                            item_map_values.insert(
                                attr.clone(),
                                Value::Array(
                                    values.iter().map(|s| Value::String(s.clone())).collect(),
                                ),
                            );
                            values = Vec::new();
                        }
                    }
                    if !key_data.is_empty() {
                        if let Some(ref k) = key {
                            item_map_values.insert(
                                format!("{}_data", k),
                                Value::Array(
                                    key_data.iter().map(|s| Value::String(s.clone())).collect(),
                                ),
                            );
                            key_data = Vec::new();
                        }
                    }

                    let k = normalize_key(parts[0]);
                    let v = parts[1].trim().to_string();
                    key = Some(k.clone());
                    item_map_values.insert(k, Value::String(v));
                    attribute = None;
                    continue;
                }
            }

            // Multi-line key: starts with \t but not \t\t, ends with ":"
            if item_values
                && line.starts_with('\t')
                && !line.starts_with("\t\t")
                && line.trim().ends_with(':')
            {
                item_header = false;
                item_values = true;
                value_list = true;

                // Flush previous list/data
                if !values.is_empty() {
                    if let Some(ref attr) = attribute {
                        item_map_values.insert(
                            attr.clone(),
                            Value::Array(values.iter().map(|s| Value::String(s.clone())).collect()),
                        );
                        values = Vec::new();
                    }
                }
                if !key_data.is_empty() {
                    if let Some(ref k) = key {
                        item_map_values.insert(
                            format!("{}_data", k),
                            Value::Array(
                                key_data.iter().map(|s| Value::String(s.clone())).collect(),
                            ),
                        );
                        key_data = Vec::new();
                    }
                }

                // Strip trailing colon from attribute name
                let attr_line = line.trim();
                let attr_name = &attr_line[..attr_line.len() - 1];
                attribute = Some(normalize_key(attr_name));
                values = Vec::new();
                continue;
            }

            // Multi-line values (double-tab indented, inside a value_list)
            if value_list && line.starts_with("\t\t") {
                values.push(line.trim().to_string());
                continue;
            }

            // Hybrid data: double-tab indented, not in value_list
            if item_values && !value_list && line.starts_with("\t\t") {
                key_data.push(line.trim().to_string());
                continue;
            }
        }

        // Flush last item
        flush_item(
            &mut item,
            &mut item_map_values,
            &attribute,
            &mut values,
            &key,
            &mut key_data,
            &mut raw_output,
        );

        Ok(ParseOutput::Array(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dmidecode_empty() {
        let result = DmidecodeParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_dmidecode_registered() {
        assert!(jc_rs_core::registry::find_parser("dmidecode").is_some());
    }
}

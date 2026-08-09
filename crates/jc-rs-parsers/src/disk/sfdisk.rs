//! Parser for `sfdisk` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Number, Value};

pub struct SfdiskParser;

static INFO: ParserInfo = ParserInfo {
    name: "sfdisk",
    argument: "--sfdisk",
    version: "1.0.0",
    description: "Converts `sfdisk` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["sfdisk"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SFDISK_PARSER: SfdiskParser = SfdiskParser;

inventory::submit! {
    ParserEntry::new(&SFDISK_PARSER)
}

impl Parser for SfdiskParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_sfdisk(input)?;
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_sfdisk(input: &str) -> Result<Vec<Map<String, Value>>, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // JSON mode: sfdisk --json outputs JSON directly
    if trimmed.starts_with('{') {
        return parse_sfdisk_json(trimmed);
    }

    // Text mode
    let raw = parse_sfdisk_text(input);
    Ok(process(raw))
}

fn parse_sfdisk_json(input: &str) -> Result<Vec<Map<String, Value>>, ParseError> {
    let parsed: Value = serde_json::from_str(input)
        .map_err(|e| ParseError::InvalidInput(format!("Failed to parse sfdisk JSON: {}", e)))?;

    if let Some(pt) = parsed.get("partitiontable")
        && let Some(obj) = pt.as_object()
    {
        return Ok(vec![obj.clone()]);
    }

    if let Some(obj) = parsed.as_object() {
        Ok(vec![obj.clone()])
    } else if let Some(arr) = parsed.as_array() {
        Ok(arr
            .iter()
            .filter_map(|v: &Value| v.as_object().cloned())
            .collect())
    } else {
        Ok(Vec::new())
    }
}

fn parse_sfdisk_text(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut item: Map<String, Value> = Map::new();
    let mut partitions: Vec<String> = Vec::new();
    let mut option = String::new();
    let mut section = String::new();

    for line in input.lines() {
        // deprecated -d option: "# partition table of /dev/sda"
        if line.starts_with("# partition table of") {
            if !item.is_empty() {
                raw_output.push(item.clone());
            }
            item = Map::new();
            partitions.clear();
            option = "d".to_string();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                item.insert("disk".to_string(), Value::String(parts[4].to_string()));
            }
            continue;
        }

        if option == "d" {
            // deprecated -d option parsing
            if line.starts_with("unit: ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    item.insert("units".to_string(), Value::String(parts[1].to_string()));
                }
                section = "partitions".to_string();
                continue;
            }

            if section == "partitions" && !line.is_empty() {
                let mut part = Map::new();
                let fields: Vec<&str> = line.split_whitespace().collect();
                if !fields.is_empty() {
                    part.insert("device".to_string(), Value::String(fields[0].to_string()));
                }
                // Parse: /dev/sda1 : start=  2048, size=  2097152, Id=83, bootable
                let normalized = line.replace([',', '='], " ");
                let norm_fields: Vec<&str> = normalized.split_whitespace().collect();
                // Find start, size, Id values by keyword
                for i in 0..norm_fields.len() {
                    if norm_fields[i] == "start" && i + 1 < norm_fields.len() {
                        part.insert(
                            "start".to_string(),
                            Value::String(norm_fields[i + 1].to_string()),
                        );
                    }
                    if norm_fields[i] == "size" && i + 1 < norm_fields.len() {
                        part.insert(
                            "size".to_string(),
                            Value::String(norm_fields[i + 1].to_string()),
                        );
                    }
                    if norm_fields[i] == "Id" && i + 1 < norm_fields.len() {
                        part.insert(
                            "id".to_string(),
                            Value::String(norm_fields[i + 1].to_string()),
                        );
                    }
                }
                if line.contains("bootable") {
                    part.insert("boot".to_string(), Value::String("*".to_string()));
                } else {
                    part.insert("boot".to_string(), Value::Null);
                }
                partitions.push(String::new()); // placeholder
                if !item.contains_key("partitions") {
                    item.insert("partitions".to_string(), Value::Array(Vec::new()));
                }
                if let Some(Value::Array(arr)) = item.get_mut("partitions") {
                    arr.push(Value::Object(part));
                }
                continue;
            }
        } else {
            // older versions: Disk /dev/sda: 2610 cylinders, 255 heads, 63 sectors/track
            if line.starts_with("Disk ") && line.contains("sectors/track") {
                if !item.is_empty() {
                    // finalize previous partition section
                    if section == "partitions" && !partitions.is_empty() {
                        let parsed = sparse_table_parse(&partitions);
                        item.insert(
                            "partitions".to_string(),
                            Value::Array(parsed.into_iter().map(Value::Object).collect()),
                        );
                        partitions.clear();
                    }
                    raw_output.push(item.clone());
                }
                item = Map::new();
                partitions.clear();
                section.clear();

                let cleaned = line.replace([':', ','], "");
                let fields: Vec<&str> = cleaned.split_whitespace().collect();
                if fields.len() >= 7 {
                    item.insert("disk".to_string(), Value::String(fields[1].to_string()));
                    item.insert(
                        "cylinders".to_string(),
                        Value::String(fields[2].to_string()),
                    );
                    item.insert("heads".to_string(), Value::String(fields[4].to_string()));
                    item.insert(
                        "sectors_per_track".to_string(),
                        Value::String(fields[6].to_string()),
                    );
                }
                continue;
            }

            // newer versions: Disk /dev/sda: 20 GiB, 21474836480 bytes, 41943040 sectors
            if line.starts_with("Disk ")
                && line.ends_with("sectors")
                && !line.contains("sectors/track")
            {
                if !item.is_empty() {
                    if section == "partitions" && !partitions.is_empty() {
                        let parsed = sparse_table_parse(&partitions);
                        item.insert(
                            "partitions".to_string(),
                            Value::Array(parsed.into_iter().map(Value::Object).collect()),
                        );
                        partitions.clear();
                    }
                    raw_output.push(item.clone());
                }
                item = Map::new();
                partitions.clear();
                section.clear();

                let cleaned = line.replace([':', ','], "");
                let fields: Vec<&str> = cleaned.split_whitespace().collect();
                if fields.len() >= 7 {
                    item.insert("disk".to_string(), Value::String(fields[1].to_string()));
                    item.insert(
                        "disk_size".to_string(),
                        Value::String(format!("{} {}", fields[2], fields[3])),
                    );
                    item.insert("bytes".to_string(), Value::String(fields[4].to_string()));
                    item.insert("sectors".to_string(), Value::String(fields[6].to_string()));
                }
                continue;
            }

            if line.starts_with("Disk model: ") {
                if let Some(val) = line.split_once(':').map(|x| x.1) {
                    item.insert(
                        "disk_model".to_string(),
                        Value::String(val.trim().to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("Sector size (logical/physical)") {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 7 {
                    item.insert(
                        "logical_sector_size".to_string(),
                        Value::String(fields[3].to_string()),
                    );
                    item.insert(
                        "physical_sector_size".to_string(),
                        Value::String(fields[6].to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("I/O size (minimum/optimal)") {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 7 {
                    item.insert(
                        "min_io_size".to_string(),
                        Value::String(fields[3].to_string()),
                    );
                    item.insert(
                        "optimal_io_size".to_string(),
                        Value::String(fields[6].to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("Disklabel type") {
                if let Some(val) = line.split_once(':').map(|x| x.1) {
                    item.insert(
                        "disk_label_type".to_string(),
                        Value::String(val.trim().to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("Disk identifier") {
                if let Some(val) = line.split_once(':').map(|x| x.1) {
                    item.insert(
                        "disk_identifier".to_string(),
                        Value::String(val.trim().to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("Units: ") {
                if let Some(val) = line.split_once(':').map(|x| x.1) {
                    item.insert("units".to_string(), Value::String(val.trim().to_string()));
                }
                continue;
            }

            // sfdisk -F: Unpartitioned space /dev/sda: 0 B, 0 bytes, 0 sectors
            if line.starts_with("Unpartitioned space") {
                let cleaned = line.replace([':', ','], "");
                let fields: Vec<&str> = cleaned.split_whitespace().collect();
                // "Unpartitioned space /dev/sda 0 B 0 bytes 0 sectors"
                if fields.len() >= 8 {
                    item.insert("disk".to_string(), Value::String(fields[2].to_string()));
                    item.insert(
                        "free_disk_size".to_string(),
                        Value::String(format!("{} {}", fields[3], fields[4])),
                    );
                    item.insert(
                        "free_bytes".to_string(),
                        Value::String(fields[5].to_string()),
                    );
                    item.insert(
                        "free_sectors".to_string(),
                        Value::String(fields[7].to_string()),
                    );
                }
                continue;
            }

            // Partition table header line
            if line.contains("Start")
                && line.contains("End")
                && (line.contains("Sectors") || line.contains("Device"))
            {
                section = "partitions".to_string();
                partitions.push(line.to_lowercase().replace('#', " "));
                continue;
            }

            if section == "partitions" && !line.is_empty() {
                partitions.push(line.to_string());
                continue;
            }

            if section == "partitions" && line.is_empty() {
                let parsed = sparse_table_parse(&partitions);
                item.insert(
                    "partitions".to_string(),
                    Value::Array(parsed.into_iter().map(Value::Object).collect()),
                );
                section.clear();
                partitions.clear();
                continue;
            }
        }
    }

    // Get final partitions if there are any left over
    if section == "partitions" && option != "d" && !partitions.is_empty() {
        let parsed = sparse_table_parse(&partitions);
        item.insert(
            "partitions".to_string(),
            Value::Array(parsed.into_iter().map(Value::Object).collect()),
        );
    }

    if !item.is_empty() {
        raw_output.push(item);
    }

    raw_output
}

/// Implements jc's sparse_table_parse algorithm.
/// Takes lines where the first line is the header. Column positions from the header
/// are used to extract field values from data lines.
fn sparse_table_parse(lines: &[String]) -> Vec<Map<String, Value>> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Pad all lines to the same length
    let max_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let data: Vec<String> = lines
        .iter()
        .map(|l| format!("{:<width$}", l, width = max_len))
        .collect();

    let header_text = format!("{} ", data[0]);
    let header_list: Vec<&str> = header_text.split_whitespace().collect();

    if header_list.is_empty() {
        return Vec::new();
    }

    // Build header_search patterns: first col is just the name, rest are " name "
    let mut header_search: Vec<String> = vec![header_list[0].to_string()];
    for h in &header_list[1..] {
        header_search.push(format!(" {} ", h));
    }

    // For each column except the last, find the end position (where the next column starts)
    let mut header_spec: Vec<(String, usize)> = Vec::new(); // (name, end_pos)
    for i in 0..header_list.len() - 1 {
        if let Some(pos) = header_text.find(&header_search[i + 1]) {
            header_spec.push((header_list[i].to_string(), pos));
        }
    }

    let delim = '\u{2063}'; // invisible separator

    let mut output = Vec::new();
    for entry_line in &data[1..] {
        let mut entry: Vec<char> = entry_line.chars().collect();

        // Insert delimiters at column boundaries (process in reverse)
        for (_col_name, h_end) in header_spec.iter().rev() {
            let mut pos = *h_end;
            // Adjust left if position lands on non-whitespace
            while pos > 0 && !entry.get(pos).is_none_or(|c| c.is_whitespace()) {
                pos -= 1;
            }
            if pos < entry.len() {
                entry[pos] = delim;
            }
        }

        let entry_str: String = entry.into_iter().collect();
        let parts: Vec<&str> = entry_str.splitn(header_list.len(), delim).collect();

        let mut row = Map::new();
        for (i, col_name) in header_list.iter().enumerate() {
            let val = parts.get(i).map(|s| s.trim()).unwrap_or("");
            if val.is_empty() {
                row.insert(col_name.to_string(), Value::Null);
            } else {
                row.insert(col_name.to_string(), Value::String(val.to_string()));
            }
        }
        output.push(row);
    }

    output
}

/// Post-processing: convert string values to appropriate types (int, bool).
/// Matches jc's _process function.
fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    let int_keys: &[&str] = &[
        "cylinders",
        "heads",
        "sectors_per_track",
        "start",
        "end",
        "cyls",
        "mib",
        "blocks",
        "sectors",
        "bytes",
        "logical_sector_size",
        "physical_sector_size",
        "min_io_size",
        "optimal_io_size",
        "free_bytes",
        "free_sectors",
    ];

    let mut output = raw;
    for entry in &mut output {
        // Convert disk-level int fields
        for &key in int_keys {
            if let Some(val) = entry.get(key).cloned()
                && let Some(s) = val.as_str()
            {
                let cleaned = s.replace('-', "");
                entry.insert(key.to_string(), convert_to_int(&cleaned));
            }
        }

        // Process partitions
        if let Some(Value::Array(parts)) = entry.get_mut("partitions") {
            for part_val in parts.iter_mut() {
                if let Value::Object(part) = part_val {
                    // Special handling for "size" field in -d option
                    if let Some(size_val) = part.get("size").cloned()
                        && let Some(s) = size_val.as_str()
                    {
                        // Only convert to int if it's purely numeric
                        if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
                            part.insert("size".to_string(), convert_to_int(s));
                        }
                    }

                    // Convert int fields
                    for &key in int_keys {
                        if let Some(val) = part.get(key).cloned()
                            && let Some(s) = val.as_str()
                        {
                            let cleaned = s.replace('-', "");
                            part.insert(key.to_string(), convert_to_int(&cleaned));
                        }
                    }

                    // Convert boot field to boolean
                    if let Some(val) = part.get("boot").cloned() {
                        match &val {
                            Value::String(s) if s == "*" => {
                                part.insert("boot".to_string(), Value::Bool(true));
                            }
                            Value::Null => {
                                part.insert("boot".to_string(), Value::Bool(false));
                            }
                            Value::String(_) => {
                                // Any other string (empty, etc.) => false
                                part.insert("boot".to_string(), Value::Bool(false));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    output
}

/// Convert a string to an integer Value, or null if not parseable.
fn convert_to_int(s: &str) -> Value {
    let trimmed = s.trim().trim_end_matches('+').trim_end_matches('-');
    if trimmed.is_empty() {
        return Value::Null;
    }
    match trimmed.parse::<i64>() {
        Ok(n) => Value::Number(Number::from(n)),
        Err(_) => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfdisk_json_mode() {
        let input = r#"{"partitiontable":{"device":"/dev/sda","unit":"sectors"}}"#;

        let parser = SfdiskParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0]["device"], Value::String("/dev/sda".into()));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_sfdisk_text_mode() {
        let input = "Disk /dev/sda: 50 GiB, 53687091200 bytes, 104857600 sectors\n\
                      Disk identifier: 0x0001234\n\
                      \n\
                      Device     Boot   Start      End  Sectors Size Id Type\n\
                      /dev/sda1  *       2048  2099199  2097152   1G 83 Linux\n\
                      /dev/sda2       2099200 104857599 102758400  49G 83 Linux\n";

        let parser = SfdiskParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0]["disk"], Value::String("/dev/sda".into()));
            assert!(arr[0]["partitions"].is_array());
        } else {
            panic!("expected array");
        }
    }
}

//! Parser for `lsusb` command output.
//!
//! Supports both simple `lsusb` and `lsusb -v` verbose output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsusbParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsusb",
    argument: "--lsusb",
    version: "1.0.0",
    description: "Converts `lsusb` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["lsusb"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSUSB_PARSER: LsusbParser = LsusbParser;

inventory::submit! {
    ParserEntry::new(&LSUSB_PARSER)
}

impl Parser for LsusbParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Detect if this is verbose output
        let is_verbose = input
            .lines()
            .any(|l| l.trim_start().starts_with("Device Descriptor:"));

        if is_verbose {
            parse_verbose(input)
        } else {
            parse_simple(input)
        }
    }
}

/// Parse simple `lsusb` output (one device per line)
/// Format: Bus NNN Device NNN: ID xxxx:xxxx description
fn parse_simple(input: &str) -> Result<ParseOutput, ParseError> {
    let mut result = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // "Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub"
        let mut entry = Map::new();

        // Extract Bus number
        if let Some(bus_rest) = line.strip_prefix("Bus ") {
            let space = bus_rest.find(' ').unwrap_or(bus_rest.len());
            let bus = &bus_rest[..space];
            entry.insert("bus".to_string(), Value::String(bus.to_string()));

            let rest = bus_rest[space..].trim_start();
            // Device NNN: ID ...
            if let Some(dev_rest) = rest.strip_prefix("Device ") {
                let colon = dev_rest.find(':').unwrap_or(dev_rest.len());
                let device = &dev_rest[..colon];
                entry.insert("device".to_string(), Value::String(device.to_string()));

                let rest2 = dev_rest[colon + 1..].trim_start();
                // ID xxxx:xxxx description
                if let Some(id_rest) = rest2.strip_prefix("ID ") {
                    let space2 = id_rest.find(' ').unwrap_or(id_rest.len());
                    let id = &id_rest[..space2];
                    entry.insert("id".to_string(), Value::String(id.to_string()));

                    let description = id_rest[space2..].trim();
                    entry.insert(
                        "description".to_string(),
                        Value::String(description.to_string()),
                    );
                }
            }
        }

        if !entry.is_empty() {
            result.push(entry);
        }
    }

    Ok(ParseOutput::Array(result))
}

/// Count leading spaces for indentation level
fn count_indent(line: &str) -> usize {
    line.chars().take_while(|&c| c == ' ').count()
}

/// Parse a key/value/description line using jc's sparse_table_parse algorithm.
///
/// The algorithm mirrors Python's `sparse_table_parse`:
/// - Column boundaries are found by scanning left from the column position
///   until whitespace is found (same as Python's "slide left" approach).
/// - This correctly handles cases like `bcdDevice            1.00` where the
///   numeric part of the value (`1`) falls before the column boundary.
///
/// Normal sections: key(col 0-21), val(col 22-25), description(col 26+)
/// Wide sections (videocontrol/videostreaming/cdc_mbim_extended): val at 34, desc at 38
fn parse_kv_line_sparse(
    line: &str,
    wide: bool,
) -> Option<(String, Option<String>, Option<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Column positions matching jc headers (using col_end positions, same as Python):
    // Normal: 'key                   val description'  → col_end(key)=21, col_end(val)=25
    // Wide:   'key                               val description' → col_end(key)=33, col_end(val)=37
    let (val_col, desc_col) = if wide { (33, 37) } else { (21, 25) };

    // Work with the line content (stripped of leading indent)
    let indent = count_indent(line);
    let content = &line[indent..];

    if content.is_empty() {
        return None;
    }

    // Pad content to at least desc_col + 25 chars (as jc does with ' ' * 25)
    let min_len = desc_col + 25;
    let padded: String = if content.len() < min_len {
        format!("{}{}", content, " ".repeat(min_len - content.len()))
    } else {
        content.to_string()
    };
    let padded = padded.as_bytes();

    // Find the actual boundary for 'desc' column: start at desc_col, slide left if not whitespace
    let desc_boundary = {
        let mut pos = desc_col;
        while pos > 0 && !padded[pos].is_ascii_whitespace() {
            pos -= 1;
        }
        pos
    };

    // Find the actual boundary for 'val' column: start at val_col, slide left if not whitespace.
    // Python inserts a non-whitespace delimiter (\u2063) at desc_boundary before sliding 'key',
    // so the slide skips past desc_boundary. We replicate this by treating desc_boundary as
    // non-whitespace when sliding val_boundary.
    let val_boundary = {
        let mut pos = val_col;
        loop {
            if pos == 0 {
                break;
            }
            if pos != desc_boundary && padded[pos].is_ascii_whitespace() {
                break;
            }
            pos -= 1;
        }
        pos
    };

    // Extract key: content[..val_boundary], trimmed
    let key = std::str::from_utf8(&padded[..val_boundary])
        .unwrap_or("")
        .trim_end();

    if key.is_empty() {
        return None;
    }
    let key = key.to_string();

    // Extract val: content[val_boundary+1..desc_boundary], trimmed
    let val = if val_boundary < desc_boundary && val_boundary < padded.len() {
        let end = desc_boundary.min(padded.len());
        let start = (val_boundary + 1).min(end);
        let v = std::str::from_utf8(&padded[start..end])
            .unwrap_or("")
            .trim();
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    } else {
        None
    };

    // Extract description: content[desc_boundary+1..], trimmed
    let desc = if desc_boundary + 1 < padded.len() {
        // Find the original content end (not the padding)
        let orig_len = content.len();
        let end = orig_len.min(padded.len());
        let start = (desc_boundary + 1).min(end);
        if start < end {
            let d = std::str::from_utf8(&padded[start..end])
                .unwrap_or("")
                .trim();
            if d.is_empty() {
                None
            } else {
                Some(d.to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    Some((key, val, desc))
}

/// Build a value object {"value": ..., "description": ...}
fn make_value_obj(value: Option<String>, description: Option<String>) -> Map<String, Value> {
    let mut obj = Map::new();
    if let Some(v) = value
        && !v.is_empty()
    {
        obj.insert("value".to_string(), Value::String(v));
    }
    if let Some(d) = description
        && !d.is_empty()
    {
        obj.insert("description".to_string(), Value::String(d));
    }
    obj
}

/// Determine if a line is an attribute (more indented) relative to the last non-attribute line
/// in the same section. This mirrors jc's _add_attributes logic.
fn is_attribute_line(indent: usize, last_indent: usize, same_section: bool) -> bool {
    if !same_section {
        return false;
    }
    indent > last_indent
}

/// Map special section header names to their canonical key names
fn map_section_name(header: &str) -> Option<String> {
    let trimmed = header.trim();
    match trimmed {
        "Device Descriptor:" => Some("device_descriptor".to_string()),
        "Configuration Descriptor:" => Some("configuration_descriptor".to_string()),
        "Interface Association:" => Some("interface_association".to_string()),
        "Interface Descriptor:" => Some("interface_descriptor".to_string()),
        "Endpoint Descriptor:" => Some("endpoint_descriptor".to_string()),
        "Hub Descriptor:" => Some("hub_descriptor".to_string()),
        "Hub Port Status:" => Some("hub_port_status".to_string()),
        "Device Qualifier (for other device speed):" => Some("device_qualifier".to_string()),
        "Binary Object Store Descriptor:" => None, // not implemented, skip
        "Device Status:" => Some("device_status".to_string()),
        "CDC Header:" => Some("cdc_header".to_string()),
        "CDC Call Management:" => Some("cdc_call_management".to_string()),
        "CDC ACM:" => Some("cdc_acm".to_string()),
        "CDC Union:" => Some("cdc_union".to_string()),
        "CDC MBIM:" => Some("cdc_mbim".to_string()),
        "CDC MBIM Extended:" => Some("cdc_mbim_extended".to_string()),
        "HID Device Descriptor:" => Some("hid_device_descriptor".to_string()),
        "VideoControl Interface Descriptor:" => {
            Some("videocontrol_interface_descriptor".to_string())
        }
        "VideoStreaming Interface Descriptor:" => {
            Some("videostreaming_interface_descriptor".to_string())
        }
        _ => {
            // Generic: lowercase + underscore
            let name = trimmed
                .trim_end_matches(':')
                .to_lowercase()
                .replace([' ', '-', '/'], "_");
            Some(name)
        }
    }
}

/// Determine if a section uses wide column format
fn is_wide_section(section: &str) -> bool {
    matches!(
        section,
        "videocontrol_interface_descriptor"
            | "videostreaming_interface_descriptor"
            | "cdc_mbim_extended"
    )
}

/// Parse verbose `lsusb -v` output
fn parse_verbose(input: &str) -> Result<ParseOutput, ParseError> {
    // Fix known too-long field names (same as jc Python)
    let input = input.replace("bmNetworkCapabilities", "bmNetworkCapabilit   ");

    let lines: Vec<&str> = input.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Device header line: "Bus NNN Device NNN: ID xxxx:xxxx description"
        if line.starts_with("Bus ") && line.contains("Device ") {
            let mut device = Map::new();

            // Parse the header
            parse_device_header(line, &mut device);

            i += 1;

            // Skip "Couldn't open device..." lines
            while i < lines.len()
                && (lines[i].starts_with("Couldn't") || lines[i].trim().is_empty())
            {
                i += 1;
            }

            // Parse all sections for this device
            let mut device_descriptor: Map<String, Value> = Map::new();
            let mut hub_descriptor: Map<String, Value> = Map::new();
            let mut device_qualifier: Map<String, Value> = Map::new();
            let mut device_status: Map<String, Value> = Map::new();
            let mut has_hub_descriptor = false;
            let mut has_device_qualifier = false;
            let mut has_device_status = false;

            while i < lines.len() {
                let cur_line = lines[i];

                // Next device header
                if cur_line.starts_with("Bus ") && cur_line.contains("Device ") {
                    break;
                }

                let trimmed = cur_line.trim();

                if trimmed.is_empty() {
                    i += 1;
                    continue;
                }

                if trimmed == "Device Descriptor:" {
                    i += 1;
                    i = parse_device_descriptor(&lines, i, &mut device_descriptor);
                    continue;
                }

                if trimmed == "Hub Descriptor:" {
                    has_hub_descriptor = true;
                    i += 1;
                    i = parse_hub_descriptor(&lines, i, &mut hub_descriptor);
                    continue;
                }

                if trimmed == "Device Qualifier (for other device speed):" {
                    has_device_qualifier = true;
                    i += 1;
                    i = parse_flat_section(&lines, i, 2, &mut device_qualifier, false);
                    continue;
                }

                if trimmed.starts_with("Device Status:") {
                    has_device_status = true;
                    // Value is on the same line: "Device Status:     0x0001"
                    let val_part = trimmed.strip_prefix("Device Status:").unwrap().trim();
                    if !val_part.is_empty() {
                        device_status
                            .insert("value".to_string(), Value::String(val_part.to_string()));
                    }
                    i += 1;
                    // Next line(s) are description
                    while i < lines.len() {
                        let ds_line = lines[i];
                        let ds_trimmed = ds_line.trim();
                        if ds_trimmed.is_empty()
                            || (!ds_line.starts_with(' ') && !ds_trimmed.is_empty())
                        {
                            break;
                        }
                        device_status.insert(
                            "description".to_string(),
                            Value::String(ds_trimmed.to_string()),
                        );
                        i += 1;
                    }
                    continue;
                }

                if trimmed == "Binary Object Store Descriptor:" {
                    // Not implemented in jc, skip entire section
                    i += 1;
                    while i < lines.len() {
                        let skip_line = lines[i];
                        if skip_line.trim().is_empty() {
                            i += 1;
                            continue;
                        }
                        let skip_indent = count_indent(skip_line);
                        // Stop when we hit a line at indent 0 that's a new section
                        if skip_indent == 0 && !skip_line.trim().is_empty() {
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }

                i += 1;
            }

            if !device_descriptor.is_empty() {
                device.insert(
                    "device_descriptor".to_string(),
                    Value::Object(device_descriptor),
                );
            }

            if has_hub_descriptor {
                device.insert("hub_descriptor".to_string(), Value::Object(hub_descriptor));
            }

            if has_device_qualifier {
                device.insert(
                    "device_qualifier".to_string(),
                    Value::Object(device_qualifier),
                );
            }

            if has_device_status {
                device.insert("device_status".to_string(), Value::Object(device_status));
            }

            result.push(device);
        } else {
            i += 1;
        }
    }

    Ok(ParseOutput::Array(result))
}

fn parse_device_header(line: &str, device: &mut Map<String, Value>) {
    // "Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub"
    let parts: Vec<&str> = line.splitn(7, ' ').collect();
    // parts: ["Bus", "001", "Device", "001:", "ID", "1d6b:0002", "description..."]
    if parts.len() >= 2 {
        device.insert("bus".to_string(), Value::String(parts[1].to_string()));
    }
    if parts.len() >= 4 {
        let dev = parts[3].trim_end_matches(':');
        device.insert("device".to_string(), Value::String(dev.to_string()));
    }
    if parts.len() >= 6 {
        device.insert("id".to_string(), Value::String(parts[5].to_string()));
    }
    if parts.len() >= 7 {
        let desc = parts[6..].join(" ");
        device.insert(
            "description".to_string(),
            Value::String(
                if desc.is_empty() {
                    Value::Null
                } else {
                    Value::String(desc)
                }
                .as_str()
                .unwrap_or("")
                .to_string(),
            ),
        );
        // Actually, jc uses (line_split[6:7] or [None])[0], so description can be None
        // But for simplicity, let's just set it as a string (could be empty)
    }

    // Re-parse more carefully using the original approach
    device.clear();
    if let Some(bus_rest) = line.strip_prefix("Bus ") {
        let space = bus_rest.find(' ').unwrap_or(bus_rest.len());
        device.insert(
            "bus".to_string(),
            Value::String(bus_rest[..space].to_string()),
        );
        let rest = bus_rest[space..].trim_start();
        if let Some(dev_rest) = rest.strip_prefix("Device ") {
            let colon = dev_rest.find(':').unwrap_or(dev_rest.len());
            device.insert(
                "device".to_string(),
                Value::String(dev_rest[..colon].to_string()),
            );
            let rest2 = dev_rest[colon + 1..].trim_start();
            if let Some(id_rest) = rest2.strip_prefix("ID ") {
                let space2 = id_rest.find(' ').unwrap_or(id_rest.len());
                device.insert(
                    "id".to_string(),
                    Value::String(id_rest[..space2].to_string()),
                );
                let desc = id_rest[space2..].trim();
                if !desc.is_empty() {
                    device.insert("description".to_string(), Value::String(desc.to_string()));
                }
            }
        }
    }
}

/// Parse device descriptor section (fields at indent 2)
fn parse_device_descriptor(lines: &[&str], start: usize, out: &mut Map<String, Value>) -> usize {
    let mut i = start;
    let mut last_key = String::new();
    let mut last_indent: usize = 2;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Next device header
        if line.starts_with("Bus ") && line.contains("Device ") {
            break;
        }

        // Back to top-level device sections (indent 0)
        if indent == 0 && !trimmed.is_empty() {
            break;
        }

        // Configuration Descriptor section at indent 2
        // Multiple configuration descriptors can appear for a device; jc merges them all
        // into a single configuration_descriptor with a combined interface_descriptors list.
        if indent == 2 && trimmed == "Configuration Descriptor:" {
            let mut config = Map::new();
            i += 1;
            i = parse_configuration_descriptor(lines, i, &mut config);

            // Merge with existing configuration_descriptor if present
            if let Some(Value::Object(existing)) = out.get_mut("configuration_descriptor") {
                // Extract new interface_descriptors
                let new_ifaces = config.remove("interface_descriptors");
                // Extend or set interface_descriptors
                match new_ifaces {
                    Some(Value::Array(new_arr)) => {
                        let existing_ifaces = existing
                            .entry("interface_descriptors".to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing_ifaces {
                            arr.extend(new_arr);
                        }
                    }
                    Some(other) => {
                        existing.insert("interface_descriptors".to_string(), other);
                    }
                    None => {}
                }
                // Update other keys from the new config
                for (k, v) in config {
                    existing.insert(k, v);
                }
            } else {
                out.insert(
                    "configuration_descriptor".to_string(),
                    Value::Object(config),
                );
            }
            last_key.clear();
            continue;
        }

        // Regular key-value at indent 2
        if indent == 2
            && let Some((key, val, desc)) = parse_kv_line_sparse(line, false)
        {
            let obj = make_value_obj(val, desc);
            out.insert(key.clone(), Value::Object(obj));
            last_key = key;
            last_indent = indent;
            i += 1;
            continue;
        }

        // Attribute lines (indent > 2, more indented than the last field)
        if indent > 2 && !last_key.is_empty() && is_attribute_line(indent, last_indent, true) {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    // Format attribute as "key value description" like jc does
                    let attr_str =
                        format_attribute_line(trimmed, is_wide_section("device_descriptor"));
                    arr.push(Value::String(attr_str));
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    i
}

/// Format an attribute line the way jc does: "key value description" stripped
fn format_attribute_line(trimmed: &str, wide: bool) -> String {
    // jc formats attributes as: f'{keyname} {value} {description}'.strip()
    // This collapses multiple spaces into single spaces via key/val/desc join
    if let Some((key, val, desc)) = parse_kv_line_sparse(trimmed, wide) {
        let val_str = val.unwrap_or_default();
        let desc_str = desc.unwrap_or_default();
        format!("{} {} {}", key, val_str, desc_str)
            .trim_end()
            .to_string()
    } else {
        trimmed.to_string()
    }
}

/// Parse a configuration descriptor section (fields at indent 4)
fn parse_configuration_descriptor(
    lines: &[&str],
    start: usize,
    out: &mut Map<String, Value>,
) -> usize {
    let mut i = start;
    let mut last_key = String::new();
    let mut last_indent: usize = 4;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Back to device descriptor level or higher
        if indent <= 2 {
            break;
        }

        // Interface Association at indent 4
        if indent == 4 && trimmed == "Interface Association:" {
            let mut section = Map::new();
            i += 1;
            i = parse_generic_section(lines, i, 6, &mut section, "interface_association");
            out.insert("interface_association".to_string(), Value::Object(section));
            last_key.clear();
            continue;
        }

        // Interface Descriptor at indent 4
        if indent == 4 && trimmed == "Interface Descriptor:" {
            let existing = out
                .entry("interface_descriptors".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(arr) = existing {
                let mut iface = Map::new();
                i += 1;
                i = parse_interface_descriptor(lines, i, &mut iface);
                arr.push(Value::Object(iface));
            }
            last_key.clear();
            continue;
        }

        // Regular key-value at indent 4
        if indent == 4
            && let Some((key, val, desc)) = parse_kv_line_sparse(line, false)
        {
            let obj = make_value_obj(val, desc);
            out.insert(key.clone(), Value::Object(obj));
            last_key = key;
            last_indent = indent;
            i += 1;
            continue;
        }

        // Attribute lines (indent > 4)
        if indent > 4 && !last_key.is_empty() && is_attribute_line(indent, last_indent, true) {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    arr.push(Value::String(trimmed.to_string()));
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    i
}

/// Parse an interface descriptor section (fields at indent 6)
fn parse_interface_descriptor(lines: &[&str], start: usize, out: &mut Map<String, Value>) -> usize {
    let mut i = start;
    let mut last_key = String::new();
    let mut last_indent: usize = 6;
    let mut attribute_value = false;
    let mut current_section = "interface_descriptor".to_string();

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Back to configuration descriptor level or another interface
        if indent <= 4 {
            break;
        }

        // Sub-sections at indent 6
        if indent == 6 && trimmed.ends_with(':') {
            if let Some(section_name) = map_section_name(trimmed) {
                match section_name.as_str() {
                    "endpoint_descriptor" => {
                        let existing = out
                            .entry("endpoint_descriptors".to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing {
                            let mut ep = Map::new();
                            i += 1;
                            i = parse_generic_section(lines, i, 8, &mut ep, "endpoint_descriptor");
                            arr.push(Value::Object(ep));
                        }
                        current_section = "endpoint_descriptor".to_string();
                        last_key.clear();
                        continue;
                    }
                    "videocontrol_interface_descriptor" => {
                        let existing = out
                            .entry("videocontrol_interface_descriptors".to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing {
                            let mut sec = Map::new();
                            i += 1;
                            i = parse_generic_section(
                                lines,
                                i,
                                8,
                                &mut sec,
                                "videocontrol_interface_descriptor",
                            );
                            arr.push(Value::Object(sec));
                        }
                        current_section = "videocontrol_interface_descriptor".to_string();
                        last_key.clear();
                        continue;
                    }
                    "videostreaming_interface_descriptor" => {
                        let existing = out
                            .entry("videostreaming_interface_descriptors".to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing {
                            let mut sec = Map::new();
                            i += 1;
                            i = parse_generic_section(
                                lines,
                                i,
                                8,
                                &mut sec,
                                "videostreaming_interface_descriptor",
                            );
                            arr.push(Value::Object(sec));
                        }
                        current_section = "videostreaming_interface_descriptor".to_string();
                        last_key.clear();
                        continue;
                    }
                    other => {
                        // Only create sub-sections for CDC sections and other known jc-recognized sections.
                        // Unknown sections (e.g. "AudioControl Interface Descriptor:") are NOT in jc's
                        // string_section_map and are treated as regular key-value lines by jc.
                        let known_subsections = [
                            "cdc_header",
                            "cdc_call_management",
                            "cdc_acm",
                            "cdc_union",
                            "cdc_mbim",
                            "cdc_mbim_extended",
                            "hid_device_descriptor",
                            "report_descriptors",
                        ];
                        if known_subsections.contains(&other) {
                            let mut section = Map::new();
                            i += 1;
                            i = parse_generic_section(lines, i, 8, &mut section, other);
                            out.insert(other.to_string(), Value::Object(section));
                            current_section = other.to_string();
                            last_key.clear();
                            continue;
                        }
                        // Unknown section header: treat as regular key-value line (jc behavior)
                        // Fall through to the regular key-value handler below.
                    }
                }
            } else {
                // Section mapped to None (skip)
                i += 1;
                while i < lines.len() {
                    let skip_indent = count_indent(lines[i]);
                    if lines[i].trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    if skip_indent <= 6 {
                        break;
                    }
                    i += 1;
                }
                continue;
            }
        }

        // HID Device Descriptor at indent 8 is a special sub-section (not a regular key-value)
        if indent == 8 && trimmed == "HID Device Descriptor:" && !attribute_value {
            let mut section = Map::new();
            i += 1;
            i = parse_generic_section(lines, i, 10, &mut section, "hid_device_descriptor");
            out.insert("hid_device_descriptor".to_string(), Value::Object(section));
            current_section = "hid_device_descriptor".to_string();
            last_key.clear();
            attribute_value = false;
            last_indent = 8;
            continue;
        }

        // Attribute lines: mirrors jc's logic (indent > last_indent, or same indent still in attr mode)
        if !last_key.is_empty()
            && (indent > last_indent || (indent == last_indent && attribute_value))
        {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let wide = is_wide_section(&current_section);
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    arr.push(Value::String(format_attribute_line(trimmed, wide)));
                }
            }
            attribute_value = true;
            last_indent = indent;
            i += 1;
            continue;
        }
        attribute_value = false;

        // Key-value at any indent >= 6 (when not an attribute - mirrors jc's state machine).
        // This handles lines deeper than indent 6 that become standalone keys, e.g. bAssocTerminal
        // at indent 8 appearing after an attribute block's indent decreases.
        // Lines ending with ':' that are recognized section headers are skipped (they're handled
        // by jc's _set_sections and don't appear in interface_descriptor output).
        if indent >= 6 {
            let is_recognized_section_header = trimmed.ends_with(':') && {
                // jc's string_section_map recognizes these at their specific indents
                let recognized = [
                    "Report Descriptors:",
                    // Add others if needed
                ];
                recognized.contains(&trimmed)
            };

            if is_recognized_section_header {
                // Skip the header and its content (find next line at indent <= 6)
                i += 1;
                while i < lines.len() {
                    if lines[i].trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    let skip_indent = count_indent(lines[i]);
                    if skip_indent <= 6 {
                        break;
                    }
                    i += 1;
                }
                attribute_value = false;
                continue;
            }

            let wide = is_wide_section(&current_section);
            if let Some((key, val, desc)) = parse_kv_line_sparse(line, wide) {
                let obj = make_value_obj(val, desc);
                out.insert(key.clone(), Value::Object(obj));
                last_key = key;
                last_indent = indent;
                if indent == 6 {
                    current_section = "interface_descriptor".to_string();
                }
                i += 1;
                continue;
            }
        }

        i += 1;
    }

    i
}

/// Parse a generic flat section with key-value pairs and attributes
fn parse_generic_section(
    lines: &[&str],
    start: usize,
    base_indent: usize,
    out: &mut Map<String, Value>,
    section_name: &str,
) -> usize {
    let mut i = start;
    let mut last_key = String::new();
    let mut last_indent = base_indent;
    let mut attribute_value = false;
    let wide = is_wide_section(section_name);

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Break only if significantly less indented than base AND it's a section header.
        // Lines with slightly less indentation (anomalous, like "Warning: Descriptor too short")
        // are processed as key-value lines to match jc's state-machine behavior.
        if indent < base_indent {
            if trimmed.ends_with(':') || indent + 2 < base_indent {
                break;
            }
            // Anomalous line at slightly reduced indent - process as key-value.
            // Use the actual indent so subsequent lines at base_indent become attributes.
            if let Some((key, val, desc)) = parse_kv_line_sparse(line, wide) {
                let obj = make_value_obj(val, desc);
                out.insert(key.clone(), Value::Object(obj));
                last_key = key;
                last_indent = indent; // use actual indent so base_indent lines become attributes
                attribute_value = false; // reset; next line will determine
            }
            i += 1;
            continue;
        }

        // Attribute lines: mirrors jc's logic:
        //   - indent > last_indent -> attribute (first attribute after a key)
        //   - indent == last_indent && attribute_value -> attribute (same-level continuation)
        // last_indent is updated on every line (attribute or not), matching jc's _add_attributes.
        if !last_key.is_empty()
            && (indent > last_indent || (indent == last_indent && attribute_value))
        {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    arr.push(Value::String(format_attribute_line(trimmed, wide)));
                }
            }
            attribute_value = true;
            last_indent = indent;
            i += 1;
            continue;
        }
        attribute_value = false;

        // Key-value at base indent or deeper (when not an attribute - mirrors jc's state machine).
        // Note: lines ending with ':' are NOT treated as sub-section breaks here; jc processes them
        // as regular key-values (e.g. 'AudioStreaming Endpoint Descriptor:' becomes key='AudioStreaming').
        // Lines deeper than base_indent can become standalone keys when indent decreases after attributes
        // (e.g. bLockDelayUnits at indent 10 after Sampling Frequency at indent 12).
        if indent >= base_indent
            && let Some((key, val, desc)) = parse_kv_line_sparse(line, wide)
        {
            let obj = make_value_obj(val, desc);
            out.insert(key.clone(), Value::Object(obj));
            last_key = key;
            last_indent = indent;
            i += 1;
            continue;
        }

        i += 1;
    }

    i
}

/// Parse hub descriptor with special hub_port_status handling
fn parse_hub_descriptor(lines: &[&str], start: usize, out: &mut Map<String, Value>) -> usize {
    let mut i = start;
    let mut last_key = String::new();
    let mut last_indent: usize = 2;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Back to top level
        if indent == 0 && !trimmed.is_empty() {
            break;
        }

        // Hub Port Status section (indent 1)
        if trimmed == "Hub Port Status:" {
            let mut hub_port_status = Map::new();
            i += 1;
            i = parse_hub_port_status(lines, i, &mut hub_port_status);
            out.insert(
                "hub_port_status".to_string(),
                Value::Object(hub_port_status),
            );
            last_key.clear();
            continue;
        }

        // Regular key-value at indent 2
        if indent == 2
            && let Some((key, val, desc)) = parse_kv_line_sparse(line, false)
        {
            let obj = make_value_obj(val, desc);
            out.insert(key.clone(), Value::Object(obj));
            last_key = key;
            last_indent = indent;
            i += 1;
            continue;
        }

        // Attribute lines
        if indent > last_indent && !last_key.is_empty() {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    arr.push(Value::String(trimmed.to_string()));
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    i
}

/// Parse hub port status entries
/// Format: "Port 1: 0000.0103 power enable connect"
fn parse_hub_port_status(lines: &[&str], start: usize, out: &mut Map<String, Value>) -> usize {
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        // Hub port status entries are at indent 3-4 typically
        // Stop at indent 0 (new top-level section)
        if indent == 0 && !trimmed.is_empty() {
            break;
        }

        // jc only processes hub port status lines with indent 1-4 (not 5+ spaces).
        // Lines like "     Ext Status:" at indent 5 are excluded from hub_port_status.
        if indent >= 5 {
            i += 1;
            continue;
        }

        // Parse "Port N: XXXX.XXXX attr1 attr2 ..."
        if let Some(colon_pos) = trimmed.find(": ") {
            let port_field = trimmed[..colon_pos].to_string();
            let rest = &trimmed[colon_pos + 2..];
            let parts: Vec<&str> = rest.split_whitespace().collect();

            let mut port_obj = Map::new();
            if !parts.is_empty() {
                port_obj.insert("value".to_string(), Value::String(parts[0].to_string()));
            }
            if parts.len() > 1 {
                let attrs: Vec<Value> = parts[1..]
                    .iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                port_obj.insert("attributes".to_string(), Value::Array(attrs));
            }

            out.insert(port_field, Value::Object(port_obj));
        }

        i += 1;
    }

    i
}

/// Parse a flat section (for device_qualifier, etc.)
fn parse_flat_section(
    lines: &[&str],
    start: usize,
    base_indent: usize,
    out: &mut Map<String, Value>,
    wide: bool,
) -> usize {
    let mut i = start;
    let mut last_key = String::new();

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);

        if indent < base_indent {
            break;
        }

        if indent == base_indent
            && let Some((key, val, desc)) = parse_kv_line_sparse(line, wide)
        {
            let obj = make_value_obj(val, desc);
            out.insert(key.clone(), Value::Object(obj));
            last_key = key;
            i += 1;
            continue;
        }

        // Attribute lines
        if indent > base_indent && !last_key.is_empty() {
            if let Some(Value::Object(obj)) = out.get_mut(&last_key) {
                let attrs = obj
                    .entry("attributes".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = attrs {
                    arr.push(Value::String(trimmed.to_string()));
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsusb_simple() {
        let input = "Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub\nBus 002 Device 004: ID 0e0f:0008 VMware, Inc. \nBus 002 Device 001: ID 1d6b:0001 Linux Foundation 1.1 root hub\n";
        let parser = LsusbParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].get("bus"), Some(&Value::String("001".to_string())));
            assert_eq!(
                arr[0].get("device"),
                Some(&Value::String("001".to_string()))
            );
            assert_eq!(
                arr[0].get("id"),
                Some(&Value::String("1d6b:0002".to_string()))
            );
            assert_eq!(
                arr[0].get("description"),
                Some(&Value::String("Linux Foundation 2.0 root hub".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsusb_verbose_basic() {
        let input = "Bus 003 Device 090: ID 1915:521a Nordic Semiconductor ASA nRF52 USB CDC BLE Demo\nCouldn't open device, some information will be missing\nDevice Descriptor:\n  bLength                18\n  bDescriptorType         1\n  bcdUSB               2.00\n  idVendor           0x1915 Nordic Semiconductor ASA\n  idProduct          0x521a \n";
        let parser = LsusbParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].get("bus"), Some(&Value::String("003".to_string())));
            assert!(arr[0].get("device_descriptor").is_some());
            if let Some(Value::Object(dd)) = arr[0].get("device_descriptor") {
                assert!(dd.get("bLength").is_some());
                if let Some(Value::Object(bl)) = dd.get("bLength") {
                    assert_eq!(bl.get("value"), Some(&Value::String("18".to_string())));
                }
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsusb_empty() {
        let parser = LsusbParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

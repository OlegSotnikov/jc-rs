//! Parser for `mdadm` command output (--examine and --query/--detail modes).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct MdadmParser;

static INFO: ParserInfo = ParserInfo {
    name: "mdadm",
    argument: "--mdadm",
    version: "1.0.0",
    description: "Converts `mdadm` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["mdadm"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static MDADM_PARSER: MdadmParser = MdadmParser;

inventory::submit! {
    ParserEntry::new(&MDADM_PARSER)
}

impl Parser for MdadmParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_mdadm(input);
        Ok(ParseOutput::Object(record))
    }
}

/// Fields that should be converted to integers in the top-level record.
const INT_FIELDS: &[&str] = &[
    "raid_devices",
    "total_devices",
    "active_devices",
    "working_devices",
    "failed_devices",
    "spare_devices",
    "physical_disks",
    "virtual_disks",
    "preferred_minor",
    "array_size_num",
    "used_dev_size_num",
    "avail_dev_size_num",
    "data_offset",
    "super_offset",
    "unused_space_before",
    "unused_space_after",
    "chunk_size_num",
    "check_status_percent",
    "resync_status_percent",
    "rebuild_status_percent",
    "events_num",
    "events_maj",
    "events_min",
    "container_member",
];

/// Normalize a key: lowercase, replace non-alphanumeric with underscore, collapse runs.
fn normalize_key(key: &str) -> String {
    let mut result = String::new();
    let lower = key.trim().to_lowercase();
    let mut last_underscore = false;

    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            result.push(ch);
            last_underscore = false;
        } else if !last_underscore {
            result.push('_');
            last_underscore = true;
        }
    }

    result.trim_matches('_').to_string()
}

/// Try to parse a ctime-style date string into a Unix epoch.
///
/// mdadm prints local time with no zone, and jc's `timestamp().naive` reads it
/// as local. Reading it as UTC instead put every `*_epoch` in this parser out by
/// the machine's offset -- seven hours under the `TZ=PST8PDT` the corpus is
/// pinned to, and a different number of hours everywhere else.
fn parse_date_epoch(s: &str) -> Option<i64> {
    let normalized = s.trim().split_whitespace().collect::<Vec<&str>>().join(" ");
    jc_rs_utils::parse_timestamp(&normalized, Some("%a %b %d %H:%M:%S %Y")).naive_epoch
}

/// Replicate jc's convert_to_int: extract all digit characters, concatenate, parse as i64.
/// Returns None if no digits or empty.
fn convert_to_int(s: &str) -> Option<i64> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<i64>().ok()
    }
}

/// Device table column spec derived from the modified header string.
struct DeviceTableSpec {
    /// Modified header string (as Python uses for sparse_table_parse).
    header: String,
    /// Column names in order.
    col_names: Vec<String>,
    /// End positions for each column except the last (byte offsets into header).
    col_ends: Vec<usize>,
    /// Whether this table has a "state" column.
    has_state: bool,
}

impl DeviceTableSpec {
    fn from_original_header(original: &str) -> Self {
        let has_state = original.contains("State");

        let modified_header = if has_state {
            "    number   major   minor   RaidDevice state         device".to_string()
        } else {
            "    number   major   minor   RaidDevice device".to_string()
        };

        let header_with_space = format!("{} ", modified_header);
        let col_names: Vec<String> = header_with_space
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        // Find end position for each column (except the last) by searching
        // for " next_col " in the header string, matching Python's sparse_table_parse.
        let mut col_ends = Vec::new();
        for i in 0..col_names.len() - 1 {
            let search = format!(" {} ", col_names[i + 1]);
            // Find the search string in the header (case-insensitive match for RaidDevice)
            let pos = find_column_end(&header_with_space, &search);
            col_ends.push(pos);
        }

        DeviceTableSpec {
            header: modified_header,
            col_names,
            col_ends,
            has_state,
        }
    }

    /// Parse one row using column positions with move-left adjustment (like Python's sparse_table_parse).
    fn parse_row(&self, row: &str) -> Vec<(String, Option<String>)> {
        let header_len = self.header.len() + 1;
        // Pad row to at least header length
        let row_padded: String = if row.len() < header_len {
            format!("{:<width$}", row, width = header_len)
        } else {
            row.to_string()
        };
        let row_bytes = row_padded.as_bytes();
        let row_len = row_bytes.len();

        let mut result = Vec::new();
        let mut prev = 0usize;

        for (i, &end) in self.col_ends.iter().enumerate() {
            let col_name = &self.col_names[i];
            let end_clamped = end.min(row_len);
            // Apply move-left: if char at end position is not whitespace, move left
            let adjusted_end = adjust_col_end(row_bytes, end_clamped);
            let chunk = if prev >= row_len {
                ""
            } else {
                let actual = adjusted_end.min(row_len);
                std::str::from_utf8(&row_bytes[prev..actual])
                    .unwrap_or("")
                    .trim()
            };
            let val = if chunk.is_empty() {
                None
            } else {
                Some(chunk.to_string())
            };
            result.push((col_name.clone(), val));
            prev = adjusted_end;
        }

        // Last column
        let last_name = self.col_names.last().cloned().unwrap_or_default();
        let last_chunk = if prev >= row_len {
            ""
        } else {
            std::str::from_utf8(&row_bytes[prev..]).unwrap_or("").trim()
        };
        let last_val = if last_chunk.is_empty() {
            None
        } else {
            Some(last_chunk.to_string())
        };
        result.push((last_name, last_val));
        result
    }
}

/// Adjust column end: if position falls inside a token, move left until whitespace.
/// Mirrors Python's sparse_table_parse adjustment.
fn adjust_col_end(bytes: &[u8], end: usize) -> usize {
    let mut h_end = end;
    while h_end > 0 && h_end < bytes.len() && !bytes[h_end].is_ascii_whitespace() {
        h_end -= 1;
    }
    h_end
}

/// Find position of `search` in `text` (case-sensitive).
fn find_column_end(text: &str, search: &str) -> usize {
    if let Some(pos) = text.find(search) {
        return pos;
    }
    // Try case-insensitive fallback
    let search_lower = search.to_lowercase();
    let text_lower = text.to_lowercase();
    text_lower.find(&search_lower).unwrap_or(text.len())
}

/// Container device table header (examine --container format).
const CONTAINER_TABLE_HEADER: &str = "      Number    RefNo      Size       Device      Type/State";

/// Parse a container-format device table row using the header's column positions.
fn parse_container_table_row(header: &str, row: &str) -> Map<String, Value> {
    // Build column positions from the header (same sparse_table logic)
    let header_with_space = format!("{} ", header);
    let col_names: Vec<&str> = header.split_whitespace().collect();
    let n = col_names.len();

    // Find col_ends using the same approach as sparse_table_parse
    let mut col_ends: Vec<usize> = Vec::new();
    for i in 0..n - 1 {
        let search = format!(" {} ", col_names[i + 1]);
        let pos = if let Some(p) = header_with_space.find(&search) {
            p
        } else {
            header_with_space.len()
        };
        col_ends.push(pos);
    }

    // Pad row
    let max_len = header_with_space.len();
    let row_padded: String = if row.len() < max_len {
        format!("{:<width$}", row, width = max_len)
    } else {
        row.to_string()
    };
    let row_bytes = row_padded.as_bytes();
    let row_len = row_bytes.len();

    let mut map = Map::new();
    let mut prev = 0usize;

    for (i, &end) in col_ends.iter().enumerate() {
        let col_name = col_names[i];
        let end_clamped = end.min(row_len);
        let adjusted_end = adjust_col_end(row_bytes, end_clamped);
        let chunk = if prev >= row_len {
            ""
        } else {
            std::str::from_utf8(&row_bytes[prev..adjusted_end.min(row_len)])
                .unwrap_or("")
                .trim()
        };
        let val = if chunk.is_empty() {
            None
        } else {
            Some(chunk.to_string())
        };
        insert_container_field(&mut map, col_name, val);
        prev = adjusted_end;
    }

    // Last column
    let last_name = col_names.last().copied().unwrap_or("");
    let last_chunk = if prev >= row_len {
        ""
    } else {
        std::str::from_utf8(&row_bytes[prev..]).unwrap_or("").trim()
    };
    let last_val = if last_chunk.is_empty() {
        None
    } else {
        Some(last_chunk.to_string())
    };
    insert_container_field(&mut map, last_name, last_val);

    map
}

fn insert_container_field(map: &mut Map<String, Value>, col_name: &str, val: Option<String>) {
    match col_name {
        "Number" => {
            map.insert(
                "Number".to_string(),
                match val.as_deref().and_then(|s| convert_to_int(s)) {
                    Some(n) => Value::Number(n.into()),
                    None => Value::Null,
                },
            );
        }
        "Device" => {
            map.insert(
                "Device".to_string(),
                match val {
                    Some(s) if !s.is_empty() => Value::String(s),
                    _ => Value::Null,
                },
            );
        }
        _ => {
            map.insert(
                col_name.to_string(),
                match val {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                },
            );
        }
    }
}

fn parse_mdadm(input: &str) -> Map<String, Value> {
    let mut record = Map::new();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return record;
    }

    // Filter blank lines like Python's filter(None, data.splitlines())
    let lines: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();

    let mut in_device_table = false;
    let mut in_container_table = false;
    let mut device_table: Vec<Map<String, Value>> = Vec::new();
    let mut table_spec: Option<DeviceTableSpec> = None;

    for line in &lines {
        // Device name line: starts with '/' and ends with ':'
        if line.starts_with('/') && line.trim_end().ends_with(':') {
            let device = line.trim_end().trim_end_matches(':').to_string();
            record.insert("device".to_string(), Value::String(device));
            continue;
        }

        // Container device table header (examine --container format)
        if line.contains("Number") && line.contains("RefNo") && line.contains("Type/State") {
            in_container_table = true;
            in_device_table = false;
            table_spec = None;
            continue;
        }

        // Standard device table header
        if line.contains("Number") && line.contains("Major") && line.contains("Minor") {
            let spec = DeviceTableSpec::from_original_header(line);
            table_spec = Some(spec);
            in_device_table = true;
            in_container_table = false;
            continue;
        }

        // Container table rows
        if in_container_table {
            if line.contains(" : ") && !line.trim_start().starts_with(|c: char| c.is_ascii_digit())
            {
                in_container_table = false;
                // Fall through to key:value parsing
            } else {
                let dev_record = parse_container_table_row(CONTAINER_TABLE_HEADER, line);
                device_table.push(dev_record);
                continue;
            }
        }

        // Standard device table rows
        if in_device_table {
            if line.contains(" : ")
                && !line
                    .trim_start()
                    .starts_with(|c: char| c.is_ascii_digit() || c == '-')
            {
                in_device_table = false;
                // Fall through to key:value parsing below
            } else if let Some(spec) = &table_spec {
                let cols = spec.parse_row(line);
                let dev_record = process_device_table_row(&cols, spec.has_state);
                device_table.push(dev_record);
                continue;
            }
        }

        // Key : Value lines
        if let Some(sep_pos) = line.find(" : ") {
            let raw_key = line[..sep_pos].trim();
            let val = line[sep_pos + 3..].trim();
            let key = normalize_key(raw_key);
            record.insert(key, Value::String(val.to_string()));
        }
    }

    if !device_table.is_empty() {
        record.insert(
            "device_table".to_string(),
            Value::Array(device_table.into_iter().map(Value::Object).collect()),
        );
    }

    // Post-processing: derive additional fields and convert types
    process_derived_fields(&mut record);

    record
}

/// Process a parsed device table row into a JSON map.
fn process_device_table_row(
    cols: &[(String, Option<String>)],
    has_state: bool,
) -> Map<String, Value> {
    let mut map = Map::new();

    for (col_name, val) in cols {
        match col_name.as_str() {
            "number" => match val {
                None => {
                    map.insert("number".to_string(), Value::Null);
                }
                Some(s) => match convert_to_int(s) {
                    Some(n) => {
                        map.insert("number".to_string(), Value::Number(n.into()));
                    }
                    None => {
                        map.insert("number".to_string(), Value::Null);
                    }
                },
            },
            "major" => match val {
                None => {
                    map.insert("major".to_string(), Value::Null);
                }
                Some(s) => match convert_to_int(s) {
                    Some(n) => {
                        map.insert("major".to_string(), Value::Number(n.into()));
                    }
                    None => {
                        map.insert("major".to_string(), Value::Null);
                    }
                },
            },
            "minor" => match val {
                None => {
                    map.insert("minor".to_string(), Value::Null);
                }
                Some(s) => match convert_to_int(s) {
                    Some(n) => {
                        map.insert("minor".to_string(), Value::Number(n.into()));
                    }
                    None => {
                        map.insert("minor".to_string(), Value::Null);
                    }
                },
            },
            "raiddevice" => match val {
                None => {
                    map.insert("raid_device".to_string(), Value::Null);
                }
                Some(s) => match convert_to_int(s) {
                    Some(n) => {
                        map.insert("raid_device".to_string(), Value::Number(n.into()));
                    }
                    None => {
                        map.insert("raid_device".to_string(), Value::Null);
                    }
                },
            },
            "state" => {
                // Will be processed after "device" column is known
                // Store raw for now; we handle state+device together below
            }
            "device" => {
                // Will be processed together with state
            }
            _ => {}
        }
    }

    // Now handle state + device together (Python moves flags from device to state)
    let state_raw = cols
        .iter()
        .find(|(k, _)| k == "state")
        .and_then(|(_, v)| v.clone());
    let device_raw = cols
        .iter()
        .find(|(k, _)| k == "device")
        .and_then(|(_, v)| v.clone());

    let (final_state, final_device) = if has_state {
        match device_raw {
            None => (state_raw.unwrap_or_default(), None),
            Some(ref dev) if dev.starts_with('/') => {
                (state_raw.unwrap_or_default(), Some(dev.clone()))
            }
            Some(ref combined) => {
                // Device field contains "flags /dev/xxx" - split on last whitespace
                if let Some(last_space) = combined.rfind(char::is_whitespace) {
                    let flags = combined[..last_space].trim().to_string();
                    let dev = combined[last_space..].trim().to_string();
                    let full_state = if let Some(ref s) = state_raw {
                        format!("{} {}", s, flags)
                    } else {
                        flags
                    };
                    (full_state, Some(dev))
                } else {
                    // No space: whole thing is a state flag, no device
                    let full_state = if let Some(ref s) = state_raw {
                        format!("{} {}", s, combined)
                    } else {
                        combined.clone()
                    };
                    (full_state, None)
                }
            }
        }
    } else {
        // No state column: device is just the device path
        (String::new(), device_raw)
    };

    // Insert state as array (split on whitespace), only if non-empty
    if !final_state.is_empty() || has_state {
        let state_list: Vec<Value> = final_state
            .split_whitespace()
            .map(|s| Value::String(s.to_string()))
            .collect();
        if !state_list.is_empty() {
            map.insert("state".to_string(), Value::Array(state_list));
        }
    }

    // Insert device (null if not present)
    match final_device {
        Some(dev) if !dev.is_empty() => {
            map.insert("device".to_string(), Value::String(dev));
        }
        _ => {
            // Insert null only if has_state (to match Python behavior for missing devices)
            if has_state {
                map.insert("device".to_string(), Value::Null);
            }
        }
    }

    map
}

fn process_derived_fields(record: &mut Map<String, Value>) {
    // array_size_num: extract leading number
    if let Some(Value::String(s)) = record.get("array_size") {
        if let Some(num) = extract_leading_number(s) {
            record.insert("array_size_num".to_string(), Value::String(num.to_string()));
        }
    }

    // used_dev_size_num
    if let Some(Value::String(s)) = record.get("used_dev_size") {
        if let Some(num) = extract_leading_number(s) {
            record.insert(
                "used_dev_size_num".to_string(),
                Value::String(num.to_string()),
            );
        }
    }

    // avail_dev_size_num
    if let Some(Value::String(s)) = record.get("avail_dev_size") {
        if let Some(num) = extract_leading_number(s) {
            record.insert(
                "avail_dev_size_num".to_string(),
                Value::String(num.to_string()),
            );
        }
    }

    // data_offset: extract leading integer, store as string first then convert
    if let Some(Value::String(s)) = record.get("data_offset") {
        if let Some(num) = extract_leading_number(s) {
            record.insert("data_offset".to_string(), Value::String(num.to_string()));
        }
    }

    // super_offset: extract leading integer
    if let Some(Value::String(s)) = record.get("super_offset") {
        if let Some(num) = extract_leading_number(s) {
            record.insert("super_offset".to_string(), Value::String(num.to_string()));
        }
    }

    // unused_space: "before=X sectors, after=Y sectors" → unused_space_before/after
    if let Some(Value::String(s)) = record.get("unused_space") {
        let s_clone = s.clone();
        let parts: Vec<&str> = s_clone.split(',').collect();
        if parts.len() >= 2 {
            let before_text = parts[0].trim();
            let after_text = parts[1].trim();
            if let Some(eq_pos) = before_text.find('=') {
                let val = before_text[eq_pos + 1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                record.insert(
                    "unused_space_before".to_string(),
                    Value::String(val.to_string()),
                );
            }
            if let Some(eq_pos) = after_text.find('=') {
                let val = after_text[eq_pos + 1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                record.insert(
                    "unused_space_after".to_string(),
                    Value::String(val.to_string()),
                );
            }
        }
    }

    // name_val: first whitespace-separated token of name
    // homehost: only if name ends with ')' — take last word without ')'
    if let Some(Value::String(s)) = record.get("name") {
        let s_clone = s.clone();
        let name_val = s_clone.split_whitespace().next().unwrap_or("").to_string();
        record.insert("name_val".to_string(), Value::String(name_val));
        if s_clone.ends_with(')') {
            let homehost = s_clone
                .split_whitespace()
                .last()
                .unwrap_or("")
                .trim_end_matches(')')
                .to_string();
            record.insert("homehost".to_string(), Value::String(homehost));
        }
    }

    // uuid_val: first whitespace-separated token of uuid
    // homehost from uuid: only if uuid ends with ')'
    if let Some(Value::String(s)) = record.get("uuid") {
        let s_clone = s.clone();
        let uuid_val = s_clone.split_whitespace().next().unwrap_or("").to_string();
        record.insert("uuid_val".to_string(), Value::String(uuid_val));
        if s_clone.ends_with(')') {
            let homehost = s_clone
                .split_whitespace()
                .last()
                .unwrap_or("")
                .trim_end_matches(')')
                .to_string();
            record.insert("homehost".to_string(), Value::String(homehost));
        }
    }

    // events: if contains '.', split into events_maj/events_min; else events_num
    if let Some(Value::String(s)) = record.get("events") {
        let s_clone = s.clone();
        if s_clone.contains('.') {
            let mut parts = s_clone.splitn(2, '.');
            let maj = parts.next().unwrap_or("").to_string();
            let min = parts.next().unwrap_or("").to_string();
            record.insert("events_maj".to_string(), Value::String(maj));
            record.insert("events_min".to_string(), Value::String(min));
        } else {
            record.insert("events_num".to_string(), Value::String(s_clone));
        }
    }

    // checksum: "hash - state" → checksum_val, checksum_state
    if let Some(Value::String(s)) = record.get("checksum") {
        let s_clone = s.clone();
        let parts: Vec<&str> = s_clone.split_whitespace().collect();
        if !parts.is_empty() {
            record.insert(
                "checksum_val".to_string(),
                Value::String(parts[0].to_string()),
            );
        }
        if !parts.is_empty() {
            record.insert(
                "checksum_state".to_string(),
                Value::String(parts.last().unwrap_or(&"").to_string()),
            );
        }
    }

    // state_list: split on comma
    if let Some(Value::String(s)) = record.get("state") {
        let state_list: Vec<Value> = s
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| Value::String(s.to_string()))
            .collect();
        record.insert("state_list".to_string(), Value::Array(state_list));
    }

    // flag_list: split flags on whitespace
    if let Some(Value::String(s)) = record.get("flags") {
        let flag_list: Vec<Value> = s
            .split_whitespace()
            .map(|s| Value::String(s.to_string()))
            .collect();
        if !flag_list.is_empty() {
            record.insert("flag_list".to_string(), Value::Array(flag_list));
        }
    }

    // member_arrays_list: split member_arrays on whitespace
    if let Some(Value::String(s)) = record.get("member_arrays") {
        let list: Vec<Value> = s
            .split_whitespace()
            .map(|s| Value::String(s.to_string()))
            .collect();
        if !list.is_empty() {
            record.insert("member_arrays_list".to_string(), Value::Array(list));
        }
    }

    // array_state_list: map chars to state names
    if let Some(Value::String(s)) = record.get("array_state") {
        let s_clone = s.clone();
        let state_chars = s_clone
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '(')
            .collect::<String>();

        let state_list: Vec<Value> = state_chars
            .chars()
            .map(|c| match c {
                'A' => Value::String("active".to_string()),
                '.' | '_' => Value::String("missing".to_string()),
                'R' => Value::String("replacing".to_string()),
                other => Value::String(other.to_string()),
            })
            .collect();

        record.insert("array_state_list".to_string(), Value::Array(state_list));
    }

    // chunk_size_num: extract leading number from chunk_size (e.g. "512K" → 512)
    if let Some(Value::String(s)) = record.get("chunk_size") {
        if let Some(num) = extract_leading_number(s) {
            record.insert("chunk_size_num".to_string(), Value::String(num.to_string()));
        }
    }

    // resync_status_percent: extract number before '%'
    if let Some(Value::String(s)) = record.get("resync_status") {
        let s_clone = s.clone();
        let pct = s_clone.split('%').next().unwrap_or("").trim().to_string();
        record.insert("resync_status_percent".to_string(), Value::String(pct));
    }

    // check_status_percent
    if let Some(Value::String(s)) = record.get("check_status") {
        let s_clone = s.clone();
        let pct = s_clone.split('%').next().unwrap_or("").trim().to_string();
        record.insert("check_status_percent".to_string(), Value::String(pct));
    }

    // rebuild_status_percent
    if let Some(Value::String(s)) = record.get("rebuild_status") {
        let s_clone = s.clone();
        let pct = s_clone.split('%').next().unwrap_or("").trim().to_string();
        record.insert("rebuild_status_percent".to_string(), Value::String(pct));
    }

    // container_dev, container_member: if container has ", member "
    if let Some(Value::String(s)) = record.get("container") {
        let s_clone = s.clone();
        if s_clone.contains(", member ") {
            let dev = s_clone.split(',').next().unwrap_or("").trim().to_string();
            let member = s_clone.split_whitespace().last().unwrap_or("").to_string();
            record.insert("container_dev".to_string(), Value::String(dev));
            record.insert("container_member".to_string(), Value::String(member));
        }
    }

    // Date epoch fields
    for date_key in &["creation_time", "update_time"] {
        if let Some(Value::String(s)) = record.get(*date_key) {
            let s_clone = s.clone();
            if let Some(epoch) = parse_date_epoch(&s_clone) {
                record.insert(format!("{}_epoch", date_key), Value::Number(epoch.into()));
            }
        }
    }

    // Convert all INT_FIELDS to integers using convert_to_int
    for &field in INT_FIELDS {
        if let Some(Value::String(s)) = record.get(field) {
            let s_clone = s.clone();
            let int_val = convert_to_int(&s_clone);
            match int_val {
                Some(n) => {
                    record.insert(field.to_string(), Value::Number(n.into()));
                }
                None => {
                    record.insert(field.to_string(), Value::Null);
                }
            }
        }
    }

    // Convert integer fields in device_table entries
    const DT_INT_FIELDS: &[&str] = &["number", "major", "minor", "raid_device"];
    if let Some(Value::Array(table)) = record.get_mut("device_table") {
        for item in table.iter_mut() {
            if let Value::Object(map) = item {
                for &field in DT_INT_FIELDS {
                    if let Some(Value::String(s)) = map.get(field) {
                        let s_clone = s.clone();
                        match convert_to_int(&s_clone) {
                            Some(n) => {
                                map.insert(field.to_string(), Value::Number(n.into()));
                            }
                            None => {
                                map.insert(field.to_string(), Value::Null);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn extract_leading_number(s: &str) -> Option<i64> {
    let s = s.trim();
    let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdadm_examine_raid1_ok() {
        let input = include_str!("../../../../tests/fixtures/generic/mdadm-examine-raid1-ok.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/mdadm-examine-raid1-ok.json"
        ))
        .unwrap();

        let parser = MdadmParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mdadm_query_raid1_ok() {
        let input = include_str!("../../../../tests/fixtures/generic/mdadm-query-raid1-ok.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/mdadm-query-raid1-ok.json"
        ))
        .unwrap();

        let parser = MdadmParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("Raid Level"), "raid_level");
        assert_eq!(normalize_key("Array UUID"), "array_uuid");
        assert_eq!(normalize_key("Creation Time"), "creation_time");
    }

    #[test]
    fn test_convert_to_int() {
        assert_eq!(convert_to_int("this     0"), Some(0));
        assert_eq!(convert_to_int("0     0"), Some(0));
        assert_eq!(convert_to_int("1     1"), Some(11));
        assert_eq!(convert_to_int("2     2"), Some(22));
        assert_eq!(convert_to_int("-"), None);
        assert_eq!(convert_to_int(""), None);
        assert_eq!(convert_to_int("512"), Some(512));
    }
}

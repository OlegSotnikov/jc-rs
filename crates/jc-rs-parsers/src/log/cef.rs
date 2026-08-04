//! Common Event Format (CEF) parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

struct CefParser;

static INFO: ParserInfo = ParserInfo {
    name: "cef",
    argument: "--cef",
    version: "1.0.0",
    description: "Common Event Format (CEF) string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

// CEF known integer extension fields (must match jc's extended_ints set)
static INT_FIELDS: &[&str] = &[
    "cnt", "dpid", "dpt", "dvcpid", "eventId", "in", "out", "spt", "spid", "type", "cn1", "cn2",
    "cn3",
];

// CEF fields that get timestamp epoch processing
static TIMESTAMP_FIELDS: &[&str] = &[
    "start",
    "end",
    "rt",
    "deviceCustomDate1",
    "deviceCustomDate2",
    "flexDate1",
];

// Label prefix patterns (e.g. cn1Label, cs1Label, cfp1Label, etc.)
// Format: (prefix, type) where type is "int", "float", "date", "string"
static LABEL_PREFIXES: &[(&str, &str)] = &[
    ("cn1", "int"),
    ("cn2", "int"),
    ("cn3", "int"),
    ("cn4", "int"),
    ("cn5", "int"),
    ("cn6", "int"),
    ("cfp1", "float"),
    ("cfp2", "float"),
    ("cfp3", "float"),
    ("cfp4", "float"),
    ("cs1", "string"),
    ("cs2", "string"),
    ("cs3", "string"),
    ("cs4", "string"),
    ("cs5", "string"),
    ("cs6", "string"),
    ("deviceCustomDate1", "date"),
    ("deviceCustomDate2", "date"),
    ("flexDate1", "date"),
    ("flexNumber1", "int"),
    ("flexNumber2", "int"),
    ("flexNumber3", "int"),
    ("flexNumber4", "int"),
    ("flexString1", "string"),
    ("flexString2", "string"),
    ("flexString3", "string"),
    ("flexString4", "string"),
    ("flexString5", "string"),
    ("flexString6", "string"),
];

fn severity_to_string(sev: i64) -> Option<&'static str> {
    match sev {
        0..=3 => Some("Low"),
        4..=6 => Some("Medium"),
        7..=8 => Some("High"),
        9..=10 => Some("Very-High"),
        _ => None,
    }
}

/// Split CEF header fields (pipe-delimited, respecting `\|` escape).
fn split_cef_header(s: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            current.push('|');
            i += 2;
        } else if bytes[i] == b'|' {
            fields.push(current.clone());
            current.clear();
            i += 1;
        } else {
            current.push(bytes[i] as char);
            i += 1;
        }
    }
    fields.push(current);
    fields
}

/// Find all `word=` positions in a CEF extension string.
/// Returns (key_normalized, key_start_byte, value_start_byte) triples.
fn find_key_positions(ext: &str) -> Vec<(String, usize, usize)> {
    let mut positions: Vec<(String, usize, usize)> = Vec::new();
    let bytes = ext.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'_'
            || bytes[i] == b'.'
            || bytes[i] == b'-'
            || bytes[i] == b'/'
        {
            let key_start = i;
            // Read key chars
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || bytes[i] == b'.'
                    || bytes[i] == b'-'
                    || bytes[i] == b'/')
            {
                i += 1;
            }
            // Must be followed by '='
            if i < bytes.len() && bytes[i] == b'=' {
                let key_raw = &ext[key_start..i];
                // Normalize key: replace ., -, / with _
                let key = key_raw
                    .replace('.', "_")
                    .replace('-', "_")
                    .replace('/', "_");
                positions.push((key, key_start, i + 1));
                i += 1; // skip '='
            }
        } else {
            i += 1;
        }
    }
    positions
}

/// Unescape CEF extension values: \= → =, \\ → \, \n → newline, \r → CR, \| → |, \" → "
fn unescape_cef_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&'=') => {
                    chars.next();
                    result.push('=');
                }
                Some(&'\\') => {
                    chars.next();
                    result.push('\\');
                }
                Some(&'n') => {
                    chars.next();
                    result.push('\n');
                }
                Some(&'r') => {
                    chars.next();
                    result.push('\r');
                }
                Some(&'|') => {
                    chars.next();
                    result.push('|');
                }
                Some(&']') => {
                    chars.next();
                    result.push(']');
                }
                Some(&'"') => {
                    chars.next();
                    result.push('"');
                }
                _ => {
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Add epoch fields for a timestamp value.
fn add_epoch_fields(map: &mut Map<String, Value>, field_prefix: &str, ts_value: &str) {
    // If the value is a pure integer (possibly ms epoch), parse numerically
    let trimmed = ts_value.trim();

    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        if let Ok(n) = trimmed.parse::<i64>() {
            // If > 1e12, treat as milliseconds
            let epoch = if n > 1_000_000_000_000 { n / 1000 } else { n };
            map.insert(format!("{field_prefix}_epoch"), Value::Number(epoch.into()));
            map.insert(format!("{field_prefix}_epoch_utc"), Value::Null);
            return;
        }
    }

    let parsed = parse_timestamp(
        trimmed,
        &[
            formats::F1400,
            formats::F1410,
            formats::F1420,
            formats::F1430,
        ],
    );
    map.insert(
        format!("{field_prefix}_epoch"),
        match parsed.naive_epoch {
            Some(e) => Value::Number(e.into()),
            None => Value::Null,
        },
    );
    map.insert(
        format!("{field_prefix}_epoch_utc"),
        match parsed.utc_epoch {
            Some(e) => Value::Number(e.into()),
            None => Value::Null,
        },
    );
}

pub fn parse_cef_line(line: &str) -> Map<String, Value> {
    let mut map = Map::new();

    // Find "CEF:" in the line (may be preceded by syslog/datetime prefix)
    let cef_pos = match line.find("CEF:") {
        Some(p) => p,
        None => {
            map.insert("unparsable".to_string(), Value::String(line.to_string()));
            return map;
        }
    };

    let cef_str = &line[cef_pos..];

    // Split on first '|' that is not escaped: CEF:version|vendor|product|devVersion|classId|name|severity|extension
    // The header has exactly 7 pipes (8 fields including CEF:version)
    let parts = split_cef_header(cef_str);
    if parts.len() < 7 {
        map.insert("unparsable".to_string(), Value::String(line.to_string()));
        return map;
    }

    // parts[0] = "CEF:version"
    let cef_version_str = parts[0].trim_start_matches("CEF:");
    let cef_version: i64 = cef_version_str.parse().unwrap_or(0);
    map.insert("deviceVendor".to_string(), Value::String(parts[1].clone()));
    map.insert("deviceProduct".to_string(), Value::String(parts[2].clone()));
    map.insert("deviceVersion".to_string(), Value::String(parts[3].clone()));
    map.insert(
        "deviceEventClassId".to_string(),
        Value::String(parts[4].clone()),
    );
    map.insert("name".to_string(), Value::String(parts[5].clone()));
    let severity_str = parts[6].clone();
    map.insert(
        "agentSeverity".to_string(),
        Value::String(severity_str.clone()),
    );
    map.insert("CEFVersion".to_string(), Value::Number(cef_version.into()));

    // Extension (if present)
    let ext = if parts.len() > 7 {
        parts[7..].join("|")
    } else {
        String::new()
    };

    // Parse extension key=value pairs
    if !ext.is_empty() {
        let positions = find_key_positions(&ext);
        let mut raw_pairs: Vec<(String, String)> = Vec::new();

        for i in 0..positions.len() {
            let (key, _key_start, val_start) = &positions[i];
            let val_end = if i + 1 < positions.len() {
                positions[i + 1].1 // next key's byte start
            } else {
                ext.len()
            };

            let val_raw = &ext[*val_start..val_end];
            let val_trimmed = val_raw.trim();

            // Skip if value is just a quote char (artifact of msg="..." CEF quoting)
            if val_trimmed == "\"" || val_trimmed.is_empty() {
                continue;
            }

            // Strip leading " if present (from quoted values like msg="content")
            let val_stripped = if val_trimmed.starts_with('"') {
                &val_trimmed[1..]
            } else {
                val_trimmed
            };

            let val_unescaped = unescape_cef_value(val_stripped);
            raw_pairs.push((key.clone(), val_unescaped));
        }

        // Process raw pairs: handle label-based renaming first
        let mut raw_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in &raw_pairs {
            raw_map.insert(k.clone(), v.clone());
        }

        // Build set of keys to skip (original label keys and labeled value keys)
        let mut skip_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
        // (label_name, orig_val_for_epoch, converted_value, is_date)
        let mut label_replacements: Vec<(String, String, Value, bool)> = Vec::new();

        // Build type lookup from LABEL_PREFIXES for type-specific conversion
        let type_map: std::collections::HashMap<&str, &str> =
            LABEL_PREFIXES.iter().copied().collect();

        // Handle any key ending in "Label" — generic label renaming.
        // Covers cs1-cs6, cn1-cn6, cfp1-cfp4, deviceCustomDate*, flexDate*
        // as well as arbitrary cs7, cs8, cs9, cs11, etc. not in LABEL_PREFIXES.
        for (key, val) in &raw_pairs {
            if key.ends_with("Label") {
                let field_key = &key[..key.len() - 5];
                if let Some(orig_val) = raw_map.get(field_key) {
                    // Normalize label value as key name: replace non-alphanumeric with '_', strip edges
                    let label_name: String = val
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                        .trim_matches('_')
                        .to_string();
                    let type_hint = type_map.get(field_key).copied().unwrap_or("string");
                    let converted: Value = match type_hint {
                        "int" => orig_val
                            .trim()
                            .parse::<i64>()
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or_else(|_| Value::String(orig_val.clone())),
                        "float" => orig_val
                            .trim()
                            .parse::<f64>()
                            .ok()
                            .and_then(|f| serde_json::Number::from_f64(f))
                            .map(Value::Number)
                            .unwrap_or_else(|| Value::String(orig_val.clone())),
                        _ => Value::String(orig_val.clone()),
                    };
                    let is_date = type_hint == "date";
                    label_replacements.push((label_name, orig_val.clone(), converted, is_date));
                    skip_keys.insert(field_key.to_string());
                    skip_keys.insert(key.clone());
                }
            }
        }

        // Insert non-skipped raw pairs
        for (key, val) in &raw_pairs {
            if skip_keys.contains(key) {
                continue;
            }
            // Type conversion for known int fields
            // Timestamp fields always stay as strings; int conversion only for non-timestamp fields
            let json_val: Value = if TIMESTAMP_FIELDS.contains(&key.as_str()) {
                Value::String(val.clone())
            } else if INT_FIELDS.contains(&key.as_str()) {
                val.trim()
                    .parse::<i64>()
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or_else(|_| Value::String(val.clone()))
            } else {
                Value::String(val.clone())
            };

            // Add epoch for known timestamp fields
            if TIMESTAMP_FIELDS.contains(&key.as_str()) {
                map.insert(key.clone(), json_val);
                add_epoch_fields(&mut map, key, val);
            } else {
                map.insert(key.clone(), json_val);
            }
        }

        // Insert label replacements
        for (label_name, orig_val, converted, is_date) in &label_replacements {
            map.insert(label_name.clone(), converted.clone());
            if *is_date {
                add_epoch_fields(&mut map, label_name, orig_val);
            }
        }
    }

    // Compute agentSeverityString, agentSeverityNum, deviceEventClassIdNum
    let sev_num = severity_str.trim().parse::<i64>().ok();
    let (sev_string, sev_num_val) = if let Some(n) = sev_num {
        let s = severity_to_string(n).map(|s| Value::String(s.to_string()));
        (s, Some(Value::Number(n.into())))
    } else {
        // String severity — only add fields if it's a known severity string
        let lower = severity_str.to_lowercase();
        match lower.as_str() {
            "low" | "medium" | "high" | "very-high" => {
                (Some(Value::String(severity_str.clone())), Some(Value::Null))
            }
            _ => (None, None),
        }
    };

    if let Some(s) = sev_string {
        map.insert("agentSeverityString".to_string(), s);
    }
    if let Some(n) = sev_num_val {
        map.insert("agentSeverityNum".to_string(), n);
    }

    // deviceEventClassIdNum
    let class_id_num = parts[4].trim().parse::<i64>().ok();
    map.insert(
        "deviceEventClassIdNum".to_string(),
        match class_id_num {
            Some(n) => Value::Number(n.into()),
            None => Value::Null,
        },
    );

    map
}

impl Parser for CefParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let records: Vec<Map<String, Value>> = input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_cef_line)
            .collect();
        Ok(ParseOutput::Array(records))
    }
}

static INSTANCE: CefParser = CefParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_cef_registered() {
        assert!(find_parser("cef").is_some());
    }

    #[test]
    fn test_cef_basic_header() {
        let line = "CEF:0|Fortinet|FortiDeceptor|3.2.0|1|SYSTEM|1|date=2020-12-08";
        let map = parse_cef_line(line);
        assert_eq!(map["deviceVendor"], serde_json::json!("Fortinet"));
        assert_eq!(map["deviceProduct"], serde_json::json!("FortiDeceptor"));
        assert_eq!(map["deviceVersion"], serde_json::json!("3.2.0"));
        assert_eq!(map["deviceEventClassId"], serde_json::json!("1"));
        assert_eq!(map["name"], serde_json::json!("SYSTEM"));
        assert_eq!(map["agentSeverity"], serde_json::json!("1"));
        assert_eq!(map["CEFVersion"], serde_json::json!(0));
        assert_eq!(map["agentSeverityNum"], serde_json::json!(1));
        assert_eq!(map["agentSeverityString"], serde_json::json!("Low"));
        assert_eq!(map["deviceEventClassIdNum"], serde_json::json!(1));
    }

    #[test]
    fn test_cef_severity_medium() {
        let line = "CEF:0|Trend Micro|DSA|1.0|1|name|6|field=val";
        let map = parse_cef_line(line);
        assert_eq!(map["agentSeverityNum"], serde_json::json!(6));
        assert_eq!(map["agentSeverityString"], serde_json::json!("Medium"));
    }

    #[test]
    fn test_cef_unparsable() {
        let line = "unparsable line";
        let map = parse_cef_line(line);
        assert!(map.contains_key("unparsable"));
    }

    #[test]
    fn test_cef_label_renaming() {
        let line = "CEF:0|Vendor|Product|1.0|1|name|6|cn1=42 cn1Label=MyCount dvchost=host";
        let map = parse_cef_line(line);
        assert_eq!(map["MyCount"], serde_json::json!(42));
        assert!(!map.contains_key("cn1"));
        assert!(!map.contains_key("cn1Label"));
        assert_eq!(map["dvchost"], serde_json::json!("host"));
    }

    #[test]
    fn test_cef_float_label() {
        let line = "CEF:0|Vendor|Product|1.0|1|name|6|cfp1=3.14 cfp1Label=myFloat";
        let map = parse_cef_line(line);
        assert_eq!(map["myFloat"], serde_json::json!(3.14));
    }

    #[test]
    fn test_cef_timestamp_epoch() {
        let line = "CEF:0|Elastic|Vaporware|1.0.0|18|Web request|low|start=1453290121336";
        let map = parse_cef_line(line);
        assert_eq!(map["start_epoch"], serde_json::json!(1453290121i64));
        assert!(map["start_epoch_utc"].is_null());
    }

    #[test]
    fn test_cef_fixture() {
        let input = get_fixture("generic/cef.out");
        let parser = find_parser("cef").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let arr = match result {
            ParseOutput::Array(v) => v,
            _ => panic!("expected array"),
        };
        assert!(!arr.is_empty());
        // First record: Fortinet
        assert_eq!(arr[0]["deviceVendor"], serde_json::json!("Fortinet"));
        assert_eq!(arr[0]["agentSeverityString"], serde_json::json!("Low"));
        // Unparsable
        let has_unparsable = arr.iter().any(|r| r.contains_key("unparsable"));
        assert!(has_unparsable);
        // Record with labels (index 2)
        assert!(arr[2].contains_key("Host_ID") || arr[2].contains_key("dvchost"));
        // Elastic record (index 10)
        assert_eq!(arr[10]["deviceVendor"], serde_json::json!("Elastic"));
        assert_eq!(arr[10]["agentSeverity"], serde_json::json!("low"));
    }
}

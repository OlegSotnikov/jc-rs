//! Apple plist file parser.
//!
//! Parses XML, binary (bplist00), and NeXTSTEP plist files into JSON.
//! - Dates are converted to Unix timestamps with a corresponding _iso key
//! - Byte data is converted to colon-delimited hex strings

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct PlistParser;

static PLIST_INFO: ParserInfo = ParserInfo {
    name: "plist",
    argument: "--plist",
    version: "1.0.0",
    description: "Apple plist file parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Convert bytes to colon-separated hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Convert a plist::Value into a serde_json::Value with the key context.
/// When a datetime is encountered, it needs to be inserted as two keys in the parent map.
/// We use a recursive approach where datetimes are wrapped in a special enum.
enum JsonValOrDatetime {
    Json(serde_json::Value),
    Datetime { ts: i64, iso: String },
}

fn plist_val_to_json_inner(val: plist::Value) -> JsonValOrDatetime {
    match val {
        plist::Value::Boolean(b) => JsonValOrDatetime::Json(serde_json::Value::Bool(b)),
        plist::Value::Integer(i) => {
            let n = i
                .as_signed()
                .map(|s| serde_json::Value::Number(s.into()))
                .unwrap_or_else(|| {
                    serde_json::Value::Number((i.as_unsigned().unwrap_or(0)).into())
                });
            JsonValOrDatetime::Json(n)
        }
        plist::Value::Real(f) => {
            let n = serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0));
            JsonValOrDatetime::Json(serde_json::Value::Number(n))
        }
        plist::Value::String(s) => JsonValOrDatetime::Json(serde_json::Value::String(s)),
        plist::Value::Data(bytes) => {
            JsonValOrDatetime::Json(serde_json::Value::String(bytes_to_hex(&bytes)))
        }
        plist::Value::Date(dt) => {
            // plist::Date represents seconds since 2001-01-01 (Apple epoch)
            // Convert to Unix timestamp
            use std::time::UNIX_EPOCH;
            let system_time: std::time::SystemTime = dt.into();
            let ts = system_time
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // Format ISO string without timezone for naive times, with tz for aware
            let iso = format_plist_date_iso(dt);
            JsonValOrDatetime::Datetime { ts, iso }
        }
        plist::Value::Array(arr) => {
            let json_arr: Vec<serde_json::Value> = arr
                .into_iter()
                .map(|v| match plist_val_to_json_inner(v) {
                    JsonValOrDatetime::Json(j) => j,
                    JsonValOrDatetime::Datetime { ts, .. } => serde_json::Value::Number(ts.into()),
                })
                .collect();
            JsonValOrDatetime::Json(serde_json::Value::Array(json_arr))
        }
        plist::Value::Dictionary(dict) => {
            let mut map = serde_json::Map::new();
            for (k, v) in dict {
                match plist_val_to_json_inner(v) {
                    JsonValOrDatetime::Json(j) => {
                        map.insert(k, j);
                    }
                    JsonValOrDatetime::Datetime { ts, iso } => {
                        map.insert(k.clone(), serde_json::Value::Number(ts.into()));
                        map.insert(format!("{k}_iso"), serde_json::Value::String(iso));
                    }
                }
            }
            JsonValOrDatetime::Json(serde_json::Value::Object(map))
        }
        plist::Value::Uid(uid) => {
            JsonValOrDatetime::Json(serde_json::Value::Number(uid.get().into()))
        }
        _ => JsonValOrDatetime::Json(serde_json::Value::Null),
    }
}

/// Format a plist::Date as an ISO string.
fn format_plist_date_iso(dt: plist::Date) -> String {
    use std::time::UNIX_EPOCH;
    let system_time: std::time::SystemTime = dt.into();
    let ts = system_time.duration_since(UNIX_EPOCH).unwrap_or_default();

    // Use chrono to format
    use chrono::{TimeZone, Utc};
    let secs = ts.as_secs() as i64;
    let nanos = ts.subsec_nanos();
    let dt = Utc.timestamp_opt(secs, nanos).single();

    match dt {
        Some(d) => {
            if nanos == 0 {
                d.format("%Y-%m-%dT%H:%M:%S").to_string()
            } else {
                // Include subsecond precision
                let sub = nanos as f64 / 1_000_000_000.0;
                let sub_str = format!("{sub:.6}")[1..].to_string(); // e.g. ".123456"
                format!("{}{}", d.format("%Y-%m-%dT%H:%M:%S"), sub_str)
            }
        }
        None => "invalid".to_string(),
    }
}

/// Parse plist bytes into a JSON map.
fn parse_plist_bytes(
    bytes: &[u8],
) -> Result<serde_json::Map<String, serde_json::Value>, ParseError> {
    let val = plist::Value::from_reader(std::io::Cursor::new(bytes))
        .map_err(|e| ParseError::Generic(format!("plist parse error: {e}")))?;

    match plist_val_to_json_inner(val) {
        JsonValOrDatetime::Json(serde_json::Value::Object(map)) => Ok(map),
        JsonValOrDatetime::Json(other) => {
            let mut map = serde_json::Map::new();
            map.insert("value".to_string(), other);
            Ok(map)
        }
        JsonValOrDatetime::Datetime { ts, iso } => {
            let mut map = serde_json::Map::new();
            map.insert("value".to_string(), serde_json::Value::Number(ts.into()));
            map.insert("value_iso".to_string(), serde_json::Value::String(iso));
            Ok(map)
        }
    }
}

impl Parser for PlistParser {
    fn info(&self) -> &'static ParserInfo {
        &PLIST_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let bytes = input.as_bytes();
        let map = parse_plist_bytes(bytes)?;
        Ok(ParseOutput::Object(map))
    }
}

static PLIST_PARSER_INSTANCE: PlistParser = PlistParser;

inventory::submit! {
    ParserEntry::new(&PLIST_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/generic");

    fn load_fixture_str(name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn load_fixture_bytes(name: &str) -> Vec<u8> {
        std::fs::read(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn parse_json_obj(s: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(s).expect("invalid fixture JSON")
    }

    /// Remove timestamp fields from a JSON map recursively, so we can compare
    /// plist output independent of timezone-dependent timestamps.
    fn remove_timestamp_fields(val: &mut serde_json::Value) {
        match val {
            serde_json::Value::Object(map) => {
                let ts_keys: Vec<String> = map
                    .keys()
                    .filter(|k| {
                        !k.ends_with("_iso") && {
                            let iso_key = format!("{k}_iso");
                            map.contains_key(&iso_key)
                        }
                    })
                    .cloned()
                    .collect();
                for k in ts_keys {
                    map.remove(&k);
                }
                for v in map.values_mut() {
                    remove_timestamp_fields(v);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr.iter_mut() {
                    remove_timestamp_fields(v);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_plist_xml_alltypes() {
        let input = load_fixture_str("plist-alltypes.plist");
        let mut expected =
            serde_json::Value::Object(parse_json_obj(&load_fixture_str("plist-alltypes.json")));
        let parser = PlistParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(mut map) = result {
            // Remove timezone-dependent timestamp fields; compare only iso strings and non-date values
            remove_timestamp_fields(&mut serde_json::Value::Object(map.clone()));
            remove_timestamp_fields(&mut expected);
            let mut result_val = serde_json::Value::Object(map);
            remove_timestamp_fields(&mut result_val);
            assert_eq!(result_val, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_plist_binary_alltypes() {
        let bytes = load_fixture_bytes("plist-alltypes-bin.plist");
        let mut expected =
            serde_json::Value::Object(parse_json_obj(&load_fixture_str("plist-alltypes-bin.json")));
        let map = parse_plist_bytes(&bytes).unwrap();
        let mut result_val = serde_json::Value::Object(map);
        remove_timestamp_fields(&mut result_val);
        remove_timestamp_fields(&mut expected);
        assert_eq!(result_val, expected);
    }

    #[test]
    fn test_plist_garageband() {
        // garageband is a binary plist — read as bytes and parse directly
        let bytes = load_fixture_bytes("plist-garageband-info.plist");
        let mut expected = serde_json::Value::Object(parse_json_obj(&load_fixture_str(
            "plist-garageband-info.json",
        )));
        let map = parse_plist_bytes(&bytes).unwrap();
        let mut result_val = serde_json::Value::Object(map);
        remove_timestamp_fields(&mut result_val);
        remove_timestamp_fields(&mut expected);
        assert_eq!(result_val, expected);
    }
}

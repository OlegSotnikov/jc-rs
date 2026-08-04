//! `_jc_meta` injection into parser output.
//!
//! When `--meta-out` is enabled the CLI inserts a `_jc_meta` key with
//! contextual information (parser name, timestamp, slice info, magic command).
//!
//! Mirrors `JcCli::add_metadata_to_output()` from the original Python.

use serde_json::{Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// All the fields that end up inside `_jc_meta`.
#[derive(Debug, Default)]
pub struct MetaInfo {
    pub parser: String,
    pub timestamp: f64,
    pub slice_start: Option<i64>,
    pub slice_end: Option<i64>,
    pub magic_command: Option<Vec<String>>,
    pub magic_command_exit: Option<i32>,
    pub input_list: Option<Vec<String>>,
}

impl MetaInfo {
    /// Create a new MetaInfo with the current UTC timestamp.
    pub fn new_now(parser: impl Into<String>) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        MetaInfo {
            parser: parser.into(),
            timestamp: ts,
            ..Default::default()
        }
    }

    /// Serialize to a serde_json::Map for injection.
    pub fn to_json_map(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("parser".to_string(), Value::String(self.parser.clone()));
        m.insert(
            "timestamp".to_string(),
            Value::Number(
                serde_json::Number::from_f64(self.timestamp).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        m.insert(
            "slice_start".to_string(),
            match self.slice_start {
                Some(v) => Value::Number(v.into()),
                None => Value::Null,
            },
        );
        m.insert(
            "slice_end".to_string(),
            match self.slice_end {
                Some(v) => Value::Number(v.into()),
                None => Value::Null,
            },
        );
        if let Some(ref cmd) = self.magic_command {
            m.insert(
                "magic_command".to_string(),
                Value::Array(cmd.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if let Some(exit_code) = self.magic_command_exit {
            m.insert(
                "magic_command_exit".to_string(),
                Value::Number(exit_code.into()),
            );
        }
        if let Some(ref il) = self.input_list {
            m.insert(
                "input_list".to_string(),
                Value::Array(il.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        m
    }
}

/// Inject `_jc_meta` into a `Value::Object`.
fn inject_into_object(obj: &mut Map<String, Value>, meta: &Map<String, Value>) {
    let entry = obj
        .entry("_jc_meta".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(existing) = entry {
        existing.extend(meta.clone());
    } else {
        *entry = Value::Object(meta.clone());
    }
}

/// Inject `_jc_meta` into the output Value.
///
/// - `Object`: inject directly.
/// - `Array`: inject into each element that is an Object.
///   If the array is empty, push a single `{_jc_meta: ...}` object.
pub fn inject_meta(value: &mut Value, meta: &MetaInfo) {
    let meta_map = meta.to_json_map();
    match value {
        Value::Object(obj) => {
            inject_into_object(obj, &meta_map);
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                let mut empty_obj = Map::new();
                inject_into_object(&mut empty_obj, &meta_map);
                arr.push(Value::Object(empty_obj));
            } else {
                for item in arr.iter_mut() {
                    if let Value::Object(obj) = item {
                        inject_into_object(obj, &meta_map);
                    }
                }
            }
        }
        _ => {
            eprintln!(
                "jc-rs: warning - Parser returned an unsupported object type for meta injection."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── MetaInfo ──────────────────────────────────────────────────────────────

    #[test]
    fn test_meta_info_new_now_sets_parser_and_timestamp() {
        let m = MetaInfo::new_now("test_parser");
        assert_eq!(m.parser, "test_parser");
        assert!(m.timestamp > 0.0, "timestamp should be positive");
    }

    #[test]
    fn test_meta_info_default_optional_fields_are_none() {
        let m = MetaInfo::new_now("p");
        assert!(m.slice_start.is_none());
        assert!(m.slice_end.is_none());
        assert!(m.magic_command.is_none());
        assert!(m.magic_command_exit.is_none());
        assert!(m.input_list.is_none());
    }

    #[test]
    fn test_meta_info_to_json_map_has_required_keys() {
        let m = MetaInfo::new_now("df");
        let map = m.to_json_map();
        assert!(map.contains_key("parser"));
        assert!(map.contains_key("timestamp"));
        assert!(map.contains_key("slice_start"));
        assert!(map.contains_key("slice_end"));
        assert_eq!(map["parser"], Value::String("df".to_string()));
    }

    #[test]
    fn test_meta_info_to_json_map_slice_fields_null_when_none() {
        let m = MetaInfo::new_now("ls");
        let map = m.to_json_map();
        assert_eq!(map["slice_start"], Value::Null);
        assert_eq!(map["slice_end"], Value::Null);
    }

    #[test]
    fn test_meta_info_to_json_map_slice_populated() {
        let m = MetaInfo {
            parser: "ps".to_string(),
            timestamp: 1000.0,
            slice_start: Some(3),
            slice_end: Some(7),
            ..Default::default()
        };
        let map = m.to_json_map();
        assert_eq!(map["slice_start"], json!(3_i64));
        assert_eq!(map["slice_end"], json!(7_i64));
    }

    #[test]
    fn test_meta_info_magic_command_and_exit_serialized() {
        let m = MetaInfo {
            parser: "ls".to_string(),
            timestamp: 0.0,
            magic_command: Some(vec!["ls".to_string(), "-al".to_string()]),
            magic_command_exit: Some(0),
            ..Default::default()
        };
        let map = m.to_json_map();
        let cmd = map["magic_command"].as_array().expect("array");
        assert_eq!(cmd.len(), 2);
        assert_eq!(cmd[0], json!("ls"));
        assert_eq!(cmd[1], json!("-al"));
        assert_eq!(map["magic_command_exit"], json!(0_i32));
    }

    #[test]
    fn test_meta_info_input_list_serialized() {
        let m = MetaInfo {
            parser: "file".to_string(),
            timestamp: 0.0,
            input_list: Some(vec!["a.txt".to_string(), "b.txt".to_string()]),
            ..Default::default()
        };
        let map = m.to_json_map();
        let il = map["input_list"].as_array().expect("array");
        assert_eq!(il.len(), 2);
        assert_eq!(il[0], json!("a.txt"));
    }

    // ── inject_meta ───────────────────────────────────────────────────────────

    #[test]
    fn test_inject_meta_adds_jc_meta_to_object() {
        let meta = MetaInfo {
            parser: "df".to_string(),
            timestamp: 1234.5,
            ..Default::default()
        };
        let mut v = json!({"filesystem": "/dev/sda1"});
        inject_meta(&mut v, &meta);
        assert!(
            v.get("_jc_meta").is_some(),
            "_jc_meta key should be injected"
        );
        assert_eq!(v["_jc_meta"]["parser"], json!("df"));
    }

    #[test]
    fn test_inject_meta_injects_into_each_array_element() {
        let meta = MetaInfo {
            parser: "ls".to_string(),
            timestamp: 100.0,
            ..Default::default()
        };
        let mut v = json!([{"name": "file1"}, {"name": "file2"}]);
        inject_meta(&mut v, &meta);
        let arr = v.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["_jc_meta"]["parser"], json!("ls"));
        assert_eq!(arr[1]["_jc_meta"]["parser"], json!("ls"));
    }

    #[test]
    fn test_inject_meta_empty_array_adds_singleton() {
        let meta = MetaInfo {
            parser: "empty".to_string(),
            timestamp: 0.0,
            ..Default::default()
        };
        let mut v = json!([]);
        inject_meta(&mut v, &meta);
        let arr = v.as_array().expect("array");
        assert_eq!(arr.len(), 1, "empty array should get one sentinel object");
        assert_eq!(arr[0]["_jc_meta"]["parser"], json!("empty"));
    }

    #[test]
    fn test_inject_meta_preserves_existing_keys() {
        let meta = MetaInfo {
            parser: "df".to_string(),
            timestamp: 50.0,
            ..Default::default()
        };
        let mut v = json!({"filesystem": "/tmp"});
        inject_meta(&mut v, &meta);
        // Original key still present
        assert_eq!(v["filesystem"], json!("/tmp"));
        // Meta added
        assert!(v["_jc_meta"].is_object());
    }
}

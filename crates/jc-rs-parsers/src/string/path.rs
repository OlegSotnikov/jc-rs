//! POSIX path string parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

struct PathParser;

static PATH_INFO: ParserInfo = ParserInfo {
    name: "path",
    argument: "--path",
    version: "1.0.0",
    description: "POSIX path string parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Parse a path string into a Map object (reusable by path_list parser)
pub fn parse_path_str(path_str: &str) -> Map<String, Value> {
    let path_str = path_str.trim_end_matches('\n');

    // Detect Windows paths by presence of backslash
    if path_str.contains('\\') {
        return parse_windows_path(path_str);
    }

    let p = std::path::Path::new(path_str);

    let parent = p
        .parent()
        .map(|pp| pp.to_string_lossy().to_string())
        .unwrap_or_default();

    let filename = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let extension = p
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    // path_list: parts as Python pathlib returns them (includes "/" as first element for absolute paths)
    let path_list: Vec<Value> = p
        .components()
        .map(|c| Value::String(c.as_os_str().to_string_lossy().to_string()))
        .collect();

    let mut map = Map::new();
    map.insert("path".to_string(), Value::String(path_str.to_string()));
    map.insert("parent".to_string(), Value::String(parent));
    map.insert("filename".to_string(), Value::String(filename));
    map.insert("stem".to_string(), Value::String(stem));
    map.insert("extension".to_string(), Value::String(extension));
    map.insert("path_list".to_string(), Value::Array(path_list));

    map
}

/// Parse a Windows-style path (backslash-separated, drive letter optional)
fn parse_windows_path(path_str: &str) -> Map<String, Value> {
    // Normalize: strip trailing backslashes unless the path is just a drive root (e.g. "c:\")
    // Matches PureWindowsPath normalization in Python
    let normalized: String = if path_str.ends_with('\\') {
        let stripped = path_str.trim_end_matches('\\');
        // Restore trailing backslash only for bare drive roots like "c:"
        if stripped.len() == 2 && stripped.ends_with(':') {
            format!("{}\\", stripped)
        } else {
            stripped.to_string()
        }
    } else {
        path_str.to_string()
    };

    // Split on backslash to get components
    let raw_parts: Vec<&str> = normalized.split('\\').collect();

    // Build path_list: first component is drive root (e.g. "c:\"), rest are plain names
    let mut path_list: Vec<Value> = Vec::new();
    for (i, part) in raw_parts.iter().enumerate() {
        if i == 0 {
            // Drive letter (e.g. "c:" → "c:\")
            if part.ends_with(':') {
                path_list.push(Value::String(format!("{}\\", part)));
            } else if !part.is_empty() {
                path_list.push(Value::String(part.to_string()));
            }
        } else if !part.is_empty() {
            path_list.push(Value::String(part.to_string()));
        }
    }

    // Filename: last non-empty component
    let filename = raw_parts
        .iter()
        .rev()
        .find(|p| !p.is_empty())
        .copied()
        .unwrap_or("")
        .to_string();

    // Parent: everything before the last backslash
    // If the parent is a bare drive letter (e.g. "C:"), add backslash to match PureWindowsPath
    let parent = if let Some(pos) = normalized.rfind('\\') {
        let p = &normalized[..pos];
        if p.ends_with(':') {
            format!("{}\\", p)
        } else {
            p.to_string()
        }
    } else {
        String::new()
    };

    // Stem and extension
    let (stem, extension) = if let Some(dot_pos) = filename.rfind('.') {
        if dot_pos == 0 {
            (filename.clone(), String::new())
        } else {
            (
                filename[..dot_pos].to_string(),
                filename[dot_pos + 1..].to_string(),
            )
        }
    } else {
        (filename.clone(), String::new())
    };

    let mut map = Map::new();
    map.insert("path".to_string(), Value::String(normalized));
    map.insert("parent".to_string(), Value::String(parent));
    map.insert("filename".to_string(), Value::String(filename));
    map.insert("stem".to_string(), Value::String(stem));
    map.insert("extension".to_string(), Value::String(extension));
    map.insert("path_list".to_string(), Value::Array(path_list));

    map
}

impl Parser for PathParser {
    fn info(&self) -> &'static ParserInfo {
        &PATH_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let map = parse_path_str(input);
        Ok(ParseOutput::Object(map))
    }
}

static PATH_PARSER_INSTANCE: PathParser = PathParser;

inventory::submit! {
    ParserEntry::new(&PATH_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;
    use std::fs;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = PathParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_basic() {
        let v = parse_to_value("/abc/def/gh.txt");
        assert_eq!(v["path"], "/abc/def/gh.txt");
        assert_eq!(v["parent"], "/abc/def");
        assert_eq!(v["filename"], "gh.txt");
        assert_eq!(v["stem"], "gh");
        assert_eq!(v["extension"], "txt");
        assert_eq!(
            v["path_list"],
            serde_json::json!(["/", "abc", "def", "gh.txt"])
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_one_fixture() {
        let out_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path--one.out"
        );
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path--one.json"
        );
        if let (Ok(input), Ok(expected_json)) =
            (fs::read_to_string(out_path), fs::read_to_string(json_path))
        {
            let v = parse_to_value(input.trim());
            let expected: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
            assert_eq!(v, expected);
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_long_fixture() {
        let out_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path--long.out"
        );
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generic/path--long.json"
        );
        if let (Ok(input), Ok(expected_json)) =
            (fs::read_to_string(out_path), fs::read_to_string(json_path))
        {
            let v = parse_to_value(input.trim());
            let expected: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
            assert_eq!(v, expected);
        }
    }
}

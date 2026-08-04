//! Parser for `lsattr` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsattrParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsattr",
    argument: "--lsattr",
    version: "1.0.0",
    description: "Converts `lsattr` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["lsattr"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSATTR_PARSER: LsattrParser = LsattrParser;

inventory::submit! {
    ParserEntry::new(&LSATTR_PARSER)
}

// Attribute flag -> field name mapping
// From https://github.com/mirror/busybox/blob/2d4a3d9e6c1493a9520b907e07a41aca90cdfd94/e2fsprogs/e2fs_lib.c#L40
fn attribute_name(ch: char) -> Option<&'static str> {
    match ch {
        'B' => Some("compressed_file"),
        'Z' => Some("compressed_dirty_file"),
        'X' => Some("compression_raw_access"),
        's' => Some("secure_deletion"),
        'u' => Some("undelete"),
        'S' => Some("synchronous_updates"),
        'D' => Some("synchronous_directory_updates"),
        'i' => Some("immutable"),
        'a' => Some("append_only"),
        'd' => Some("no_dump"),
        'A' => Some("no_atime"),
        'c' => Some("compression_requested"),
        'E' => Some("encrypted"),
        'j' => Some("journaled_data"),
        'I' => Some("indexed_directory"),
        't' => Some("no_tailmerging"),
        'T' => Some("top_of_directory_hierarchies"),
        'e' => Some("extents"),
        'C' => Some("no_cow"),
        'F' => Some("casefold"),
        'N' => Some("inline_data"),
        'P' => Some("project_hierarchy"),
        'V' => Some("verity"),
        _ => None,
    }
}

impl Parser for LsattrParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            // Skip blank lines
            if line.trim().is_empty() {
                continue;
            }

            // Skip folder headers from -R output (lines ending with ':')
            if line.trim_end().ends_with(':') && !line.trim().contains(' ') {
                continue;
            }

            // Skip error lines
            if line.starts_with("lsattr:") {
                continue;
            }

            // Split once: everything after the attribute block is the
            // filename, spaces and all. Splitting on whitespace truncated
            // `./ok ok ok ok ok` to `./ok`.
            let Some((attributes, file)) = line.trim_end().split_once(' ') else {
                continue;
            };
            if attributes.is_empty() || file.is_empty() {
                continue;
            }

            let mut out = Map::new();
            out.insert("file".to_string(), Value::String(file.to_string()));

            for ch in attributes.chars() {
                if let Some(name) = attribute_name(ch) {
                    out.insert(name.to_string(), Value::Bool(true));
                }
            }

            result.push(out);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsattr_basic() {
        let input = "--------c-----e----- /tmp/folder/folder/test_file\n";
        let parser = LsattrParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("file"),
                Some(&Value::String("/tmp/folder/folder/test_file".to_string()))
            );
            assert_eq!(
                arr[0].get("compression_requested"),
                Some(&Value::Bool(true))
            );
            assert_eq!(arr[0].get("extents"), Some(&Value::Bool(true)));
            assert!(arr[0].get("immutable").is_none());
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsattr_error_line_skipped() {
        let input = "lsattr: Operation not supported While reading flags on /proc/1\n--------e----- /etc/passwd\n";
        let parser = LsattrParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsattr_empty() {
        let parser = LsattrParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

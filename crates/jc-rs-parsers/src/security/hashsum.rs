//! Parser for hashsum commands (md5sum, sha1sum, sha256sum, etc.) output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct HashsumParser;

static INFO: ParserInfo = ParserInfo {
    name: "hashsum",
    argument: "--hashsum",
    version: "1.2.0",
    description: "Converts hashsum command output to JSON (md5sum, sha1sum, sha256sum, etc.)",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &[
        "md5sum",
        "md5",
        "shasum",
        "sha1sum",
        "sha224sum",
        "sha256sum",
        "sha384sum",
        "sha512sum",
    ],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HASHSUM_PARSER: HashsumParser = HashsumParser;

inventory::submit! {
    ParserEntry::new(&HASHSUM_PARSER)
}

impl Parser for HashsumParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (file_hash, file_name) = if line.starts_with("MD5 (") {
                // Legacy macOS md5 format: "MD5 (filename) = hash"
                let parts: Vec<&str> = line.splitn(2, " = ").collect();
                if parts.len() != 2 {
                    continue;
                }
                let hash = parts[1].trim().to_string();
                // Extract filename from "MD5 (filename)"
                let name_part = parts[0].trim();
                let name = if name_part.starts_with("MD5 (") && name_part.ends_with(')') {
                    name_part[5..name_part.len() - 1].to_string()
                } else {
                    name_part.to_string()
                };
                (hash, name)
            } else {
                // Standard format: "hash  filename" or "hash filename"
                let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
                if parts.len() != 2 {
                    continue;
                }
                let hash = parts[0].to_string();
                let name = parts[1]
                    .trim_start_matches(' ')
                    .trim_start_matches('*')
                    .to_string();
                (hash, name)
            };

            let mut obj = Map::new();
            obj.insert("filename".to_string(), Value::String(file_name));
            obj.insert("hash".to_string(), Value::String(file_hash));
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashsum_standard() {
        let input = "65fc958c1add637ec23c4b137aecf3d3  devtoolset-3-gcc-4.9.2-6.el7.x86_64.rpm\n5b9312ee5aff080927753c63a347707d  digout";
        let parser = HashsumParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("hash"),
                Some(&Value::String(
                    "65fc958c1add637ec23c4b137aecf3d3".to_string()
                ))
            );
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String(
                    "devtoolset-3-gcc-4.9.2-6.el7.x86_64.rpm".to_string()
                ))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_hashsum_md5_legacy() {
        let input = "MD5 (test.txt) = d8e8fca2dc0f896fd7cb4cb0031ba249";
        let parser = HashsumParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String("test.txt".to_string()))
            );
            assert_eq!(
                arr[0].get("hash"),
                Some(&Value::String(
                    "d8e8fca2dc0f896fd7cb4cb0031ba249".to_string()
                ))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_hashsum_empty() {
        let parser = HashsumParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}

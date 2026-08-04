//! Parser for `zipinfo` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int, simple_table_parse};
use serde_json::{Map, Value};

pub struct ZipinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "zipinfo",
    argument: "--zipinfo",
    version: "1.2.0",
    description: "Converts `zipinfo` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin],
    tags: &[Tag::Command],
    magic_commands: &["zipinfo"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ZIPINFO_PARSER: ZipinfoParser = ZipinfoParser;

inventory::submit! {
    ParserEntry::new(&ZIPINFO_PARSER)
}

impl Parser for ZipinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut datalines: Vec<&str> = input.lines().collect();

        // Remove trailing "N archives were successfully processed." line
        if let Some(last) = datalines.last()
            && last.ends_with("archives were successfully processed.")
        {
            datalines.pop();
        }

        // Split into archive blocks by empty lines
        let mut archives: Vec<Vec<&str>> = Vec::new();
        let mut current: Vec<&str> = Vec::new();
        for &row in &datalines {
            if row.is_empty() {
                if !current.is_empty() {
                    archives.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(row);
            }
        }
        if !current.is_empty() {
            archives.push(current);
        }

        let mut result = Vec::new();

        for mut archive_item in archives {
            if archive_item.len() < 3 {
                continue;
            }

            // First line: "Archive:  <name>"
            let archive_line = archive_item.remove(0);
            let archive = if archive_line.starts_with("Archive:  ") {
                archive_line["Archive:  ".len()..].to_string()
            } else {
                archive_line.trim().to_string()
            };

            // Second line: "Zip file size: N bytes, number of entries: N"
            let size_line = archive_item.remove(0);
            let size_tokens: Vec<&str> = size_line.split_whitespace().collect();
            // ["Zip", "file", "size:", "N", "bytes,", ..., "entries:", "N"]
            let size_str = size_tokens.get(3).copied().unwrap_or("0");
            let size_unit = size_tokens
                .get(4)
                .copied()
                .unwrap_or("bytes")
                .trim_end_matches(',');
            let number_entries = size_tokens.last().copied().unwrap_or("0");

            // Last line: "N file(s), N bytes uncompressed, N bytes compressed:  N%"
            let summary_line = archive_item.pop().unwrap_or("");
            let summary_tokens: Vec<&str> = summary_line.split_whitespace().collect();
            // ["N", "file(s),", "N", "bytes", "uncompressed,", "N", "bytes", "compressed:", "N%"]
            let number_files = summary_tokens.first().copied().unwrap_or("0");
            let bytes_uncompressed = summary_tokens.get(2).copied().unwrap_or("0");
            let bytes_compressed = summary_tokens.get(5).copied().unwrap_or("0");
            let percent_str = summary_tokens.last().copied().unwrap_or("0%");
            let percent_compressed = percent_str.trim_end_matches('%');

            // Remaining: file entries
            // Prepend header row for simple_table_parse
            let header = "flags zipversion zipunder filesize type method date time filename";
            let mut table_lines: Vec<&str> = vec![header];
            table_lines.extend(archive_item.iter());
            let table_str = table_lines.join("\n");
            let file_rows = simple_table_parse(&table_str);

            // Convert file rows
            let files: Vec<Value> = file_rows
                .into_iter()
                .map(|row| {
                    let mut obj = Map::new();
                    for key in &[
                        "flags",
                        "zipversion",
                        "zipunder",
                        "type",
                        "method",
                        "date",
                        "time",
                        "filename",
                    ] {
                        obj.insert(
                            key.to_string(),
                            row.get(*key)
                                .cloned()
                                .unwrap_or(Value::String(String::new())),
                        );
                    }
                    // filesize -> integer
                    let filesize = row
                        .get("filesize")
                        .and_then(|v| v.as_str())
                        .and_then(convert_to_int)
                        .map(Value::from)
                        .unwrap_or(Value::Number(0.into()));
                    obj.insert("filesize".to_string(), filesize);
                    Value::Object(obj)
                })
                .collect();

            let mut archive_obj = Map::new();
            archive_obj.insert("archive".to_string(), Value::String(archive));
            archive_obj.insert(
                "size".to_string(),
                convert_to_int(size_str)
                    .map(Value::from)
                    .unwrap_or(Value::String(size_str.to_string())),
            );
            archive_obj.insert(
                "size_unit".to_string(),
                Value::String(size_unit.to_string()),
            );
            archive_obj.insert(
                "number_entries".to_string(),
                convert_to_int(number_entries)
                    .map(Value::from)
                    .unwrap_or(Value::String(number_entries.to_string())),
            );
            archive_obj.insert(
                "number_files".to_string(),
                convert_to_int(number_files)
                    .map(Value::from)
                    .unwrap_or(Value::String(number_files.to_string())),
            );
            archive_obj.insert(
                "bytes_uncompressed".to_string(),
                convert_to_int(bytes_uncompressed)
                    .map(Value::from)
                    .unwrap_or(Value::String(bytes_uncompressed.to_string())),
            );
            archive_obj.insert(
                "bytes_compressed".to_string(),
                convert_to_int(bytes_compressed)
                    .map(Value::from)
                    .unwrap_or(Value::String(bytes_compressed.to_string())),
            );
            archive_obj.insert(
                "percent_compressed".to_string(),
                convert_to_float(percent_compressed)
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::String(percent_compressed.to_string())),
            );
            archive_obj.insert("files".to_string(), Value::Array(files));

            result.push(archive_obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_files(arr: &[Map<String, Value>], expected: &[serde_json::Value]) {
        assert_eq!(arr.len(), expected.len(), "archive count mismatch");
        for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
            for field in &[
                "archive",
                "size",
                "size_unit",
                "number_entries",
                "number_files",
                "bytes_uncompressed",
                "bytes_compressed",
                "percent_compressed",
            ] {
                assert_eq!(
                    got.get(*field).unwrap_or(&Value::Null),
                    exp.get(*field).unwrap_or(&Value::Null),
                    "archive {} field '{}' mismatch",
                    i,
                    field
                );
            }
            let got_files = got
                .get("files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let exp_files = exp
                .get("files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            assert_eq!(got_files, exp_files, "archive {} file count mismatch", i);

            // Check first few files
            if let (Some(Value::Array(gf)), Some(Value::Array(ef))) =
                (got.get("files"), exp.get("files"))
            {
                for (j, (gfile, efile)) in gf.iter().zip(ef.iter()).enumerate().take(5) {
                    for fld in &[
                        "flags",
                        "zipversion",
                        "zipunder",
                        "filesize",
                        "type",
                        "method",
                        "date",
                        "time",
                        "filename",
                    ] {
                        assert_eq!(
                            gfile.get(fld).unwrap_or(&Value::Null),
                            efile.get(fld).unwrap_or(&Value::Null),
                            "archive {} file {} field '{}' mismatch",
                            i,
                            j,
                            fld
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_zipinfo_rhel8() {
        let input = include_str!("../../../../tests/fixtures/rhel-8/zipinfo.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/rhel-8/zipinfo.json"
        ))
        .unwrap();
        let parser = ZipinfoParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => check_files(&arr, &expected),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zipinfo_space_in_name() {
        let input = include_str!("../../../../tests/fixtures/rhel-8/zipinfo-space-in-name.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/rhel-8/zipinfo-space-in-name.json"
        ))
        .unwrap();
        let parser = ZipinfoParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => check_files(&arr, &expected),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zipinfo_multi() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/zipinfo-multi.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/zipinfo-multi.json"
        ))
        .unwrap();
        let parser = ZipinfoParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), expected.len(), "archive count mismatch");
                // Just check archive names and file counts
                for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                    assert_eq!(
                        got.get("archive"),
                        exp.get("archive"),
                        "archive {} name mismatch",
                        i
                    );
                    let got_files = got
                        .get("files")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    let exp_files = exp
                        .get("files")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    assert_eq!(got_files, exp_files, "archive {} file count mismatch", i);
                }
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zipinfo_empty() {
        let parser = ZipinfoParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}

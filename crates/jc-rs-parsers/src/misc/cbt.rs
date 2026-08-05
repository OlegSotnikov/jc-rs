//! Parser for `cbt` (Google Cloud Bigtable) command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub struct CbtParser;

static INFO: ParserInfo = ParserInfo {
    name: "cbt",
    argument: "--cbt",
    version: "1.0.0",
    description: "`cbt` (Google Bigtable) command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::FreeBSD,
        Platform::Aix,
    ],
    tags: &[Tag::Command],
    magic_commands: &["cbt"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CBT_PARSER: CbtParser = CbtParser;

inventory::submit! {
    ParserEntry::new(&CBT_PARSER)
}

/// Parse a cbt timestamp string like "1970/01/01-01:00:00.000000" to ISO-8601.
fn parse_cbt_timestamp(ts: &str) -> String {
    // Format: YYYY/MM/DD-HH:MM:SS.ffffff
    let ts = ts.trim();
    if ts.len() < 19 {
        return ts.to_string();
    }
    // Replace '/' with '-' and '-' separator with 'T'
    let year = &ts[0..4];
    let month = &ts[5..7];
    let day = &ts[8..10];
    let time_part = &ts[11..19]; // HH:MM:SS
    format!("{}-{}-{}T{}", year, month, day, time_part)
}

/// A raw cell record
struct RawCell {
    column_family: String,
    column: String,
    value: String,
    timestamp_iso: String,
    /// The stamp as cbt printed it (`2000/01/01-01:00:00.000000`). The epoch is
    /// computed from this, not from the ISO rendering: jc hints the parse with
    /// cbt's own format, and the ISO string does not match it.
    timestamp_raw: String,
}

/// One row: its key and every cell version the dump listed, newest last.
struct Row {
    key: String,
    cells: Vec<RawCell>,
}

/// Walk the dump. Shaping the cells is left to the caller, because raw and
/// processed output disagree about it.
fn rows(input: &str) -> Vec<Row> {
    let separator = "-".repeat(40);
    let mut result = Vec::new();

    for section in input.split(separator.as_str()) {
        if section.trim().is_empty() {
            continue;
        }

        let mut key: Option<String> = None;
        let mut cells: Vec<RawCell> = Vec::new();
        let mut column_name = String::new();
        let mut timestamp = String::new();
        let mut value_next = false;

        for line in section.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if line.starts_with("    ") {
                if value_next {
                    let value = line.trim().trim_matches('"').to_string();
                    if let Some((cf, col)) = column_name.split_once(':') {
                        cells.push(RawCell {
                            column_family: cf.to_string(),
                            column: col.to_string(),
                            value,
                            timestamp_iso: parse_cbt_timestamp(&timestamp),
                            timestamp_raw: timestamp.clone(),
                        });
                    }
                }
            } else if line.starts_with("  ") {
                if let Some(at_pos) = line.find('@') {
                    column_name = line[..at_pos].trim().to_string();
                    timestamp = line[at_pos + 1..].trim().to_string();
                    value_next = true;
                }
            } else {
                key = Some(line.trim().to_string());
                value_next = false;
            }
        }

        if let Some(key) = key {
            result.push(Row { key, cells });
        }
    }

    result
}

impl Parser for CbtParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut result = Vec::new();

        for row in rows(input) {
            // Processed output keeps only the newest version of each cell, as
            // `{column_family: {column: value}}`.
            let mut grouped: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
            for cell in row.cells {
                grouped
                    .entry((cell.column_family, cell.column))
                    .or_default()
                    .push((cell.timestamp_iso, cell.value));
            }

            let mut cells_obj: Map<String, Value> = Map::new();
            for ((cf, col), mut versions) in grouped {
                versions.sort_by(|a, b| b.0.cmp(&a.0));
                let latest = versions
                    .into_iter()
                    .next()
                    .map(|(_, v)| v)
                    .unwrap_or_default();
                let cf_entry = cells_obj
                    .entry(cf)
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(cf_map) = cf_entry {
                    cf_map.insert(col, Value::String(latest));
                }
            }

            let mut row_obj = Map::with_capacity(2);
            row_obj.insert("key".to_string(), Value::String(row.key));
            row_obj.insert("cells".to_string(), Value::Object(cells_obj));
            result.push(row_obj);
        }

        Ok(ParseOutput::Array(result))
    }

    /// Raw output keeps every version of every cell, each with its timestamp --
    /// which is the point of `-r` here: the processed form throws the history
    /// away.
    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut result = Vec::new();

        for row in rows(input) {
            let cells: Vec<Value> = row
                .cells
                .into_iter()
                .map(|cell| {
                    let ts = jc_rs_utils::parse_timestamp(
                        &cell.timestamp_raw,
                        &[
                            jc_rs_utils::timestamp::formats::F1750,
                            jc_rs_utils::timestamp::formats::F1755,
                        ],
                    );
                    let mut obj = Map::with_capacity(6);
                    obj.insert(
                        "column_family".to_string(),
                        Value::String(cell.column_family),
                    );
                    obj.insert("column".to_string(), Value::String(cell.column));
                    obj.insert("value".to_string(), Value::String(cell.value));
                    obj.insert(
                        "timestamp_iso".to_string(),
                        Value::String(cell.timestamp_iso),
                    );
                    obj.insert(
                        "timestamp_epoch".to_string(),
                        ts.naive_epoch
                            .map_or(Value::Null, |n| Value::Number(n.into())),
                    );
                    obj.insert(
                        "timestamp_epoch_utc".to_string(),
                        ts.utc_epoch
                            .map_or(Value::Null, |n| Value::Number(n.into())),
                    );
                    Value::Object(obj)
                })
                .collect();

            let mut row_obj = Map::with_capacity(2);
            row_obj.insert("key".to_string(), Value::String(row.key));
            row_obj.insert("cells".to_string(), Value::Array(cells));
            result.push(row_obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = CbtParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();

        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    serde_json::Value::Object(got.clone()),
                    *exp,
                    "mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_cbt_single() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/cbt-single.out"),
            include_str!("../../../../tests/fixtures/generic/cbt-single.json"),
        );
    }

    #[test]
    fn test_cbt_multiple_rows() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/cbt-multiple-rows.out"),
            include_str!("../../../../tests/fixtures/generic/cbt-multiple-rows.json"),
        );
    }

    #[test]
    fn test_cbt_multiple_columns() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/cbt-multiple-columns.out"),
            include_str!("../../../../tests/fixtures/generic/cbt-multiple-columns.json"),
        );
    }

    #[test]
    fn test_cbt_empty() {
        let parser = CbtParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}

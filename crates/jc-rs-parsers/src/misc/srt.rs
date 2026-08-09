use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct SrtParser;

static INFO: ParserInfo = ParserInfo {
    name: "srt",
    argument: "--srt",
    version: "1.0.0",
    description: "SRT file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SRT_PARSER: SrtParser = SrtParser;

inventory::submit! {
    ParserEntry::new(&SRT_PARSER)
}

static TS_RE: OnceLock<Regex> = OnceLock::new();
static ARROW_RE: OnceLock<Regex> = OnceLock::new();
static INDEX_RE: OnceLock<Regex> = OnceLock::new();

fn get_ts_re() -> &'static Regex {
    TS_RE.get_or_init(|| {
        Regex::new(r"^([0-9]+)[,.:，．。：]([0-9]+)[,.:，．。：]([0-9]+)[,.:，．。：]?([0-9]*)$")
            .unwrap()
    })
}

fn get_arrow_re() -> &'static Regex {
    ARROW_RE.get_or_init(|| {
        // Match: "HH:MM:SS,mmm --> HH:MM:SS,mmm" with flexible separators
        Regex::new(r"^([0-9][0-9,:，．。：.]+[0-9]) *-[ -]* *> *([0-9][0-9,:，．。：.]+[0-9])")
            .unwrap()
    })
}

fn get_index_re() -> &'static Regex {
    INDEX_RE.get_or_init(|| Regex::new(r"^-?[0-9]+\.?[0-9]*$").unwrap())
}

fn parse_timestamp(ts: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let ts = ts.trim();
    if let Some(caps) = get_ts_re().captures(ts) {
        let hours: i64 = caps.get(1).map_or("0", |m| m.as_str()).parse().unwrap_or(0);
        let minutes: i64 = caps.get(2).map_or("0", |m| m.as_str()).parse().unwrap_or(0);
        let seconds: i64 = caps.get(3).map_or("0", |m| m.as_str()).parse().unwrap_or(0);
        let ms_str = caps.get(4).map_or("0", |m| m.as_str());
        let milliseconds: i64 = if ms_str.is_empty() {
            0
        } else {
            ms_str.parse().unwrap_or(0)
        };

        obj.insert("hours".to_string(), Value::Number(hours.into()));
        obj.insert("minutes".to_string(), Value::Number(minutes.into()));
        obj.insert("seconds".to_string(), Value::Number(seconds.into()));
        obj.insert(
            "milliseconds".to_string(),
            Value::Number(milliseconds.into()),
        );
        obj.insert("timestamp".to_string(), Value::String(ts.to_string()));
    }
    obj
}

impl Parser for SrtParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        // Normalize line endings
        let input = input.replace("\r\n", "\n").replace('\r', "\n");

        // Split on double (or more) newlines to get blocks
        // But also handle blocks without blank line separation using regex lookahead
        // Simple approach: split on blank lines
        let blocks: Vec<&str> = input.split("\n\n").collect();

        for block in blocks {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            let lines: Vec<&str> = block.lines().collect();
            if lines.is_empty() {
                continue;
            }

            let mut line_idx = 0;

            // Try to parse index from first line
            let mut index_val: Option<i64> = None;
            if get_index_re().is_match(lines[line_idx])
                && let Ok(n) = lines[line_idx].trim().parse::<i64>()
            {
                index_val = Some(n);
                line_idx += 1;
            }

            if line_idx >= lines.len() {
                continue;
            }

            // Parse timestamp line
            let ts_line = lines[line_idx];
            let arrow_re = get_arrow_re();
            let caps = match arrow_re.captures(ts_line) {
                Some(c) => c,
                None => continue,
            };
            let start_ts = caps.get(1).map_or("", |m| m.as_str()).trim();
            let end_ts = caps.get(2).map_or("", |m| m.as_str()).trim();
            line_idx += 1;

            // Remaining lines are content
            let content_lines: Vec<&str> = lines[line_idx..].to_vec();
            let content = content_lines.join("\n");

            let mut obj = Map::new();
            if let Some(idx) = index_val {
                obj.insert("index".to_string(), Value::Number(idx.into()));
            }
            obj.insert(
                "start".to_string(),
                Value::Object(parse_timestamp(start_ts)),
            );
            obj.insert("end".to_string(), Value::Object(parse_timestamp(end_ts)));
            obj.insert("content".to_string(), Value::String(content));

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/srt-attack_of_the_clones.srt");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/srt-attack_of_the_clones.json"
        ))
        .unwrap();
        let parser = SrtParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }

    #[test]
    fn test_srt_complex() {
        let input = include_str!("../../../../tests/fixtures/generic/srt-complex.srt");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/srt-complex.json"
        ))
        .unwrap();
        let parser = SrtParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}

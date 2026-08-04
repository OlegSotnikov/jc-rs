//! Streaming parser for `rsync` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub struct RsyncStreamParser;

static INFO: ParserInfo = ParserInfo {
    name: "rsync_s",
    argument: "--rsync-s",
    version: "1.3.0",
    description: "Streaming parser for `rsync` command output",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Streaming],
    streaming: true,
    hidden: false,
    deprecated: false,
    magic_commands: &[],
};

static RSYNC_STREAM_PARSER: RsyncStreamParser = RsyncStreamParser;
inventory::submit! { ParserEntry::new(&RSYNC_STREAM_PARSER) }

fn parse_size_to_int(s: &str) -> Option<i64> {
    let s = s.replace(',', "");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(n) = s.strip_suffix('K') {
        (n, 1024i64)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1024i64 * 1024 * 1024 * 1024)
    } else {
        (s, 1)
    };
    num.parse::<f64>().ok().map(|f| (f * mult as f64) as i64)
}

fn parse_size_to_float(s: &str) -> Option<f64> {
    let s = s.replace(',', "");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(n) = s.strip_suffix('K') {
        (n, 1024.0f64)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1024.0 * 1024.0 * 1024.0 * 1024.0)
    } else {
        (s, 1.0)
    };
    num.parse::<f64>().ok().map(|f| f * mult)
}

fn flag_bool(c: char, true_char: char) -> Value {
    match c {
        _ if c == true_char => Value::Bool(true),
        '.' => Value::Bool(false),
        '+' | ' ' | '?' => Value::Null,
        _ => Value::Null,
    }
}

fn parse_file_meta(meta: &str, name: &str) -> Map<String, Value> {
    let update_type_map: &[(&str, Option<&str>)] = &[
        ("<", Some("file sent")),
        (">", Some("file received")),
        ("c", Some("local change or creation")),
        ("h", Some("hard link")),
        (".", Some("not updated")),
        ("*", Some("message")),
        ("+", None),
    ];

    let file_type_map: &[(&str, Option<&str>)] = &[
        ("f", Some("file")),
        ("d", Some("directory")),
        ("L", Some("symlink")),
        ("D", Some("device")),
        ("S", Some("special file")),
        ("+", None),
    ];

    let chars: Vec<char> = meta.chars().collect();
    let get = |i: usize| chars.get(i).copied().unwrap_or('+');

    let update_type = update_type_map
        .iter()
        .find(|(k, _)| meta.starts_with(k))
        .map(|(_, v)| {
            v.map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null)
        })
        .unwrap_or(Value::Null);

    let file_type = {
        let ft_char = get(1);
        file_type_map
            .iter()
            .find(|(k, _)| k.starts_with(ft_char))
            .map(|(_, v)| {
                v.map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null)
            })
            .unwrap_or(Value::Null)
    };

    let is_mac_format = meta.len() == 9;

    let mut obj = Map::new();
    obj.insert("type".to_string(), Value::String("file".to_string()));
    obj.insert("filename".to_string(), Value::String(name.to_string()));
    obj.insert("metadata".to_string(), Value::String(meta.to_string()));
    obj.insert("update_type".to_string(), update_type);
    obj.insert("file_type".to_string(), file_type);
    obj.insert(
        "checksum_or_value_different".to_string(),
        flag_bool(get(2), 'c'),
    );
    obj.insert("size_different".to_string(), flag_bool(get(3), 's'));
    obj.insert(
        "modification_time_different".to_string(),
        flag_bool(get(4), 't'),
    );
    obj.insert("permissions_different".to_string(), flag_bool(get(5), 'p'));
    obj.insert("owner_different".to_string(), flag_bool(get(6), 'o'));
    obj.insert("group_different".to_string(), flag_bool(get(7), 'g'));

    if !is_mac_format {
        obj.insert("acl_different".to_string(), flag_bool(get(9), 'a'));
        obj.insert(
            "extended_attribute_different".to_string(),
            flag_bool(get(10), 'x'),
        );
    }

    obj
}

static FILE_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][u.+ ?][a.+ ?][x.+ ?]) (.+)$",
    )
    .expect("valid file_line_re pattern")
});
static FILE_LINE_MAC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][x.+ ?]) (.+)$")
        .expect("valid file_line_mac_re pattern")
});
static FILE_LINE_LOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] ([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][u.+ ?][a.+ ?][x.+ ?]) (.+)$").expect("valid file_line_log_re pattern")
});
static FILE_LINE_LOG_MAC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] ([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][x.+ ?]) (.+)$").expect("valid file_line_log_mac_re pattern")
});
static STAT1_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"sent\s+([0-9,]+)\s+bytes\s+received\s+([0-9,]+)\s+bytes\s+([0-9,.]+)\s+bytes/sec")
        .expect("valid stat1_re pattern")
});
static STAT2_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"total size is\s+([0-9,]+)\s+speedup is\s+([0-9,.]+)")
        .expect("valid stat2_re pattern")
});
static STAT2_SIMPLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"total\s+size\s+is\s+([0-9,.TGMK]+)\s+speedup\s+is\s+([0-9,.TGMK]+)")
        .expect("valid stat2_simple pattern")
});
static STAT1_SIMPLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"sent\s+([0-9,.TGMK]+)\s+bytes\s+received\s+([0-9,.TGMK]+)\s+bytes\s+([0-9,.TGMK]+)\s+bytes/sec").expect("valid stat1_simple_re pattern")
});
static STAT_LOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] sent\s+([\d,]+)\s+bytes\s+received\s+([\d,]+)\s+bytes\s+total\s+size\s+([\d,]+)").expect("valid stat_log_re pattern")
});
static STAT1_LOG_V_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] total:\s+matches=([\d,]+)\s+hash_hits=([\d,]+)\s+false_alarms=([\d,]+)\s+data=([\d,]+)").expect("valid stat1_log_v_re pattern")
});
static STAT2_LOG_V_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] sent\s+([\d,]+)\s+bytes\s+received\s+([\d,]+)\s+bytes\s+([\d,.]+)\s+bytes/sec").expect("valid stat2_log_v_re pattern")
});
static STAT3_LOG_V_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] total\s+size\s+is\s+([\d,]+)\s+speedup\s+is\s+([\d,.]+)").expect("valid stat3_log_v_re pattern")
});

/// rsync reports each transferred file on its own line and closes with a
/// summary block whose fields arrive over several lines, so files stream out
/// immediately and the summary is emitted once at the end.
#[derive(Default)]
struct RsyncSession {
    summary: Map<String, Value>,
}

impl LineParser for RsyncSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        if line.trim().is_empty() {
            return Ok(None);
        }

        // Log file lines (longer meta format)
        if let Some(caps) = FILE_LINE_LOG_RE.captures(line) {
            let meta = caps.get(4).map_or("", |m| m.as_str());
            let name = caps.get(5).map_or("", |m| m.as_str());
            let mut file = parse_file_meta(meta, name);
            file.insert(
                "date".to_string(),
                Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
            );
            file.insert(
                "time".to_string(),
                Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
            );
            let proc_str = caps.get(3).map_or("", |m| m.as_str());
            file.insert(
                "process".to_string(),
                proc_str
                    .parse::<i64>()
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            return Ok(Some(file));
        }

        if let Some(caps) = FILE_LINE_LOG_MAC_RE.captures(line) {
            let meta = caps.get(4).map_or("", |m| m.as_str());
            let name = caps.get(5).map_or("", |m| m.as_str());
            let mut file = parse_file_meta(meta, name);
            file.insert(
                "date".to_string(),
                Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
            );
            file.insert(
                "time".to_string(),
                Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
            );
            let proc_str = caps.get(3).map_or("", |m| m.as_str());
            file.insert(
                "process".to_string(),
                proc_str
                    .parse::<i64>()
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            return Ok(Some(file));
        }

        // Non-log file lines
        if let Some(caps) = FILE_LINE_RE.captures(line) {
            let meta = caps.get(1).map_or("", |m| m.as_str());
            let name = caps.get(2).map_or("", |m| m.as_str());
            return Ok(Some(parse_file_meta(meta, name)));
        }

        if let Some(caps) = FILE_LINE_MAC_RE.captures(line) {
            let meta = caps.get(1).map_or("", |m| m.as_str());
            let name = caps.get(2).map_or("", |m| m.as_str());
            return Ok(Some(parse_file_meta(meta, name)));
        }

        // Summary lines
        if let Some(caps) = STAT1_RE.captures(line) {
            self.summary
                .insert("type".to_string(), Value::String("summary".to_string()));
            self.summary.insert(
                "sent".to_string(),
                parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "received".to_string(),
                parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "bytes_sec".to_string(),
                parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        if let Some(caps) = STAT2_RE.captures(line) {
            self.summary.insert(
                "total_size".to_string(),
                parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "speedup".to_string(),
                parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        if let Some(caps) = STAT1_SIMPLE_RE.captures(line) {
            self.summary
                .insert("type".to_string(), Value::String("summary".to_string()));
            self.summary.insert(
                "sent".to_string(),
                parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "received".to_string(),
                parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "bytes_sec".to_string(),
                parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        if let Some(caps) = STAT2_SIMPLE_RE.captures(line) {
            self.summary.insert(
                "total_size".to_string(),
                parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "speedup".to_string(),
                parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }

        // Log format summaries
        if let Some(caps) = STAT_LOG_RE.captures(line) {
            self.summary
                .insert("type".to_string(), Value::String("summary".to_string()));
            self.summary.insert(
                "date".to_string(),
                Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
            );
            self.summary.insert(
                "time".to_string(),
                Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
            );
            self.summary.insert(
                "process".to_string(),
                caps.get(3)
                    .and_then(|m| m.as_str().parse::<i64>().ok())
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "sent".to_string(),
                parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "received".to_string(),
                parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "total_size".to_string(),
                parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }

        if let Some(caps) = STAT1_LOG_V_RE.captures(line) {
            self.summary
                .insert("type".to_string(), Value::String("summary".to_string()));
            self.summary.insert(
                "date".to_string(),
                Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
            );
            self.summary.insert(
                "time".to_string(),
                Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
            );
            self.summary.insert(
                "process".to_string(),
                caps.get(3)
                    .and_then(|m| m.as_str().parse::<i64>().ok())
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "matches".to_string(),
                parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "hash_hits".to_string(),
                parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "false_alarms".to_string(),
                parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "data".to_string(),
                parse_size_to_int(caps.get(7).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        if let Some(caps) = STAT2_LOG_V_RE.captures(line) {
            self.summary.insert(
                "sent".to_string(),
                parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "received".to_string(),
                parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "bytes_sec".to_string(),
                parse_size_to_float(caps.get(6).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        if let Some(caps) = STAT3_LOG_V_RE.captures(line) {
            self.summary.insert(
                "total_size".to_string(),
                parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            self.summary.insert(
                "speedup".to_string(),
                parse_size_to_float(caps.get(5).map_or("", |m| m.as_str()))
                    .and_then(|f| serde_json::Number::from_f64(f))
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Ok(None);
        }
        Ok(None)
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if self.summary.is_empty() {
            return Ok(None);
        }
        Ok(Some(std::mem::take(&mut self.summary)))
    }
}

impl Parser for RsyncStreamParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for RsyncStreamParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(RsyncSession::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};

    #[test]
    fn test_rsync_s_i_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/rsync-i.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/rsync-i-streaming.json"
        ))
        .unwrap();
        let result = RsyncStreamParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_rsync_s_empty() {
        let result = RsyncStreamParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_rsync_s_registered() {
        assert!(jc_rs_core::registry::find_parser("rsync_s").is_some());
    }
}

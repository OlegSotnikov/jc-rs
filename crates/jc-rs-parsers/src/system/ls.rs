//! Parser for `ls` command output.

use chrono::DateTime;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsParser;

static INFO: ParserInfo = ParserInfo {
    name: "ls",
    argument: "--ls",
    version: "1.12.0",
    description: "Converts `ls` and `vdir` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Aix,
    ],
    tags: &[Tag::Command],
    magic_commands: &["ls", "vdir"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LS_PARSER: LsParser = LsParser;

inventory::submit! {
    ParserEntry::new(&LS_PARSER)
}

/// Returns true if this line looks like an `ls -l` entry (starts with file type + permissions)
fn is_long_entry(line: &str) -> bool {
    // File type chars: - d c l p s b D C M n P ?
    // Followed by permission triplets
    let re_chars = "-dclpsbDCMnP?";
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0] as char;
    if !re_chars.contains(first) {
        return false;
    }
    // Check for permission pattern: at least 9 chars of rwx-
    if bytes.len() < 10 {
        return false;
    }
    bytes[1..10].iter().all(|&b| {
        let c = b as char;
        matches!(c, 'r' | 'w' | 'x' | '-' | 's' | 'S' | 't' | 'T' | '+')
    })
}

/// Returns true if the date looks like a standard ls date (starts with month abbreviation)
/// Standard: "Oct 21 13:18" or "Jan  1  2019"
/// ISO: "2018-01-18 01:43:49.000000000 -0800"
fn is_standard_ls_date(date_str: &str) -> bool {
    date_str
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic())
        .unwrap_or(true)
}

/// Parse an ISO datetime string to a Unix epoch timestamp.
/// Handles: "2018-01-18 01:43:49.000000000 -0800"
fn parse_iso_epoch(date_str: &str) -> Option<i64> {
    let s = date_str.trim();
    // Try with fractional seconds
    DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z")
        .ok()
        .map(|dt| dt.timestamp())
        .or_else(|| {
            DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z")
                .ok()
                .map(|dt| dt.timestamp())
        })
        .or_else(|| {
            DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
                .ok()
                .map(|dt| dt.timestamp())
        })
}

/// Convert a human-readable size string (e.g. "4.0K", "3.2K") to an integer,
/// matching jc's convert_to_int: strip non-numeric chars (except . and -), parse as float, truncate.
fn convert_size_to_int(s: &str) -> Option<i64> {
    // First try direct integer parse
    if let Ok(n) = s.trim().parse::<i64>() {
        return Some(n);
    }
    // Strip non-numeric chars (except . and -)
    let stripped: String = s
        .chars()
        .filter(|&c| c.is_ascii_digit() || c == '.' || c == '-')
        .collect();
    if stripped.is_empty() {
        return None;
    }
    // Try as float, then truncate to int
    stripped.parse::<f64>().ok().map(|f| f as i64)
}

impl Parser for LsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut lines: Vec<&str> = input.lines().collect();

        if lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Remove leading 'total N' line
        if lines
            .first()
            .map(|l| l.starts_with("total "))
            .unwrap_or(false)
        {
            lines.remove(0);
        }

        if lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Check if first line is a parent directory (ends with ':' and not a long entry)
        let mut parent = String::new();
        if !is_long_entry(lines[0]) && lines[0].ends_with(':') {
            parent = lines.remove(0).trim_end_matches(':').to_string();
            // Remove following 'total N' line if present
            if lines
                .first()
                .map(|l| l.starts_with("total "))
                .unwrap_or(false)
            {
                lines.remove(0);
            }
        }

        if lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Determine if long format (-l) by checking if first non-empty line is a long entry
        let is_long = lines
            .iter()
            .find(|l| !l.trim().is_empty())
            .map(|l| is_long_entry(l))
            .unwrap_or(false);

        if is_long {
            parse_long_format(&lines, &parent, &mut raw_output);
        } else {
            parse_short_format(&lines, &parent, &mut raw_output);
        }

        // Process: convert integer fields, add epoch for ISO dates
        let processed = process(raw_output);
        Ok(ParseOutput::Array(processed))
    }
}

fn parse_long_format(lines: &[&str], initial_parent: &str, output: &mut Vec<Map<String, Value>>) {
    let mut parent = initial_parent.to_string();
    let mut new_section = false;

    for line in lines {
        // total N line resets new_section
        if line.starts_with("total ") {
            new_section = false;
            continue;
        }

        // Directory header line for -R output
        if !is_long_entry(line) && !line.trim().is_empty() && line.trim_end().ends_with(':') {
            // fixup: always remove last character from previous entry (matches jc behavior).
            // jc unconditionally does raw_output[-1]['filename'] = raw_output[-1]['filename'][:-1]
            // which removes the '\n' added by the continuation logic, but also removes a real
            // character if called again (e.g. for empty dirs that have two consecutive headers).
            if let Some(last) = output.last_mut()
                && let Some(Value::String(s)) = last.get_mut("filename")
            {
                s.pop();
            }
            parent = line.trim_end_matches(':').to_string();
            new_section = true;
            continue;
        }

        // Fix for OSX - empty dir in -R doesn't print 'total' line
        if new_section && line.trim().is_empty() {
            new_section = false;
            continue;
        }

        // Continuation line for filenames with newlines (including blank lines)
        if !new_section && !is_long_entry(line) {
            if let Some(last) = output.last_mut()
                && let Some(Value::String(s)) = last.get_mut("filename")
            {
                s.push('\n');
                s.push_str(line);
            }
            continue;
        }

        // Skip remaining empty lines (only if new_section=false and output is empty)
        if line.trim().is_empty() {
            continue;
        }

        // Parse the long entry
        let parts = split_ls_long_line(line);

        let mut entry = Map::new();

        if parts.len() >= 9 {
            let filename_field = parts[8];
            let link_parts: Vec<&str> = filename_field.splitn(2, " -> ").collect();
            entry.insert(
                "filename".to_string(),
                Value::String(link_parts[0].to_string()),
            );
            if link_parts.len() > 1 {
                entry.insert(
                    "link_to".to_string(),
                    Value::String(link_parts[1].to_string()),
                );
            }
        } else if parts.len() >= 5 {
            // Minimal long entry
            entry.insert("filename".to_string(), Value::String(String::new()));
        } else {
            continue;
        }

        if !parent.is_empty() {
            entry.insert("parent".to_string(), Value::String(parent.clone()));
        }

        if !parts.is_empty() {
            entry.insert("flags".to_string(), Value::String(parts[0].to_string()));
        }
        if parts.len() > 1 {
            entry.insert("links".to_string(), Value::String(parts[1].to_string()));
        }
        if parts.len() > 2 {
            entry.insert("owner".to_string(), Value::String(parts[2].to_string()));
        }
        if parts.len() > 3 {
            entry.insert("group".to_string(), Value::String(parts[3].to_string()));
        }
        if parts.len() > 4 {
            entry.insert("size".to_string(), Value::String(parts[4].to_string()));
        }
        if parts.len() > 7 {
            let date = format!("{} {} {}", parts[5], parts[6], parts[7]);
            entry.insert("date".to_string(), Value::String(date));
        }

        output.push(entry);
        new_section = false;
    }
}

/// Split an ls -l line into at most 9 fields (last field is filename, may contain spaces)
fn split_ls_long_line(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut remaining = line.trim_start();

    for i in 0..9 {
        if remaining.is_empty() {
            break;
        }
        if i == 8 {
            parts.push(remaining);
            break;
        }
        // Find end of current token
        let token_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        parts.push(&remaining[..token_end]);
        remaining = remaining[token_end..].trim_start();
    }

    parts
}

fn parse_short_format(lines: &[&str], initial_parent: &str, output: &mut Vec<Map<String, Value>>) {
    let mut parent = initial_parent.to_string();
    let mut next_is_parent = false;

    for line in lines {
        if line.trim().is_empty() {
            next_is_parent = true;
            continue;
        }

        if next_is_parent && line.ends_with(':') {
            parent = line.trim_end_matches(':').to_string();
            next_is_parent = false;
            continue;
        }

        next_is_parent = false;

        let mut entry = Map::new();
        entry.insert("filename".to_string(), Value::String(line.to_string()));

        if !parent.is_empty() {
            entry.insert("parent".to_string(), Value::String(parent.clone()));
        }

        output.push(entry);
    }
}

fn process(raw: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    raw.into_iter()
        .map(|mut row| {
            // Convert links to integer
            if let Some(Value::String(s)) = row.get("links") {
                let s = s.clone();
                if let Ok(n) = s.trim().parse::<i64>() {
                    row.insert("links".to_string(), Value::Number(n.into()));
                }
            }
            // Convert size: handle both plain integers and human-readable (e.g. "4.0K")
            if let Some(Value::String(s)) = row.get("size") {
                let s = s.clone();
                if let Some(n) = convert_size_to_int(s.trim()) {
                    row.insert("size".to_string(), Value::Number(n.into()));
                }
            }
            // Add epoch/epoch_utc for ISO format dates
            if let Some(Value::String(date_str)) = row.get("date") {
                let date_str = date_str.clone();
                if !is_standard_ls_date(&date_str) {
                    let epoch = parse_iso_epoch(&date_str)
                        .map(Value::from)
                        .unwrap_or(Value::Null);
                    row.insert("epoch".to_string(), epoch);
                    row.insert("epoch_utc".to_string(), Value::Null);
                }
            }
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_short() {
        let input = "bin\nboot\ndev\netc\nhome\n";
        let parser = LsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 5);
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String("bin".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ls_long() {
        let input = "total 20\ndr-xr-xr-x.  17 root root  224 Aug 15 10:56 .\ndr-xr-xr-x.  17 root root  224 Aug 15 10:56 ..\nlrwxrwxrwx.   1 root root    7 Aug 15 10:53 bin -> usr/bin\n";
        let parser = LsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String(".".to_string()))
            );
            assert_eq!(
                arr[0].get("flags"),
                Some(&Value::String("dr-xr-xr-x.".to_string()))
            );
            assert_eq!(arr[0].get("links"), Some(&Value::Number(17.into())));
            assert_eq!(
                arr[0].get("owner"),
                Some(&Value::String("root".to_string()))
            );
            assert_eq!(arr[0].get("size"), Some(&Value::Number(224.into())));
            assert_eq!(
                arr[2].get("link_to"),
                Some(&Value::String("usr/bin".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ls_empty() {
        let parser = LsParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ls_long_with_parent() {
        let input = "/usr/bin:\ntotal 4\n-rwxr-xr-x. 1 root root 62744 Aug  8 16:14 ar\n";
        let parser = LsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("parent"),
                Some(&Value::String("/usr/bin".to_string()))
            );
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String("ar".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ls_size_human_readable() {
        assert_eq!(convert_size_to_int("4.0K"), Some(4));
        assert_eq!(convert_size_to_int("3.2K"), Some(3));
        assert_eq!(convert_size_to_int("224"), Some(224));
        assert_eq!(convert_size_to_int("0"), Some(0));
    }
}

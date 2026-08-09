//! Parser for HTTP request/response headers.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct HttpHeadersParser;

static INFO: ParserInfo = ParserInfo {
    name: "http_headers",
    argument: "--http-headers",
    version: "1.1.0",
    description: "Converts HTTP request/response headers to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::Windows],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HTTP_HEADERS_PARSER: HttpHeadersParser = HttpHeadersParser;
inventory::submit! { ParserEntry::new(&HTTP_HEADERS_PARSER) }

/// Headers that should be converted to integers (jc INT_HEADERS)
const INT_HEADERS: &[&str] = &[
    "accept-ch-lifetime",
    "access-control-max-age",
    "age",
    "content-dpr",
    "content-length",
    "device-memory",
    "downlink",
    "dpr",
    "large-allocation",
    "max-forwards",
    "rtt",
    "upgrade-insecure-requests",
];

/// Headers that should be converted to floats
const FLOAT_HEADERS: &[&str] = &["x-content-duration"];

/// Headers whose values are datetime strings → add _epoch_utc sibling
const DT_HEADERS: &[&str] = &[
    "date",
    "if-modified-since",
    "if-unmodified-since",
    "last-modified",
    "memento-datetime",
];

/// Headers that are either datetimes or integers (try datetime first, then integer)
const DT_OR_INT_HEADERS: &[&str] = &["expires", "retry-after"];

/// Headers that may appear multiple times (accumulated as arrays, no comma split)
const MULTI_HEADERS: &[&str] = &[
    "content-security-policy",
    "content-security-policy-report-only",
    "cookie",
    "set-cookie",
];

/// Headers that are split by comma into arrays AND accumulated across multiple appearances
const SPLIT_AND_MULTI_HEADERS: &[&str] = &[
    "accept",
    "accept-ch",
    "accept-encoding",
    "accept-language",
    "accept-patch",
    "accept-post",
    "accept-ranges",
    "access-control-allow-headers",
    "access-control-allow-methods",
    "access-control-expose-headers",
    "access-control-request-headers",
    "allow",
    "alt-svc",
    "cache-control",
    "clear-site-data",
    "connection",
    "content-encoding",
    "content-language",
    "critical-ch",
    "expect-ct",
    "forwarded",
    "if-match",
    "if-none-match",
    "im",
    "keep-alive",
    "link",
    "permissions-policy",
    "permissions-policy-report-only",
    "pragma",
    "proxy-authenticate",
    "reporting-endpoints",
    "sec-ch-ua",
    "sec-ch-ua-full-version-list",
    "server",
    "server-timing",
    "timing-allow-origin",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "vary",
    "via",
    "warning",
    "www-authenticate",
    "x-cache-hits",
];

const HTTP_METHODS: &[&str] = &[
    "connect", "delete", "get", "head", "options", "patch", "post", "put", "trace",
];

fn http_date_to_epoch_utc(s: &str) -> Option<i64> {
    parse_rfc1123(s)
}

fn parse_rfc1123(s: &str) -> Option<i64> {
    let s = if let Some(pos) = s.find(", ") {
        &s[pos + 2..]
    } else {
        s
    };
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let day: i64 = parts[0].parse().ok()?;
    let month = month_to_num(parts[1])?;
    let year: i64 = parts[2].parse().ok()?;
    let time_parts: Vec<&str> = parts[3].splitn(3, ':').collect();
    if time_parts.len() < 3 {
        return None;
    }
    let hour: i64 = time_parts[0].parse().ok()?;
    let min: i64 = time_parts[1].parse().ok()?;
    let sec: i64 = time_parts[2].parse().ok()?;
    let days = days_since_epoch(year, month, day)?;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn month_to_num(m: &str) -> Option<i64> {
    match m.to_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn days_since_epoch(year: i64, month: i64, day: i64) -> Option<i64> {
    if year < 1970 {
        return None;
    }
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_month(year, m);
    }
    days += day - 1;
    Some(days)
}

/// Parse a single HTTP message (request or response) block.
/// Header names are kept in lowercase kebab-case (no underscore conversion), matching jc.
fn parse_message(lines: &[&str]) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut iter = lines.iter().peekable();

    let first = match iter.next() {
        Some(l) => l.trim(),
        None => return obj,
    };
    if first.is_empty() {
        return obj;
    }

    let first_word = first.split_whitespace().next().unwrap_or("").to_lowercase();

    if first_word.starts_with("http/") {
        // Response: "HTTP/1.1 200 OK"
        let mut parts = first.splitn(3, ' ');
        let version = parts.next().unwrap_or("").to_string();
        let status_str = parts.next().unwrap_or("");
        let reason = parts.next().unwrap_or("").to_string();
        let status: i64 = status_str.parse().unwrap_or(0);

        obj.insert("_type".to_string(), Value::String("response".to_string()));
        obj.insert("_response_version".to_string(), Value::String(version));
        obj.insert("_response_status".to_string(), Value::Number(status.into()));
        if reason.is_empty() {
            obj.insert("_response_reason".to_string(), Value::Null);
        } else {
            obj.insert(
                "_response_reason".to_string(),
                Value::Array(vec![Value::String(reason)]),
            );
        }
    } else if HTTP_METHODS.contains(&first_word.as_str()) {
        // Request: "HEAD / HTTP/1.1"
        let mut parts = first.splitn(3, ' ');
        let method = parts.next().unwrap_or("").to_string();
        let uri = parts.next().unwrap_or("").to_string();
        let version = parts.next().unwrap_or("").to_string();

        obj.insert("_type".to_string(), Value::String("request".to_string()));
        obj.insert("_request_method".to_string(), Value::String(method));
        obj.insert("_request_uri".to_string(), Value::String(uri));
        obj.insert("_request_version".to_string(), Value::String(version));
    }

    // Parse header lines
    let mut raw_headers: Vec<(String, String)> = Vec::new();
    for &line in iter {
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = raw_headers.last_mut() {
                last.1.push(' ');
                last.1.push_str(line.trim());
            }
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();
            raw_headers.push((key, value));
        }
    }

    // Insert headers
    for (key, value) in &raw_headers {
        if SPLIT_AND_MULTI_HEADERS.contains(&key.as_str()) {
            let parts: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            match obj.get_mut(key) {
                Some(Value::Array(arr)) => {
                    for p in parts {
                        arr.push(Value::String(p));
                    }
                }
                _ => {
                    obj.insert(
                        key.clone(),
                        Value::Array(parts.into_iter().map(Value::String).collect()),
                    );
                }
            }
        } else if MULTI_HEADERS.contains(&key.as_str()) {
            match obj.get_mut(key) {
                Some(Value::Array(arr)) => {
                    arr.push(Value::String(value.clone()));
                }
                _ => {
                    obj.insert(
                        key.clone(),
                        Value::Array(vec![Value::String(value.clone())]),
                    );
                }
            }
        } else {
            obj.insert(key.clone(), Value::String(value.clone()));
        }
    }

    // Post-process type conversions
    let keys: Vec<String> = obj.keys().cloned().collect();
    for key in &keys {
        if key.starts_with('_') {
            continue;
        }
        if INT_HEADERS.contains(&key.as_str()) {
            if let Some(Value::String(s)) = obj.get(key)
                && let Ok(n) = s.trim().parse::<i64>()
            {
                obj.insert(key.clone(), Value::Number(n.into()));
            }
        } else if FLOAT_HEADERS.contains(&key.as_str()) {
            if let Some(Value::String(s)) = obj.get(key)
                && let Ok(f) = s.trim().parse::<f64>()
                && let Some(n) = serde_json::Number::from_f64(f)
            {
                obj.insert(key.clone(), Value::Number(n));
            }
        } else if DT_HEADERS.contains(&key.as_str()) {
            if let Some(Value::String(s)) = obj.get(key) {
                let s = s.clone();
                if let Some(epoch) = http_date_to_epoch_utc(&s) {
                    obj.insert(format!("{}_epoch_utc", key), Value::Number(epoch.into()));
                }
            }
        } else if DT_OR_INT_HEADERS.contains(&key.as_str())
            && let Some(Value::String(s)) = obj.get(key)
        {
            let s = s.clone();
            if let Some(epoch) = http_date_to_epoch_utc(&s) {
                obj.insert(format!("{}_epoch_utc", key), Value::Number(epoch.into()));
            } else if s.trim().chars().all(|c| c.is_ascii_digit())
                && let Ok(n) = s.trim().parse::<i64>()
            {
                obj.insert(key.clone(), Value::Number(n.into()));
            }
        }
    }

    // x-cache-hits: convert array string elements to integers
    if let Some(Value::Array(arr)) = obj.get_mut("x-cache-hits") {
        for v in arr.iter_mut() {
            if let Value::String(s) = v
                && let Ok(n) = s.trim().parse::<i64>()
            {
                *v = Value::Number(n.into());
            }
        }
    }

    obj
}

impl Parser for HttpHeadersParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let lines: Vec<&str> = input.lines().collect();
        let mut result: Vec<Map<String, Value>> = Vec::new();
        let mut start = 0;
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();
            if i > start
                && !trimmed.is_empty()
                && (is_response_line(trimmed) || is_request_line(trimmed))
            {
                let block = &lines[start..i];
                if block.iter().any(|l| !l.trim().is_empty()) {
                    result.push(parse_message(block));
                }
                start = i;
            }
            i += 1;
        }

        if start < lines.len() {
            let block = &lines[start..];
            if block.iter().any(|l| !l.trim().is_empty()) {
                result.push(parse_message(block));
            }
        }

        Ok(ParseOutput::Array(result))
    }
}

fn is_response_line(s: &str) -> bool {
    s.to_lowercase().starts_with("http/")
}

fn is_request_line(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return false;
    }
    let method = parts[0].to_lowercase();
    HTTP_METHODS.contains(&method.as_str()) && parts[2].to_lowercase().starts_with("http/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_http_headers_empty() {
        let result = HttpHeadersParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_http_headers_registered() {
        assert!(jc_rs_core::registry::find_parser("http_headers").is_some());
    }
}

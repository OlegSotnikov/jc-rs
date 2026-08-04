//! URL string parser — parses a URL into its component parts.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde_json::{Map, Value};
use std::collections::HashMap;

struct UrlParser;

static URL_INFO: ParserInfo = ParserInfo {
    name: "url",
    argument: "--url",
    version: "1.0.0",
    description: "URL string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Encode a string using percent-encoding, keeping certain safe chars
fn url_encode(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Decode a percent-encoded string
fn url_decode(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().to_string()
}

/// Parse query string into a map of key -> Vec<value>
fn parse_query_string(query: &str) -> Map<String, Value> {
    let mut map: HashMap<String, Vec<Value>> = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, val) = if let Some(pos) = pair.find('=') {
            (&pair[..pos], &pair[pos + 1..])
        } else {
            (pair, "")
        };
        let key = url_decode(key);
        let val = url_decode(val);
        map.entry(key).or_default().push(Value::String(val));
    }
    let mut result = Map::new();
    for (k, v) in map {
        result.insert(k, Value::Array(v));
    }
    result
}

/// Unwrap URL wrappers like `URL:...`, `<...>`, `<URL:...>`
fn unwrap_url(s: &str) -> &str {
    let s = s.trim();
    // strip angle brackets
    let s = if s.starts_with('<') && s.ends_with('>') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    // strip URL: prefix (case insensitive)
    let s = if s.len() >= 4 && s[..4].eq_ignore_ascii_case("url:") {
        &s[4..]
    } else {
        s
    };
    s
}

fn build_url_map(parsed: &url::Url, original: &str) -> Map<String, Value> {
    let scheme = if parsed.scheme().is_empty() {
        Value::Null
    } else {
        Value::String(parsed.scheme().to_string())
    };

    let netloc = match parsed.host_str() {
        Some(h) => {
            let mut netloc = h.to_string();
            if let Some(port) = parsed.port() {
                netloc.push(':');
                netloc.push_str(&port.to_string());
            }
            let username = parsed.username();
            if !username.is_empty() {
                let mut full = username.to_string();
                if let Some(pass) = parsed.password() {
                    full.push(':');
                    full.push_str(pass);
                }
                full.push('@');
                full.push_str(&netloc);
                Value::String(full)
            } else {
                Value::String(netloc)
            }
        }
        None => Value::Null,
    };

    let path_str = parsed.path();
    // normalize duplicate slashes
    let path_normalized = {
        let mut p = String::new();
        let mut prev_slash = false;
        for ch in path_str.chars() {
            if ch == '/' {
                if !prev_slash {
                    p.push(ch);
                }
                prev_slash = true;
            } else {
                p.push(ch);
                prev_slash = false;
            }
        }
        p
    };

    let (path_val, parent_val, filename_val, stem_val, extension_val, path_list_val) =
        if path_normalized.is_empty() {
            (
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            )
        } else {
            let p = std::path::Path::new(&path_normalized);
            let parent = p
                .parent()
                .map(|pp| pp.to_string_lossy().to_string())
                .unwrap_or_default();
            let parent = if parent.is_empty() {
                "/".to_string()
            } else {
                parent
            };
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

            // path_list: remove leading '/' then split
            let pl: Vec<Value> = path_normalized
                .trim_start_matches('/')
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_string()))
                .collect();

            let path_list = if pl.is_empty() {
                Value::Null
            } else {
                Value::Array(pl)
            };
            let parent_val = if parent.is_empty() {
                Value::Null
            } else {
                Value::String(parent)
            };
            let filename_val = if filename.is_empty() {
                Value::Null
            } else {
                Value::String(filename)
            };
            let stem_val = if stem.is_empty() {
                Value::Null
            } else {
                Value::String(stem)
            };
            let extension_val = if extension.is_empty() {
                Value::Null
            } else {
                Value::String(extension)
            };

            (
                Value::String(path_normalized.clone()),
                parent_val,
                filename_val,
                stem_val,
                extension_val,
                path_list,
            )
        };

    let query_str = parsed.query();
    let query_val = match query_str {
        Some(q) if !q.is_empty() => Value::String(q.to_string()),
        _ => Value::Null,
    };

    let query_obj_val = match query_str {
        Some(q) if !q.is_empty() => Value::Object(parse_query_string(q)),
        _ => Value::Null,
    };

    let fragment_val = match parsed.fragment() {
        Some(f) if !f.is_empty() => Value::String(f.to_string()),
        _ => Value::Null,
    };

    let username_val = {
        let u = parsed.username();
        if u.is_empty() {
            Value::Null
        } else {
            Value::String(u.to_string())
        }
    };

    let password_val = match parsed.password() {
        Some(p) => Value::String(p.to_string()),
        None => Value::Null,
    };

    let hostname_val = match parsed.host_str() {
        Some(h) => Value::String(h.to_string()),
        None => Value::Null,
    };

    let port_val = match parsed.port() {
        Some(p) => Value::Number(p.into()),
        None => Value::Null,
    };

    // Build encoded sub-object
    let encoded_url = url_encode(original);
    let encoded_path = if path_normalized.is_empty() {
        Value::Null
    } else {
        // encode each path component separately
        let encoded: String = path_normalized
            .split('/')
            .map(|seg| {
                if seg.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", utf8_percent_encode(seg, NON_ALPHANUMERIC))
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .trim_start_matches('/')
            .to_string();
        let encoded = format!("/{}", encoded.trim_start_matches('/'));
        Value::String(encoded)
    };

    let encoded_query = match query_str {
        Some(q) if !q.is_empty() => Value::String(url_encode(q)),
        _ => Value::Null,
    };
    let encoded_fragment = match parsed.fragment() {
        Some(f) if !f.is_empty() => Value::String(url_encode(f)),
        _ => Value::Null,
    };

    let mut encoded_map = Map::new();
    encoded_map.insert("url".to_string(), Value::String(encoded_url));
    encoded_map.insert("scheme".to_string(), scheme.clone());
    encoded_map.insert("netloc".to_string(), netloc.clone());
    encoded_map.insert("path".to_string(), encoded_path);
    encoded_map.insert("parent".to_string(), parent_val.clone());
    encoded_map.insert("filename".to_string(), filename_val.clone());
    encoded_map.insert("stem".to_string(), stem_val.clone());
    encoded_map.insert("extension".to_string(), extension_val.clone());
    encoded_map.insert("path_list".to_string(), path_list_val.clone());
    encoded_map.insert("query".to_string(), encoded_query);
    encoded_map.insert("fragment".to_string(), encoded_fragment);
    encoded_map.insert("username".to_string(), username_val.clone());
    encoded_map.insert("password".to_string(), password_val.clone());
    encoded_map.insert("hostname".to_string(), hostname_val.clone());
    encoded_map.insert("port".to_string(), port_val.clone());

    // Build decoded sub-object
    let decoded_url = url_decode(original);
    let decoded_path = if path_normalized.is_empty() {
        Value::Null
    } else {
        Value::String(url_decode(&path_normalized))
    };
    let decoded_query = match query_str {
        Some(q) if !q.is_empty() => Value::String(url_decode(q)),
        _ => Value::Null,
    };
    let decoded_fragment = match parsed.fragment() {
        Some(f) if !f.is_empty() => Value::String(url_decode(f)),
        _ => Value::Null,
    };

    let mut decoded_map = Map::new();
    decoded_map.insert("url".to_string(), Value::String(decoded_url));
    decoded_map.insert("scheme".to_string(), scheme.clone());
    decoded_map.insert("netloc".to_string(), netloc.clone());
    decoded_map.insert("path".to_string(), decoded_path);
    decoded_map.insert("parent".to_string(), parent_val.clone());
    decoded_map.insert("filename".to_string(), filename_val.clone());
    decoded_map.insert("stem".to_string(), stem_val.clone());
    decoded_map.insert("extension".to_string(), extension_val.clone());
    decoded_map.insert("path_list".to_string(), path_list_val.clone());
    decoded_map.insert("query".to_string(), decoded_query);
    decoded_map.insert("fragment".to_string(), decoded_fragment);
    decoded_map.insert("username".to_string(), username_val.clone());
    decoded_map.insert("password".to_string(), password_val.clone());
    decoded_map.insert("hostname".to_string(), hostname_val.clone());
    decoded_map.insert("port".to_string(), port_val.clone());

    let mut map = Map::new();
    map.insert("url".to_string(), Value::String(original.to_string()));
    map.insert("scheme".to_string(), scheme);
    map.insert("netloc".to_string(), netloc);
    map.insert("path".to_string(), path_val);
    map.insert("parent".to_string(), parent_val);
    map.insert("filename".to_string(), filename_val);
    map.insert("stem".to_string(), stem_val);
    map.insert("extension".to_string(), extension_val);
    map.insert("path_list".to_string(), path_list_val);
    map.insert("query".to_string(), query_val);
    map.insert("query_obj".to_string(), query_obj_val);
    map.insert("fragment".to_string(), fragment_val);
    map.insert("username".to_string(), username_val);
    map.insert("password".to_string(), password_val);
    map.insert("hostname".to_string(), hostname_val);
    map.insert("port".to_string(), port_val);
    map.insert("encoded".to_string(), Value::Object(encoded_map));
    map.insert("decoded".to_string(), Value::Object(decoded_map));

    map
}

impl Parser for UrlParser {
    fn info(&self) -> &'static ParserInfo {
        &URL_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let unwrapped = unwrap_url(input);

        // Try parsing with the url crate
        let parsed = url::Url::parse(unwrapped)
            .map_err(|e| ParseError::InvalidInput(format!("invalid URL: {}", e)))?;

        let map = build_url_map(&parsed, unwrapped);
        Ok(ParseOutput::Object(map))
    }
}

static URL_PARSER_INSTANCE: UrlParser = UrlParser;

inventory::submit! {
    ParserEntry::new(&URL_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = UrlParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_url_basic() {
        let v = parse_to_value("http://example.com/test/path?q1=foo&q1=bar&q2=baz#frag");
        assert_eq!(v["scheme"], "http");
        assert_eq!(v["hostname"], "example.com");
        assert_eq!(v["path"], "/test/path");
        assert_eq!(v["query"], "q1=foo&q1=bar&q2=baz");
        assert_eq!(v["fragment"], "frag");
        assert!(v["port"].is_null());
    }

    #[test]
    fn test_url_with_port() {
        let v = parse_to_value("https://user:pass@example.com:8080/path/to?query=1#frag");
        assert_eq!(v["scheme"], "https");
        assert_eq!(v["hostname"], "example.com");
        assert_eq!(v["port"], 8080);
        assert_eq!(v["username"], "user");
        assert_eq!(v["password"], "pass");
    }

    #[test]
    fn test_url_ftp() {
        let v = parse_to_value("ftp://localhost/filepath");
        assert_eq!(v["scheme"], "ftp");
        assert_eq!(v["hostname"], "localhost");
        assert_eq!(v["path"], "/filepath");
        assert!(v["query"].is_null());
        assert!(v["fragment"].is_null());
    }
}

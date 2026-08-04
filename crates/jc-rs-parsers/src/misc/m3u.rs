use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct M3uParser;

static INFO: ParserInfo = ParserInfo {
    name: "m3u",
    argument: "--m3u",
    version: "1.0.0",
    description: "M3U and M3U8 file parser",
    author: "jc-rs contributors",
    author_email: "",
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

static M3U_PARSER: M3uParser = M3uParser;

inventory::submit! {
    ParserEntry::new(&M3U_PARSER)
}

/// Parse EXTINF line fields using shlex-like splitting with comma/space as whitespace.
/// Returns (runtime_str, display_str, extra_kv_pairs)
fn parse_extinf(extinf_value: &str) -> Option<(String, String, Vec<(String, String)>)> {
    // extinf_value is everything after "EXTINF:"
    // Format: runtime[,attributes]* display_title
    // or: runtime kv=val kv2=val2,display_title
    // The Python uses shlex with whitespace=', ' (comma and space)

    // We'll do a simple parse:
    // First token (split on comma or space) is runtime
    // Subsequent tokens that contain '=' are key=value pairs
    // Remaining tokens (joined) are the display
    let tokens = shlex_split(extinf_value)?;
    let mut iter = tokens.into_iter();
    let runtime = iter.next()?;
    let mut extra_kv = Vec::new();
    let mut display_parts = Vec::new();

    for token in iter {
        if let Some(eq_pos) = token.find('=') {
            let k = token[..eq_pos].to_string();
            let v = token[eq_pos + 1..].to_string();
            extra_kv.push((k, v));
        } else {
            display_parts.push(token);
        }
    }

    let display = display_parts.join(" ");
    Some((runtime, display, extra_kv))
}

/// Simple shlex-like split using comma and space as delimiters, respecting
/// double-quotes.
///
/// `#` starts a comment and ends the line, because that is what `shlex` does in
/// POSIX mode and jc passes it a default `shlex`. It reads as a bug against
/// channel names like `Bumblebee TV Classics #2 (720p)` -- jc records that name
/// as `Bumblebee TV Classics` -- but jc is the schema, quirk included.
fn shlex_split(s: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in s.chars() {
        if in_quote {
            if ch == '"' {
                in_quote = false;
            } else {
                current.push(ch);
            }
        } else if ch == '#' {
            break;
        } else if ch == '"' {
            in_quote = true;
        } else if ch == ',' || ch == ' ' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Some(tokens)
}

impl Parser for M3uParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();
        let mut current_entry: Map<String, Value> = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("#EXTINF:") {
                // Parse the EXTINF line
                let extinf_value = &line["#EXTINF:".len()..];
                match parse_extinf(extinf_value) {
                    Some((runtime, display, extra_kv)) => {
                        // Convert runtime to int
                        let runtime_val = match runtime.parse::<i64>() {
                            Ok(n) => Value::Number(n.into()),
                            Err(_) => {
                                // Try converting via convert_to_int
                                match convert_to_int(&runtime) {
                                    Some(n) => Value::Number(n.into()),
                                    None => Value::String(runtime),
                                }
                            }
                        };
                        current_entry.insert("runtime".to_string(), runtime_val);
                        current_entry.insert("display".to_string(), Value::String(display));
                        for (k, v) in extra_kv {
                            current_entry.insert(k, Value::String(v));
                        }
                    }
                    None => {
                        if !quiet {
                            // Warning: can't parse extended info
                        }
                        current_entry
                            .insert("unparsed_info".to_string(), Value::String(line.to_string()));
                    }
                }
                continue;
            }

            if line.starts_with('#') {
                continue;
            }

            // This is a path line
            current_entry.insert("path".to_string(), Value::String(line.to_string()));
            result.push(current_entry.clone());
            current_entry = Map::new();
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m3u_example_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/m3u-example.m3u");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/m3u-example.json"
        ))
        .unwrap();
        let parser = M3uParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}

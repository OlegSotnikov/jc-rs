//! Streaming parser support.
//!
//! Reads stdin line by line, calls `StreamingParser::parse_line()` on each,
//! and prints each result immediately. Mirrors `JcCli::streaming_parse_and_print()`.

use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, Tag};
use serde_json::{Map, Value};
use std::io;

use crate::meta::{MetaInfo, inject_meta};
use crate::output::{ColorScheme, print_output};

/// Streaming output options.
pub struct StreamingOptions<'a> {
    pub pretty: bool,
    pub yaml: bool,
    pub use_color: bool,
    pub scheme: &'a ColorScheme,
    pub unbuffer: bool,
    pub meta_out: bool,
    pub meta_info: &'a MetaInfo,
    pub ignore_exceptions: bool,
}

/// Run a streaming parser over stdin.
///
/// Returns the number of lines successfully processed, or an error if the
/// parser failed fatally (when `ignore_exceptions` is false).
pub fn run_streaming<P: Parser + StreamingParser>(
    parser: &P,
    opts: &StreamingOptions,
    lines_iter: impl Iterator<Item = Result<String, io::Error>>,
) -> Result<u64, String> {
    let mut count: u64 = 0;

    for line_result in lines_iter {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("jc-rs: io error reading stdin: {}", e);
                break;
            }
        };

        match parser.parse_line(&line, opts.ignore_exceptions) {
            Ok(Some(output)) => {
                let mut val = parse_output_to_value(output);
                if opts.meta_out {
                    inject_meta(&mut val, opts.meta_info);
                }
                // In streaming mode with ignore_exceptions, add success=true
                if opts.ignore_exceptions {
                    add_streaming_success(&mut val, true, None);
                }
                print_output(
                    &val,
                    opts.pretty,
                    opts.yaml,
                    opts.use_color,
                    opts.scheme,
                    opts.unbuffer,
                );
                count += 1;
            }
            Ok(None) => {
                // Skip (header line, blank, etc.)
            }
            Err(e) => {
                if opts.ignore_exceptions {
                    // Emit error meta object and continue
                    let err_val = make_error_object(&e.to_string(), &line);
                    print_output(
                        &err_val,
                        opts.pretty,
                        opts.yaml,
                        opts.use_color,
                        opts.scheme,
                        opts.unbuffer,
                    );
                } else {
                    return Err(format!("Streaming parse error: {}", e));
                }
            }
        }
    }

    // Finalize
    match parser.finalize() {
        Ok(Some(output)) => {
            let mut val = parse_output_to_value(output);
            if opts.meta_out {
                inject_meta(&mut val, opts.meta_info);
            }
            print_output(
                &val,
                opts.pretty,
                opts.yaml,
                opts.use_color,
                opts.scheme,
                opts.unbuffer,
            );
        }
        Ok(None) => {}
        Err(e) => {
            if !opts.ignore_exceptions {
                return Err(format!("Streaming finalize error: {}", e));
            }
        }
    }

    Ok(count)
}

/// Build a streaming error object (mirrors the API contract).
fn make_error_object(error: &str, line: &str) -> Value {
    let mut meta = Map::new();
    meta.insert("success".to_string(), Value::Bool(false));
    meta.insert("error".to_string(), Value::String(error.to_string()));
    meta.insert("line".to_string(), Value::String(line.to_string()));

    let mut obj = Map::new();
    obj.insert("_jc_meta".to_string(), Value::Object(meta));
    Value::Object(obj)
}

/// Add `_jc_meta.success` to a streaming output value.
fn add_streaming_success(val: &mut Value, success: bool, error: Option<&str>) {
    if let Value::Object(map) = val {
        let entry = map
            .entry("_jc_meta".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(meta_map) = entry {
            meta_map.insert("success".to_string(), Value::Bool(success));
            if let Some(e) = error {
                meta_map.insert("error".to_string(), Value::String(e.to_string()));
            }
        }
    }
}

/// Convert a `ParseOutput` to a `serde_json::Value`.
pub fn parse_output_to_value(output: ParseOutput) -> Value {
    match output {
        ParseOutput::Object(obj) => Value::Object(obj),
        ParseOutput::Array(arr) => Value::Array(arr.into_iter().map(Value::Object).collect()),
    }
}

/// Returns true if the parser's info says it is a streaming parser.
pub fn parser_is_streaming(parser: &dyn Parser) -> bool {
    parser.info().has_tag(Tag::Streaming)
}

/// Returns true if the parser supports slurp.
pub fn parser_is_slurpable(parser: &dyn Parser) -> bool {
    parser.info().is_slurpable()
}

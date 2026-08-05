//! Streaming parser runtime.
//!
//! Reads input a line at a time, feeds each line to the parser's session, and
//! writes every record the moment it exists. This is the difference between
//! `tail -f access.log | jc-rs --clf-s` printing as the log grows and printing
//! nothing at all: the standard path cannot start until stdin closes, and for
//! a live pipe it never does.
//!
//! Output is NDJSON (one JSON value per line, as jc emits) rather than a
//! single array. Mirrors `JcCli::streaming_parse_and_print()`.

use jc_rs_core::traits::{Parser, StreamingParser};
use serde_json::{Map, Value};
use std::io::{self, BufWriter, StdoutLock, Write};

use crate::meta::{MetaInfo, inject_meta};
use crate::output::{ColorScheme, render_output};

/// Streaming output options.
pub struct StreamingOptions<'a> {
    pub pretty: bool,
    pub yaml: bool,
    pub use_color: bool,
    pub scheme: &'a ColorScheme,
    pub unbuffer: bool,
    pub meta_out: bool,
    pub meta_info: &'a MetaInfo,
    /// jc's `-qq`: keep going after a line that will not parse, and label every
    /// record with `_jc_meta.success`.
    pub ignore_exceptions: bool,
}

/// Writes records as they arrive.
///
/// The buffer is what keeps a 30,000-line input from costing 30,000 syscalls;
/// `-u` (unbuffer) flushes after every record instead, which is what a live
/// pipeline needs and what jc's own `flush=self.unbuffer` does.
struct RecordWriter<'a> {
    out: BufWriter<StdoutLock<'static>>,
    opts: &'a StreamingOptions<'a>,
}

impl<'a> RecordWriter<'a> {
    fn new(opts: &'a StreamingOptions<'a>) -> Self {
        Self {
            out: BufWriter::new(io::stdout().lock()),
            opts,
        }
    }

    /// Returns `false` once the reader downstream has gone away, so the caller
    /// can stop rather than parse the rest of the input for nobody.
    fn write(&mut self, value: &Value) -> bool {
        let text = render_output(
            value,
            self.opts.pretty,
            self.opts.yaml,
            self.opts.use_color,
            self.opts.scheme,
        );

        let mut result = writeln!(self.out, "{}", text);
        if result.is_ok() && self.opts.unbuffer {
            result = self.out.flush();
        }

        match result {
            Ok(()) => true,
            // `head -5` closing the pipe is a normal end, not an error.
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => false,
            Err(e) => {
                eprintln!("jc-rs: error writing output: {}", e);
                false
            }
        }
    }

    fn finish(mut self) {
        if let Err(e) = self.out.flush()
            && e.kind() != io::ErrorKind::BrokenPipe
        {
            eprintln!("jc-rs: error writing output: {}", e);
        }
    }
}

/// Run a streaming parser over an iterator of input lines.
///
/// Returns the number of records emitted, or an error when a line fails to
/// parse and `ignore_exceptions` is off.
pub fn run_streaming(
    parser: &dyn StreamingParser,
    opts: &StreamingOptions,
    lines: impl Iterator<Item = Result<String, io::Error>>,
) -> Result<u64, String> {
    let mut session = parser.session();
    let mut writer = RecordWriter::new(opts);
    let mut count: u64 = 0;

    let mut emit = |writer: &mut RecordWriter, record: Map<String, Value>| -> bool {
        let mut value = Value::Object(record);
        if opts.meta_out {
            inject_meta(&mut value, opts.meta_info);
        }
        if opts.ignore_exceptions {
            mark_success(&mut value);
        }
        count += 1;
        writer.write(&value)
    };

    for line in lines {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("jc-rs: io error reading input: {}", e);
                break;
            }
        };

        match session.parse_line(&line, opts.ignore_exceptions) {
            Ok(record) => {
                // A line usually yields one record; `take_next` covers the
                // parsers where it can close one and open another.
                for record in record
                    .into_iter()
                    .chain(std::iter::from_fn(|| session.take_next()))
                {
                    if !emit(&mut writer, record) {
                        writer.finish();
                        return Ok(count);
                    }
                }
            }
            Err(e) => {
                if !opts.ignore_exceptions {
                    writer.finish();
                    return Err(e.to_string());
                }
                if !writer.write(&error_object(&e.to_string(), &line)) {
                    writer.finish();
                    return Ok(count);
                }
            }
        }
    }

    match session.finalize(opts.ignore_exceptions) {
        Ok(Some(record)) => {
            emit(&mut writer, record);
        }
        Ok(None) => {}
        Err(e) => {
            if !opts.ignore_exceptions {
                writer.finish();
                return Err(e.to_string());
            }
        }
    }

    writer.finish();
    Ok(count)
}

/// jc's `stream_error`: the failed line is reported as a record of its own so
/// that a consumer reading NDJSON sees the gap instead of silently missing it.
fn error_object(error: &str, line: &str) -> Value {
    let mut meta = Map::with_capacity(3);
    meta.insert("success".to_string(), Value::Bool(false));
    meta.insert("error".to_string(), Value::String(error.to_string()));
    meta.insert("line".to_string(), Value::String(line.trim().to_string()));

    let mut obj = Map::with_capacity(1);
    obj.insert("_jc_meta".to_string(), Value::Object(meta));
    Value::Object(obj)
}

/// jc's `stream_success`: only present under `-qq`, where the consumer needs to
/// tell a parsed record from an error record.
fn mark_success(value: &mut Value) {
    let Value::Object(map) = value else {
        return;
    };
    let entry = map
        .entry("_jc_meta".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(meta) = entry {
        meta.insert("success".to_string(), Value::Bool(true));
    }
}

/// Returns true if the parser supports slurp.
pub fn parser_is_slurpable(parser: &dyn Parser) -> bool {
    parser.info().is_slurpable()
}

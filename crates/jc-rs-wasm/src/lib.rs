//! jc-rs compiled to WebAssembly.
//!
//! The same parsers the CLI uses, callable from JavaScript:
//!
//! ```js
//! import init, { parse, parsers } from "jc-rs-wasm";
//!
//! await init();
//! const rows = parse("df", dfOutput);      // -> JS value, not a JSON string
//! const names = parsers();                 // -> ["airport", "arp", ...]
//! ```
//!
//! Everything here is pure text in, structured data out. There is no I/O, no
//! filesystem and no process spawning, which is what makes the whole parser set
//! portable to wasm in the first place.

use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Parse `input` with the named parser and return the result as a JS value.
///
/// The name is the jc parser name in any of its spellings: `"git_log"`,
/// `"git-log"` or `"--git-log"`.
///
/// Throws a JS `Error` if no parser has that name or the input does not fit its
/// format.
#[wasm_bindgen]
pub fn parse(name: &str, input: &str) -> Result<JsValue, JsError> {
    let output = jc_rs_parsers::parse(name, input).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&output)
}

/// Parse without the conversions that shape output to jc's schema -- jc's `-r`.
#[wasm_bindgen(js_name = parseRaw)]
pub fn parse_raw(name: &str, input: &str) -> Result<JsValue, JsError> {
    let parser = jc_rs_parsers::find(name)
        .ok_or_else(|| JsError::new(&format!("no parser named {name:?}")))?;
    let output = parser
        .parse_raw(input, true)
        .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&output)
}

/// Feed a streaming parser one line at a time.
///
/// Returns the record that line completed, or `undefined` when it completed
/// none. Call [`StreamSession::finalize`] after the last line to flush whatever
/// is still buffered.
///
/// ```js
/// const session = new StreamSession("clf_s");
/// for (const line of lines) {
///   const record = session.parseLine(line);
///   if (record !== undefined) handle(record);
/// }
/// const last = session.finalize();
/// ```
#[wasm_bindgen]
pub struct StreamSession {
    inner: Box<dyn jc_rs_core::LineParser>,
}

#[wasm_bindgen]
impl StreamSession {
    /// Throws if the name is unknown or the parser does not stream.
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str) -> Result<StreamSession, JsError> {
        let session = jc_rs_parsers::session(name)
            .ok_or_else(|| JsError::new(&format!("{name:?} is not a streaming parser")))?;
        Ok(StreamSession { inner: session })
    }

    #[wasm_bindgen(js_name = parseLine)]
    pub fn parse_line(&mut self, line: &str) -> Result<JsValue, JsError> {
        match self
            .inner
            .parse_line(line, true)
            .map_err(|e| JsError::new(&e.to_string()))?
        {
            Some(record) => to_js(&record),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    pub fn finalize(&mut self) -> Result<JsValue, JsError> {
        match self
            .inner
            .finalize(true)
            .map_err(|e| JsError::new(&e.to_string()))?
        {
            Some(record) => to_js(&record),
            None => Ok(JsValue::UNDEFINED),
        }
    }
}

/// Every parser name, sorted.
#[wasm_bindgen]
pub fn parsers() -> Vec<String> {
    let mut names: Vec<String> = jc_rs_parsers::parsers()
        .filter(|p| !p.info().hidden && !p.info().deprecated)
        .map(|p| p.info().name.to_string())
        .collect();
    names.sort_unstable();
    names
}

/// What jc-rs knows about one parser: name, argument, description, platforms
/// and tags. Returns `undefined` for an unknown name.
#[wasm_bindgen(js_name = parserInfo)]
pub fn parser_info(name: &str) -> Result<JsValue, JsError> {
    let Some(parser) = jc_rs_parsers::find(name) else {
        return Ok(JsValue::UNDEFINED);
    };
    to_js(parser.info())
}

/// The crate version, which is the jc-rs version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// serde-wasm-bindgen maps a Rust map to a JS `Map` by default, which is the
/// right call for arbitrary keys and the wrong one here: every consumer expects
/// `record.status`, `JSON.stringify(record)` and object spread to work. Records
/// are JSON objects, so they serialize as plain objects.
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[cfg(test)]
mod tests {
    // These run on the host: everything above is thin glue over
    // `jc_rs_parsers`, and the parsers themselves are tested there. What is
    // worth asserting here is that the glue reaches the right parser.
    #[test]
    fn parsers_are_registered_and_sorted() {
        let names = super::parsers();
        // The listing hides the ~50 `proc_*` parsers that `--proc` dispatches
        // to, the same way `jc-rs -l` does.
        assert!(names.len() > 150, "got {} parsers", names.len());
        assert!(names.windows(2).all(|w| w[0] <= w[1]), "not sorted");
        assert!(names.contains(&"git_log".to_string()));
        assert!(
            !names.contains(&"proc_pid_stat".to_string()),
            "hidden parsers should not be listed"
        );
    }

    #[test]
    fn a_streaming_parser_yields_a_session() {
        assert!(jc_rs_parsers::session("clf_s").is_some());
        assert!(jc_rs_parsers::session("df").is_none(), "df does not stream");
    }

    #[test]
    fn version_matches_the_workspace() {
        assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
    }
}

use crate::error::ParseError;
use crate::types::{ParseOutput, ParserInfo};

/// Core parser trait -- all parsers must implement this.
///
/// # Contract
///
/// - `info()` returns a static reference to the parser's metadata. It must be
///   pure and zero-cost (just returns a `&'static`).
///
/// - `parse()` accepts the **full input** as a `&str` and returns structured
///   JSON output. When `quiet` is `true`, the parser must suppress all warning
///   messages to stderr (but still return errors via `Result`).
///
/// - Implementations must be `Send + Sync` so parsers can be stored in a
///   global registry and invoked from any thread.
///
/// # Example
///
/// ```ignore
/// use jc_rs_core::traits::Parser;
/// use jc_rs_core::types::{ParseOutput, ParserInfo};
/// use jc_rs_core::error::ParseError;
///
/// struct DateParser;
///
/// impl Parser for DateParser {
///     fn info(&self) -> &'static ParserInfo { &DATE_INFO }
///     fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
///         // ... parse date string into a JSON object ...
///         todo!()
///     }
/// }
/// ```
pub trait Parser: Send + Sync {
    /// Returns the static metadata for this parser.
    fn info(&self) -> &'static ParserInfo;

    /// Parse the complete input string and return structured output.
    ///
    /// - `input`: the full text to parse (may contain multiple lines).
    /// - `quiet`: if `true`, suppress warning messages to stderr.
    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError>;
}

/// Extended trait for streaming parsers that process input line-by-line.
///
/// # Contract
///
/// - `parse_line()` processes a single line and returns `Ok(Some(...))` when
///   the line produces output, `Ok(None)` when the line should be skipped
///   (e.g. header lines), or `Err(...)` on parse failure.
///
/// - `finalize()` is called after the last line has been processed. It allows
///   the parser to flush any buffered state. The default implementation
///   returns `Ok(None)`.
///
/// - A `StreamingParser` must also implement `Parser`. The `Parser::parse()`
///   method on a streaming parser should process the full input by splitting
///   it into lines and calling `parse_line()` on each, collecting results.
///
/// # Streaming protocol
///
/// When `quiet` is `false` and a line fails to parse, the streaming runtime
/// should emit an error object with `_jc_meta.success = false` and continue
/// processing. When `quiet` is `true` (analogous to jc's
/// `ignore_exceptions`), errors are silently skipped.
pub trait StreamingParser: Parser {
    /// Parse a single line of input.
    ///
    /// Returns `Ok(Some(output))` for a successfully parsed line,
    /// `Ok(None)` if the line should be skipped, or `Err(e)` on failure.
    fn parse_line(&self, line: &str, quiet: bool) -> Result<Option<ParseOutput>, ParseError>;

    /// Called after the last line. Flush any buffered state.
    /// Default returns `Ok(None)` (no extra output).
    fn finalize(&self) -> Result<Option<ParseOutput>, ParseError> {
        Ok(None)
    }
}

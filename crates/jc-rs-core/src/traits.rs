use crate::error::ParseError;
use crate::types::{ParseOutput, ParserInfo};

/// One streamed record. A streaming parser emits a JSON object per record, so
/// this is deliberately narrower than [`ParseOutput`] -- an array cannot be
/// produced by a single line and should not be representable here.
pub type Record = serde_json::Map<String, serde_json::Value>;

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

    /// Upcast to the streaming interface, if this parser has one.
    ///
    /// The registry stores parsers as `&'static dyn Parser`, and a trait object
    /// cannot be downcast to a sub-trait. Rather than reach for `Any`, a
    /// streaming parser overrides this to hand back `self`, which is what lets
    /// the CLI drive it a line at a time. Non-streaming parsers keep the
    /// default and are simply not offered the streaming path.
    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        None
    }
}

/// A parser that can consume input a line at a time.
///
/// The parser itself lives in the registry as a `&'static` and is shared, so it
/// cannot hold the state a line-at-a-time parse needs (the `PING` header that
/// decides how later lines are read, the commit being accumulated, the column
/// widths taken from a header row). `session()` mints a fresh, owned
/// [`LineParser`] that carries that state for exactly one run.
pub trait StreamingParser: Parser {
    /// Start a new parse session with its own state.
    fn session(&self) -> Box<dyn LineParser>;
}

/// A single stateful streaming parse. Created by [`StreamingParser::session`],
/// used once, dropped.
///
/// # Contract
///
/// - `parse_line()` returns `Ok(Some(record))` when the line completes a
///   record, `Ok(None)` when it does not (a header, a blank line, or a line
///   that is merely accumulated into state), and `Err(_)` when the line cannot
///   be parsed.
///
///   The record a line completes is not necessarily the line itself: a parser
///   that accumulates a multi-line item emits the *previous* item when a new
///   one begins.
///
/// - `finalize()` is called once after the last line and flushes whatever is
///   still buffered -- a trailing record, a summary block. The default
///   returns `Ok(None)`.
///
/// - `quiet` matches [`Parser::parse`]: suppress warnings on stderr, still
///   return hard failures as `Err`.
pub trait LineParser {
    /// Feed one line (without its trailing newline).
    fn parse_line(&mut self, line: &str, quiet: bool) -> Result<Option<Record>, ParseError>;

    /// Called after the last line; flush any buffered state.
    fn finalize(&mut self, quiet: bool) -> Result<Option<Record>, ParseError> {
        let _ = quiet;
        Ok(None)
    }

    /// Any further records the last `parse_line` produced, in the order they
    /// follow the one it returned.
    ///
    /// Almost every parser leaves this alone: a line completes at most one
    /// record. It exists for the case where a single line both closes the
    /// previous record and opens a new one that is itself already complete.
    fn take_next(&mut self) -> Option<Record> {
        None
    }
}

/// A session for parsers whose lines are independent of one another -- no
/// header to remember, nothing accumulated across lines. Wraps the per-line
/// function so such a parser does not have to declare an empty state struct.
pub struct FnSession<F>(F);

impl<F> FnSession<F>
where
    F: FnMut(&str, bool) -> Result<Option<Record>, ParseError> + 'static,
{
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F> LineParser for FnSession<F>
where
    F: FnMut(&str, bool) -> Result<Option<Record>, ParseError> + 'static,
{
    fn parse_line(&mut self, line: &str, quiet: bool) -> Result<Option<Record>, ParseError> {
        (self.0)(line, quiet)
    }
}

/// Drive a streaming parser over a complete string and collect every record.
///
/// This is what a streaming parser's [`Parser::parse`] should call. Having the
/// batch path go through the same session as the streaming path is what keeps
/// the two from drifting: the differential corpus exercises the batch path, so
/// if they were separate implementations a green differential would say nothing
/// about what a live pipe produces.
pub fn parse_via_session(
    parser: &dyn StreamingParser,
    input: &str,
    quiet: bool,
) -> Result<ParseOutput, ParseError> {
    let mut session = parser.session();
    let mut records: Vec<Record> = Vec::new();

    for line in input.lines() {
        if let Some(record) = session.parse_line(line, quiet)? {
            records.push(record);
        }
        while let Some(record) = session.take_next() {
            records.push(record);
        }
    }

    if let Some(record) = session.finalize(quiet)? {
        records.push(record);
    }

    Ok(ParseOutput::Array(records))
}

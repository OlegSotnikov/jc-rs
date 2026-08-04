# jc-rs API Contracts

This document defines all shared interfaces for the jc-rs project. All crates (jc-rs, jc-rs-utils, jc-rs-parsers) must program against these contracts defined in jc-rs-core.

## 1. Core Types (`jc_rs_core::types`)

### ParseOutput

```rust
pub enum ParseOutput {
    Object(serde_json::Map<String, Value>),
    Array(Vec<serde_json::Map<String, Value>>),
}
```

- **Object**: Used when a parser produces a single record (e.g. `date`, `uname`, `os-release`). The map is a flat or nested JSON object.
- **Array**: Used when a parser produces multiple records (e.g. `ps`, `ls`, `netstat`). Each element is one record.

Streaming parsers always yield `Object` variants, one per successfully parsed line.

### Platform

```rust
pub enum Platform {
    Linux, Darwin, Windows, FreeBSD, OpenBSD, NetBSD, Aix, Universal,
}
```

`Universal` means the parser works on all platforms (typically file/string parsers). Used in `ParserInfo::compatible` to declare which OSes a parser supports.

### Tag

```rust
pub enum Tag {
    Command, File, String, Slurpable, Streaming, Hidden, Deprecated,
}
```

| Tag | Meaning |
|---|---|
| `Command` | Parses stdout of a CLI command |
| `File` | Parses contents of a file format |
| `String` | Parses a text/string pattern |
| `Slurpable` | Supports `--slurp` mode (multiple inputs merged into one array) |
| `Streaming` | Implements `StreamingParser` for line-by-line processing |
| `Hidden` | Not shown in default help/parser listings |
| `Deprecated` | Scheduled for removal; shown only when explicitly requested |

### ParserInfo

```rust
pub struct ParserInfo {
    pub name: &'static str,           // snake_case module name: "apt_get_sqq"
    pub argument: &'static str,       // CLI form with -- prefix: "--apt-get-sqq"
    pub version: &'static str,        // semver: "1.0.0"
    pub description: &'static str,    // one-line human description
    pub author: &'static str,         // author name
    pub author_email: &'static str,   // author email
    pub compatible: &'static [Platform],       // supported platforms
    pub tags: &'static [Tag],                  // semantic tags
    pub magic_commands: &'static [&'static str], // commands for auto-detection
    pub streaming: bool,              // true if StreamingParser is implemented
    pub hidden: bool,                 // true to hide from default listings
    pub deprecated: bool,             // true if deprecated
}
```

**Conventions for `name` and `argument`:**
- `name` is always `snake_case` (underscores): `"git_log"`, `"apt_cache_show"`
- `argument` is always `kebab-case` with `--` prefix: `"--git-log"`, `"--apt-cache-show"`
- The conversion rule: replace `_` with `-` and prepend `--`.

**Helper methods on ParserInfo:**
- `has_tag(tag: Tag) -> bool` -- check if a tag is present
- `is_compatible_with(platform: Platform) -> bool` -- checks for `Universal` or exact match
- `is_slurpable() -> bool` -- shorthand for `has_tag(Tag::Slurpable)`

## 2. Error Types (`jc_rs_core::error`)

### ParseError (parser-level)

```rust
pub enum ParseError {
    Generic(String),        // catch-all with descriptive message
    InvalidInput(String),   // input does not match expected format
    Utf8(FromUtf8Error),    // invalid UTF-8 in input
    Json(serde_json::Error),// JSON serialization/deserialization failure
    Regex(String),          // regex compilation or matching failure
}
```

**When to use each variant:**
- `Generic`: parser-specific errors that do not fit other categories
- `InvalidInput`: the input is recognizably wrong (e.g. empty, missing required headers)
- `Utf8`: byte-to-string conversion failed
- `Json`: building or parsing JSON values failed internally
- `Regex`: a regex pattern failed to compile or produced an unexpected non-match in a critical path

### CjError (application-level)

```rust
pub enum CjError {
    ParserNotFound(String),           // no parser matches the given name
    ParseFailed { parser: String, source: ParseError }, // parser found but parse() failed
    Io(std::io::Error),               // reading stdin, files, pipes
    SliceError(String),               // --slice range parsing/application error
    Other(String),                    // anything else
}
```

**When to use each variant:**
- `ParserNotFound`: the user specified a parser name that does not exist in the registry
- `ParseFailed`: wrap a `ParseError` with the parser name for context; the CLI uses this to format `"jc-rs: Error - <parser> parser could not parse the input data"`
- `Io`: any I/O error from stdin, file reads, or pipe operations
- `SliceError`: invalid `--slice` syntax (e.g. `"abc:def"`) or out-of-range indices
- `Other`: fallback for unexpected errors

**Conversion:** `ParseError` implements `From<ParseError> for CjError`, wrapping it as `ParseFailed { parser: "<unknown>", source }`. The CLI should use `CjError::ParseFailed { parser: name.into(), source: e }` explicitly when it knows the parser name.

## 3. Parser Traits (`jc_rs_core::traits`)

### Parser (required for all parsers)

```rust
pub trait Parser: Send + Sync {
    fn info(&self) -> &'static ParserInfo;
    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError>;
}
```

**Contract:**
- `info()` must return a `&'static ParserInfo`. It is zero-cost (returns a static reference). It must be pure with no side effects.
- `parse()` receives the full input as a single `&str` and returns structured output.
  - When `quiet` is `true`: suppress all warning messages to stderr. Still return `Err(...)` for hard failures.
  - When `quiet` is `false`: may emit warnings to stderr via `jc_rs_utils::warning_message()`.
  - Parsers must NOT panic. All errors must be returned as `Err(ParseError)`.

### StreamingParser (optional, for line-by-line parsers)

```rust
pub trait StreamingParser: Parser {
    fn parse_line(&self, line: &str, quiet: bool) -> Result<Option<ParseOutput>, ParseError>;
    fn finalize(&self) -> Result<Option<ParseOutput>, ParseError> { Ok(None) }
}
```

**Contract:**
- `parse_line()`:
  - `Ok(Some(output))`: the line was successfully parsed into a JSON object
  - `Ok(None)`: the line should be skipped (e.g. header, blank, separator)
  - `Err(e)`: the line could not be parsed
- `finalize()`: called after the last line; flush any buffered state. Default returns `Ok(None)`.
- A `StreamingParser` must ALSO have a working `Parser::parse()` implementation. This should split the input by lines, call `parse_line()` on each, collect all `Some(...)` results, and return them as `ParseOutput::Array(...)`.

## 4. Parser Registry (`jc_rs_core::registry`)

### ParserEntry

```rust
pub struct ParserEntry {
    parser: &'static dyn Parser,
}

impl ParserEntry {
    pub const fn new(parser: &'static dyn Parser) -> Self;
    pub fn parser(&self) -> &'static dyn Parser;
}

inventory::collect!(ParserEntry);
```

### Registration (for parser implementors)

Each parser must register itself using `inventory::submit!` in its module:

```rust
use jc_rs_core::{ParserEntry, Parser, ParserInfo, ParseOutput, ParseError};
use jc_rs_core::types::{Platform, Tag};

struct DfParser;

static DF_INFO: ParserInfo = ParserInfo {
    name: "df",
    argument: "--df",
    version: "1.0.0",
    description: "df command parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin],
    tags: &[Tag::Command],
    magic_commands: &["df"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

impl Parser for DfParser {
    fn info(&self) -> &'static ParserInfo { &DF_INFO }
    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        // ... implementation ...
        todo!()
    }
}

static DF_PARSER: DfParser = DfParser;

inventory::submit! {
    ParserEntry::new(&DF_PARSER)
}
```

### Lookup Functions

```rust
/// Iterate over ALL registered parsers (unordered).
pub fn all_parsers() -> impl Iterator<Item = &'static dyn Parser>;

/// Find parser by name. Accepts snake_case ("apt_get_sqq"),
/// kebab-case ("apt-get-sqq"), or argument form ("--apt-get-sqq").
pub fn find_parser(name: &str) -> Option<&'static dyn Parser>;

/// Find parser by magic command. `words` is the argv of the command
/// the user ran (e.g. ["df", "-h"]). Matches against magic_commands entries.
pub fn find_magic_parser(words: &[&str]) -> Option<&'static dyn Parser>;
```

## 5. Naming Conventions

| Item | Convention | Example |
|---|---|---|
| Rust types/enums | `CamelCase` | `ParseOutput`, `ParserInfo` |
| Rust functions | `snake_case` | `find_parser`, `all_parsers` |
| Parser module name | `snake_case` | `"apt_get_sqq"`, `"git_log"` |
| Parser CLI argument | `kebab-case` with `--` | `"--apt-get-sqq"`, `"--git-log"` |
| jc-rs-utils functions | `snake_case` | `convert_to_int`, `normalize_key` |

## 6. jc-rs-utils Function Contracts

Utility functions in `jc-rs-utils` serve parser implementations. They should follow these patterns:

| Function | Signature | Returns |
|---|---|---|
| `warning_message` | `(lines: &[&str])` | prints to stderr, returns nothing |
| `error_message` | `(lines: &[&str])` | prints to stderr, returns nothing |
| `has_data` | `(input: &str) -> bool` | `true` if input is non-empty after trimming |
| `input_type_check` | `(input: &str) -> Result<(), ParseError>` | `Err(InvalidInput)` if empty |
| `convert_to_int` | `(value: &str) -> Option<i64>` | parsed int or `None` |
| `convert_to_float` | `(value: &str) -> Option<f64>` | parsed float or `None` |
| `convert_to_bool` | `(value: &str) -> Option<bool>` | `true`/`false`/`None` for known bool-ish strings |
| `normalize_key` | `(key: &str) -> String` | lowercase, spaces/special chars to `_` |
| `remove_quotes` | `(s: &str) -> &str` | strip matching outer quotes |
| `line_slice` | `(data: &str, start: Option<i64>, end: Option<i64>) -> Vec<&str>` | Python-style slice of lines |

## 7. How CLI Should Call Parsers

The CLI (`jc-rs`) interacts with jc-rs-core through this flow:

```
1. Parse CLI arguments to determine:
   - parser name (via --parser-name or magic command detection)
   - options: quiet, raw, slurp, pretty, mono, meta, etc.

2. Find the parser:
   a. If user gave --parser-name: call find_parser("parser_name")
   b. If magic mode (no --parser): call find_magic_parser(&argv_words)
   c. If not found: return CjError::ParserNotFound

3. Read input:
   a. Standard parser: read all stdin into a String
   b. Streaming parser: read stdin line-by-line

4. Execute parser:
   a. Standard: parser.parse(&input, quiet) -> Result<ParseOutput, ParseError>
   b. Streaming: iterate lines, call streaming_parser.parse_line() on each,
      collect results. On error: if ignore_exceptions, emit error meta object
      and continue; otherwise propagate error.

5. Post-process output:
   a. Apply --slice if requested
   b. Add _cj_meta if --meta requested (timestamp, parser name, magic info)
   c. Serialize to JSON (pretty or compact)
   d. Colorize if terminal and not --mono
   e. Print to stdout

6. Exit codes:
   - 0: success
   - 100: jc-rs error (parse failure, parser not found, etc.)
   - Other: passthrough from magic command execution
```

### Meta Object Structure

When `--meta` is enabled, the CLI wraps output with a `_cj_meta` key:

```json
{
  "_cj_meta": {
    "parser": "df",
    "timestamp": 1711500000.0,
    "slice": "0:5",
    "magic_command": ["df", "-h"],
    "magic_command_exit": 0
  }
}
```

### Streaming Error Object

When a streaming parser encounters an error with `ignore_exceptions = true`:

```json
{
  "_cj_meta": {
    "success": false,
    "error": "ParseError: unexpected input format: ...",
    "line": "the original line that failed"
  }
}
```

When a line parses successfully with `ignore_exceptions = true`:

```json
{
  "field1": "value1",
  "_cj_meta": { "success": true }
}
```

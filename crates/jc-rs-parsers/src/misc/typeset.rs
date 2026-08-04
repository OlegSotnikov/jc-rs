//! Parser for `typeset` and `declare` output (bash/zsh).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub struct TypesetParser;

static INFO: ParserInfo = ParserInfo {
    name: "typeset",
    argument: "--typeset",
    version: "1.0.0",
    description: "`typeset` and `declare` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TYPESET_PARSER: TypesetParser = TypesetParser;

inventory::submit! {
    ParserEntry::new(&TYPESET_PARSER)
}

// jc's patterns, verbatim. They are applied with `search`, not `match`, so a
// definition is found anywhere in the line -- which is what lets the same
// pattern handle `VAR=x` and `declare -x VAR=x`.
static VAR_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)=(?P<val>[^(][^\[].+)$")
        .expect("valid var pattern")
});
static SIMPLE_ARRAY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)=(?P<body>\(\[\d+\]=.+\))$")
        .expect("valid array pattern")
});
static ASSOCIATIVE_ARRAY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)=(?P<body>\(\[[a-zA-Z_][a-zA-Z0-9_]*\]=.+\))$")
        .expect("valid associative pattern")
});
static EMPTY_ARRAY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)=\(\)$").expect("valid empty array pattern")
});
static EMPTY_VAR_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"declare\s.+\s(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)$").expect("valid empty var pattern")
});
static DECLARE_OPTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"declare\s(?P<options>.+?)\s[a-zA-Z_][a-zA-Z0-9_]*").expect("valid opts pattern")
});

/// What `declare -irx` says about a name, plus which of the three shapes it is.
///
/// The flags stay `None` when the line carries no `declare` at all -- plain
/// `typeset` output says nothing about them, and jc reports that as null rather
/// than guessing false.
struct DeclareOptions {
    kind: &'static str,
    readonly: Option<bool>,
    integer: Option<bool>,
    lowercase: Option<bool>,
    uppercase: Option<bool>,
    exported: Option<bool>,
}

impl DeclareOptions {
    fn is_integer(&self) -> bool {
        self.integer == Some(true)
    }
}

fn declare_options(line: &str, type_hint: &'static str) -> DeclareOptions {
    let mut opts = DeclareOptions {
        kind: type_hint,
        readonly: None,
        integer: None,
        lowercase: None,
        uppercase: None,
        exported: None,
    };

    let Some(caps) = DECLARE_OPTS.captures(line) else {
        return opts;
    };
    for flag in caps["options"].chars() {
        match flag {
            'r' => opts.readonly = Some(true),
            'i' => opts.integer = Some(true),
            'l' => opts.lowercase = Some(true),
            'u' => opts.uppercase = Some(true),
            'x' => opts.exported = Some(true),
            'a' => opts.kind = "array",
            'A' => opts.kind = "associative",
            _ => {}
        }
    }
    // A `declare` line states every flag by omission, so the rest are false.
    for flag in [
        &mut opts.readonly,
        &mut opts.integer,
        &mut opts.lowercase,
        &mut opts.uppercase,
        &mut opts.exported,
    ] {
        flag.get_or_insert(false);
    }
    opts
}

fn strip_bookends(s: &str, start: char, end: char) -> &str {
    s.strip_prefix(start)
        .and_then(|s| s.strip_suffix(end))
        .unwrap_or(s)
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Split `[0]="a" [1]="b"` into its items the way `shlex.split` does in POSIX
/// mode: quotes group and are removed, and a backslash escapes the character
/// after it -- which is how `declare -p` writes a quote inside a value.
fn shell_split(s: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                    started = true;
                }
            }
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            ' ' | '\t' if !in_quotes => {
                if started {
                    items.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                current.push(ch);
                started = true;
            }
        }
    }
    if started {
        items.push(current);
    }
    items
}

fn simple_array_values(body: &str, integer: bool) -> Value {
    let values = shell_split(strip_bookends(body, '(', ')'))
        .into_iter()
        .filter_map(|item| {
            item.split_once('=')
                .map(|(_, val)| strip_quotes(val).to_string())
        })
        .map(|val| typed_value(&val, integer))
        .collect();
    Value::Array(values)
}

fn associative_array_values(body: &str, integer: bool) -> Value {
    let mut map = Map::new();
    for item in shell_split(strip_bookends(body, '(', ')')) {
        let Some((key, val)) = item.split_once('=') else {
            continue;
        };
        map.insert(
            strip_bookends(key, '[', ']').to_string(),
            typed_value(val, integer),
        );
    }
    Value::Object(map)
}

/// `declare -i` means the shell treats the value as a number, and jc converts
/// it; everything else stays a string.
fn typed_value(val: &str, integer: bool) -> Value {
    if integer {
        return convert_to_int(val).map_or(Value::Null, |n| Value::Number(n.into()));
    }
    Value::String(val.to_string())
}

fn record(name: &str, value: Value, opts: DeclareOptions) -> Map<String, Value> {
    let flag = |v: Option<bool>| v.map_or(Value::Null, Value::Bool);
    let mut item = Map::with_capacity(8);
    item.insert("name".to_string(), Value::String(name.to_string()));
    item.insert("value".to_string(), value);
    item.insert("type".to_string(), Value::String(opts.kind.to_string()));
    item.insert("readonly".to_string(), flag(opts.readonly));
    item.insert("integer".to_string(), flag(opts.integer));
    item.insert("lowercase".to_string(), flag(opts.lowercase));
    item.insert("uppercase".to_string(), flag(opts.uppercase));
    item.insert("exported".to_string(), flag(opts.exported));
    item
}

pub fn parse_typeset(input: &str) -> Vec<Map<String, Value>> {
    let mut output = Vec::new();

    for line in input.lines().filter(|l| !l.is_empty()) {
        // Order matters: the variable pattern is the most permissive, and jc
        // tries it first.
        if let Some(caps) = VAR_DEF.captures(line) {
            let opts = declare_options(line, "variable");
            let value = typed_value(strip_quotes(&caps["val"]), opts.is_integer());
            output.push(record(&caps["name"], value, opts));
            continue;
        }

        if let Some(caps) = EMPTY_VAR_DEF.captures(line) {
            let opts = declare_options(line, "variable");
            output.push(record(&caps["name"], Value::Null, opts));
            continue;
        }

        if let Some(caps) = SIMPLE_ARRAY_DEF.captures(line) {
            let opts = declare_options(line, "array");
            let value = simple_array_values(&caps["body"], opts.is_integer());
            output.push(record(&caps["name"], value, opts));
            continue;
        }

        if let Some(caps) = ASSOCIATIVE_ARRAY_DEF.captures(line) {
            let opts = declare_options(line, "associative");
            let value = associative_array_values(&caps["body"], opts.is_integer());
            output.push(record(&caps["name"], value, opts));
            continue;
        }

        if let Some(caps) = EMPTY_ARRAY_DEF.captures(line) {
            let opts = declare_options(line, "array");
            output.push(record(&caps["name"], Value::Array(Vec::new()), opts));
            continue;
        }
    }

    output
}

impl Parser for TypesetParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Array(parse_typeset(input)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! typeset_test {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let input = include_str!($input);
                let expected: serde_json::Value =
                    serde_json::from_str(include_str!($expected)).unwrap();
                let result = TypesetParser.parse(input, false).unwrap();
                assert_eq!(serde_json::to_value(result).unwrap(), expected);
            }
        };
    }

    typeset_test!(
        test_typeset_plain,
        "../../../../tests/fixtures/generic/typeset--plain.out",
        "../../../../tests/fixtures/generic/typeset--plain.json"
    );
    typeset_test!(
        test_typeset_a,
        "../../../../tests/fixtures/generic/typeset--a.out",
        "../../../../tests/fixtures/generic/typeset--a.json"
    );
    typeset_test!(
        test_typeset_capital_a,
        "../../../../tests/fixtures/generic/typeset--capital-a.out",
        "../../../../tests/fixtures/generic/typeset--capital-a.json"
    );
    typeset_test!(
        test_typeset_p,
        "../../../../tests/fixtures/generic/typeset--p.out",
        "../../../../tests/fixtures/generic/typeset--p.json"
    );
    typeset_test!(
        test_typeset_r,
        "../../../../tests/fixtures/generic/typeset--r.out",
        "../../../../tests/fixtures/generic/typeset--r.json"
    );

    #[test]
    fn test_typeset_integer_values_are_converted() {
        let out = parse_typeset("declare -i COUNT=4200\n");
        assert_eq!(out[0]["value"], serde_json::json!(4200));
        assert_eq!(out[0]["integer"], serde_json::json!(true));
    }

    #[test]
    fn test_typeset_without_declare_leaves_the_flags_null() {
        // Plain `typeset` output states nothing about readonly/integer/..., and
        // jc reports null rather than guessing false.
        let out = parse_typeset("SHELL=/bin/bash\n");
        assert_eq!(out[0]["readonly"], serde_json::Value::Null);
        assert_eq!(out[0]["type"], "variable");
    }

    #[test]
    fn test_typeset_value_shorter_than_three_characters_is_skipped() {
        // jc's variable pattern requires three characters after the `=`; a
        // shorter value matches nothing and produces no record. Replicated so
        // the two agree, quirk and all.
        assert!(parse_typeset("declare -i COUNT=42\n").is_empty());
    }
}

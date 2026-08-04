//! jc-rs — jc rewritten in Rust.
//!
//! Entry point for the `jc-rs` binary. Handles argument parsing, parser dispatch,
//! input reading, output formatting, and exit codes.

mod args;
mod completions;
mod magic;
mod meta;
mod output;
mod streaming;

// Force the parsers crate to be linked (and its inventory::submit! calls run).
extern crate jc_rs_parsers;

use jc_rs_core::find_parser;
use jc_rs_core::registry::all_parsers;
use jc_rs_core::types::{ParseOutput, Tag};
use serde_json::{Map, Value};
use std::io::{self, Read};
use std::process;

use args::parse_args;
use meta::MetaInfo;
use output::{ColorScheme, print_output, should_use_color};

// ─── Version / About ────────────────────────────────────────────────────────

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "jc-rs contributors";
const WEBSITE: &str = "https://jc-rs.com";
const COPYRIGHT: &str = "© 2026 Oleg Sotnikov";
const LICENSE: &str = "MIT License";

// ─── Exit codes (mirrors jc) ─────────────────────────────────────────────────

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 100;
#[allow(dead_code)]
const EXIT_MAX: i32 = 255;

// ─── Help / version text ─────────────────────────────────────────────────────

fn helptext_preamble() -> &'static str {
    "jc-rs converts the output of many commands, file-types, and strings to JSON or YAML\n\
     \n\
     Usage:\n\
     \n\
     Standard syntax:\n\
     \n\
         COMMAND | jc-rs [SLICE] [OPTIONS] PARSER\n\
     \n\
         cat FILE | jc-rs [SLICE] [OPTIONS] PARSER\n\
     \n\
     Magic syntax:\n\
     \n\
         jc-rs [SLICE] [OPTIONS] COMMAND\n\
     \n\
         jc-rs [SLICE] [OPTIONS] /proc/<path-to-procfile>\n\
     \n\
     Parsers:\n"
}

fn options_text() -> String {
    let options = [
        ("-a,  --about", "about jc-rs"),
        (
            "-B,  --bash-comp",
            "gen Bash completion: jc-rs -B > /etc/bash_completion.d/jc-rs",
        ),
        ("-C,  --force-color", "force color output (overrides -m)"),
        ("-d,  --debug", "debug (double for verbose debug)"),
        ("-h,  --help", "help (--help --parser-name for parser docs)"),
        ("-l,  --list", "list available parsers"),
        ("-L,  --list-all", "list all parsers including hidden"),
        ("-m,  --monochrome", "monochrome output"),
        (
            "-M,  --meta-out",
            "add metadata to output including timestamp",
        ),
        ("-p,  --pretty", "pretty print output"),
        (
            "-q,  --quiet",
            "suppress warnings (double to ignore streaming errors)",
        ),
        ("-r,  --raw", "raw output"),
        ("-s,  --slurp", "slurp multiple lines into an array"),
        ("-u,  --unbuffer", "unbuffer output"),
        ("-v,  --version", "version info"),
        ("-y,  --yaml-out", "YAML output"),
        (
            "-Z,  --zsh-comp",
            "gen Zsh completion: jc-rs -Z > \"${fpath[1]}/_cj\"",
        ),
    ];
    let pad = 22usize;
    let mut out = String::new();
    for (flag, desc) in &options {
        let padding = pad.saturating_sub(flag.len());
        out.push_str(&format!("    {}{:pad$}{}\n", flag, "", desc, pad = padding));
    }
    out
}

fn slicetext() -> &'static str {
    "Slice:\n\
     \n\
         [start]:[end]\n\
     \n\
             start: [[-]index] - Zero-based start line, negative for counting from end\n\
             end:   [[-]index] - Zero-based end line (excluding), negative for counting from end\n"
}

fn helptext_end() -> &'static str {
    "Examples:\n\
     \n\
         Standard Syntax:\n\
             $ df -h | jc-rs --pretty --df\n\
             $ cat /proc/meminfo | jc-rs --pretty --proc\n\
     \n\
         Magic Syntax:\n\
             $ jc-rs --pretty df -h\n\
             $ jc-rs --pretty /proc/meminfo\n\
     \n\
         Line Slicing:\n\
             $ cat output.txt | jc-rs 4:15 --<PARSER>  # Parse lines 4-14 (zero-based)\n\
     \n\
         Parser Documentation:\n\
             $ jc-rs --help --df\n\
     \n\
         More Help:\n\
             $ jc-rs -hh          # show hidden parsers\n\
             $ jc-rs -hhh         # list parsers by category\n"
}

fn parsers_text(show_hidden: bool) -> String {
    let pad = 22usize;
    let indent = 4usize;
    let mut parsers: Vec<_> = all_parsers()
        .filter(|p| {
            if !show_hidden {
                !p.info().hidden && !p.info().deprecated
            } else {
                !p.info().deprecated
            }
        })
        .collect();
    parsers.sort_by_key(|p| p.info().argument);

    let mut out = String::new();
    for p in &parsers {
        let arg = p.info().argument;
        let desc = p.info().description;
        let padding = pad.saturating_sub(arg.len());
        out.push_str(&format!(
            "{:indent$}{}{:pad$}{}\n",
            "",
            arg,
            "",
            desc,
            indent = indent,
            pad = padding
        ));
    }
    out
}

fn print_help(show_hidden: bool) {
    let ptext = parsers_text(show_hidden);
    let otext = options_text();
    print!(
        "{}{}\nOptions:\n{}\n{}\n{}",
        helptext_preamble(),
        ptext,
        otext,
        slicetext(),
        helptext_end()
    );
}

fn print_help_categories() {
    let all: Vec<_> = all_parsers().filter(|p| !p.info().deprecated).collect();

    let pad = 22usize;
    let categories: &[(&str, Tag)] = &[
        ("Command Parsers:", Tag::Command),
        ("File Parsers:", Tag::File),
        ("String Parsers:", Tag::String),
        ("Slurpable Parsers:", Tag::Slurpable),
        ("Streaming Parsers:", Tag::Streaming),
    ];

    for (cat_name, tag) in categories {
        let mut cat: Vec<_> = all.iter().filter(|p| p.info().has_tag(*tag)).collect();
        cat.sort_by_key(|p| p.info().argument);

        println!("{}  ({})", cat_name, cat.len());
        for p in &cat {
            let arg = p.info().argument;
            let desc = p.info().description;
            let padding = pad.saturating_sub(arg.len());
            println!("{}{:pad$}{}", arg, "", desc, pad = padding);
        }
        println!();
    }
}

fn print_version() {
    println!(
        "jc-rs version:  {}\n\
         \n\
         {}\n\
         {}",
        VERSION, WEBSITE, COPYRIGHT
    );
}

fn about_cj() -> Value {
    let parsers: Vec<Value> = all_parsers()
        .map(|p| {
            let info = p.info();
            let mut m = Map::new();
            m.insert("name".to_string(), Value::String(info.name.to_string()));
            m.insert(
                "argument".to_string(),
                Value::String(info.argument.to_string()),
            );
            m.insert(
                "version".to_string(),
                Value::String(info.version.to_string()),
            );
            m.insert(
                "description".to_string(),
                Value::String(info.description.to_string()),
            );
            m.insert("author".to_string(), Value::String(info.author.to_string()));
            m.insert(
                "author_email".to_string(),
                Value::String(info.author_email.to_string()),
            );
            m.insert("streaming".to_string(), Value::Bool(info.streaming));
            m.insert("hidden".to_string(), Value::Bool(info.hidden));
            m.insert("deprecated".to_string(), Value::Bool(info.deprecated));
            Value::Object(m)
        })
        .collect();

    let parser_count = all_parsers().count();

    let mut m = Map::new();
    m.insert("name".to_string(), Value::String("jc-rs".to_string()));
    m.insert("version".to_string(), Value::String(VERSION.to_string()));
    m.insert(
        "description".to_string(),
        Value::String("JSON Convert (Rust)".to_string()),
    );
    m.insert("author".to_string(), Value::String(AUTHOR.to_string()));
    m.insert("website".to_string(), Value::String(WEBSITE.to_string()));
    m.insert(
        "copyright".to_string(),
        Value::String(COPYRIGHT.to_string()),
    );
    m.insert("license".to_string(), Value::String(LICENSE.to_string()));
    m.insert(
        "parser_count".to_string(),
        Value::Number(parser_count.into()),
    );
    m.insert("parsers".to_string(), Value::Array(parsers));
    Value::Object(m)
}

fn print_parser_list(show_all: bool) {
    let pad = 22usize;
    let mut parsers: Vec<_> = all_parsers()
        .filter(|p| {
            if show_all {
                true
            } else {
                !p.info().hidden && !p.info().deprecated
            }
        })
        .collect();
    parsers.sort_by_key(|p| p.info().argument);

    for p in &parsers {
        let arg = p.info().argument;
        let desc = p.info().description;
        let padding = pad.saturating_sub(arg.len());
        println!("{}{:pad$}{}", arg, "", desc, pad = padding);
    }
}

// ─── Slice utility ───────────────────────────────────────────────────────────

/// Parse a slice string "start:end" into (Option<i64>, Option<i64>).
fn parse_slice(s: &str) -> Result<(Option<i64>, Option<i64>), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid slice: {}", s));
    }
    let start = if parts[0].is_empty() {
        None
    } else {
        parts[0]
            .parse::<i64>()
            .ok()
            .map(Some)
            .ok_or_else(|| format!("Invalid slice start: {}", parts[0]))?
    };
    let end = if parts[1].is_empty() {
        None
    } else {
        parts[1]
            .parse::<i64>()
            .ok()
            .map(Some)
            .ok_or_else(|| format!("Invalid slice end: {}", parts[1]))?
    };
    Ok((start, end))
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn run() -> i32 {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&raw_args);

    // ── Color setup ──
    let use_color = should_use_color(args.force_color, args.mono);
    let scheme = ColorScheme::from_env();

    // ── Info-and-exit modes ──────────────────────────────────────────────────

    if args.version {
        print_version();
        return EXIT_OK;
    }

    if args.about {
        let val = about_cj();
        print_output(
            &val,
            args.pretty,
            args.yaml,
            use_color,
            &scheme,
            args.unbuffer,
        );
        return EXIT_OK;
    }

    if args.bash_comp {
        print!("{}", completions::bash_completion());
        return EXIT_OK;
    }

    if args.zsh_comp {
        print!("{}", completions::zsh_completion());
        return EXIT_OK;
    }

    if args.list_parsers {
        print_parser_list(false);
        return EXIT_OK;
    }

    if args.list_all {
        print_parser_list(true);
        return EXIT_OK;
    }

    // ── Help ─────────────────────────────────────────────────────────────────
    if args.help {
        if args.show_categories {
            print_help_categories();
        } else {
            // If a parser name was given, show parser-specific help
            if let Some(ref pname) = args.parser_name
                && let Some(parser) = find_parser(pname)
            {
                let info = parser.info();
                println!("{}\n", info.description);
                println!("Parser:        {}", info.argument);
                println!("Version:       {}", info.version);
                println!("Author:        {} ({})", info.author, info.author_email);
                let compat: Vec<&str> = info
                    .compatible
                    .iter()
                    .map(|p| match p {
                        jc_rs_core::types::Platform::Linux => "linux",
                        jc_rs_core::types::Platform::Darwin => "darwin",
                        jc_rs_core::types::Platform::Windows => "windows",
                        jc_rs_core::types::Platform::FreeBSD => "freebsd",
                        jc_rs_core::types::Platform::OpenBSD => "openbsd",
                        jc_rs_core::types::Platform::NetBSD => "netbsd",
                        jc_rs_core::types::Platform::Aix => "aix",
                        jc_rs_core::types::Platform::Universal => "universal",
                    })
                    .collect();
                println!("Compatible:    {}", compat.join(", "));
                if info.is_slurpable() {
                    println!("\nThis parser can be used with the --slurp option.");
                }
                return EXIT_OK;
            }
            print_help(args.show_hidden);
        }
        return EXIT_OK;
    }

    // ── If no args at all, show help ─────────────────────────────────────────
    if raw_args.is_empty() {
        print_help(false);
        return EXIT_OK;
    }

    // ── Determine parser and input ────────────────────────────────────────────

    let mut magic_returncode: i32 = 0;
    let mut magic_command_words: Option<Vec<String>> = None;
    let mut input_list: Option<Vec<String>> = None;

    let (parser, input_data) = if let Some(ref cmd_words) = args.magic_command {
        // Magic mode
        match magic::run_magic(cmd_words) {
            Ok(result) => {
                if !result.stderr.is_empty() {
                    // Print stderr to our stderr (mirroring jc behavior)
                    let trimmed = result.stderr.trim_end();
                    eprintln!("{}", trimmed);
                }
                magic_returncode = result.exit_code;
                magic_command_words = Some(cmd_words.clone());
                let il = result.input_list.clone();
                let mp = result.multi_proc;
                if mp {
                    input_list = il.clone();
                }
                (result.parser, result.stdout)
            }
            Err(e) => {
                eprintln!("jc-rs: error - {}", e);
                return EXIT_ERROR;
            }
        }
    } else if let Some(ref pname) = args.parser_name {
        // Standard mode: explicit --parser-name
        match find_parser(pname) {
            Some(p) => {
                // Read stdin
                if atty::is(atty::Stream::Stdin) {
                    eprintln!("jc-rs: error - Missing piped data. Use \"jc-rs -h\" for help.");
                    return EXIT_ERROR;
                }
                let mut buf = String::new();
                if let Err(e) = io::stdin().read_to_string(&mut buf) {
                    eprintln!("jc-rs: error - Failed to read stdin: {}", e);
                    return EXIT_ERROR;
                }
                (p, buf)
            }
            None => {
                eprintln!(
                    "jc-rs: error - Unknown parser: --{}. Use \"jc-rs -h\" for help.",
                    pname
                );
                return EXIT_ERROR;
            }
        }
    } else {
        // No parser or magic command found
        eprintln!("jc-rs: error - Missing or incorrect arguments. Use \"jc-rs -h\" for help.");
        return EXIT_ERROR;
    };

    // ── Slurp check ───────────────────────────────────────────────────────────
    if args.slurp && !streaming::parser_is_slurpable(parser) {
        eprintln!(
            "jc-rs: error - Slurp option not available with the {} parser. Use \"jc-rs -hhh\" for compatible parsers.",
            parser.info().name
        );
        return EXIT_ERROR;
    }

    // ── Streaming parser path ─────────────────────────────────────────────────
    // Streaming parsers implement Parser::parse() which calls parse_line()
    // internally for each line and returns ParseOutput::Array. We use the
    // standard parse path below which works for both streaming and non-streaming
    // parsers. True line-by-line streaming (unbuffered output per line) would
    // require runtime downcasting which isn't possible with dyn Parser.
    // This behavior matches jc when all stdin is available at once.

    // ── Standard parse path ───────────────────────────────────────────────────
    // Apply slice to input BEFORE parsing
    let sliced_input: String = if let Some(ref slice_s) = args.slice_str {
        match parse_slice(slice_s) {
            Ok((start, end)) => {
                let lines: Vec<&str> = input_data.lines().collect();
                let len = lines.len() as i64;
                let normalize = |idx: i64| -> usize {
                    if idx < 0 {
                        (len + idx).max(0) as usize
                    } else {
                        (idx as usize).min(lines.len())
                    }
                };
                let s = start.map(normalize).unwrap_or(0);
                let e = end.map(normalize).unwrap_or(lines.len());
                lines[s..e].join("\n")
            }
            Err(e) => {
                eprintln!("jc-rs: warning - {}", e);
                input_data.clone()
            }
        }
    } else {
        input_data.clone()
    };

    // Parse
    let parse_result = parser.parse(&sliced_input, args.quiet);

    let output = match parse_result {
        Ok(out) => out,
        Err(e) => {
            if args.debug {
                eprintln!("jc-rs: parse error details: {:?}", e);
            }
            eprintln!(
                "jc-rs: error - {} parser could not parse the input data.\n\
                 If this is the correct parser, try setting the locale to C (LC_ALL=C).\n\
                 For details use the -d or -dd option. Use \"jc-rs -h --{}\" for help.",
                parser.info().name,
                parser.info().name
            );
            return EXIT_ERROR + magic_returncode;
        }
    };

    // Convert to Value
    let mut value: Value = match output {
        ParseOutput::Object(obj) => {
            if args.slurp {
                // Slurp: wrap single object in array (not standard, jc handles this differently)
                Value::Array(vec![Value::Object(obj)])
            } else {
                Value::Object(obj)
            }
        }
        ParseOutput::Array(arr) => {
            // Apply slice if we haven't already (slurp case)
            if args.slurp {
                // Already an array — no further wrapping needed
                Value::Array(arr.into_iter().map(Value::Object).collect())
            } else {
                // Apply output-level slice (if slice was on the output, not input lines)
                Value::Array(arr.into_iter().map(Value::Object).collect())
            }
        }
    };

    // Meta injection
    if args.meta_out {
        let mut meta_info = MetaInfo::new_now(parser.info().name);
        if let Some(ref cmd) = magic_command_words {
            meta_info.magic_command = Some(cmd.clone());
            meta_info.magic_command_exit = Some(magic_returncode);
        }
        if let Some(ref s) = args.slice_str
            && let Ok((start, end)) = parse_slice(s)
        {
            meta_info.slice_start = start;
            meta_info.slice_end = end;
        }
        if let Some(ref il) = input_list {
            meta_info.input_list = Some(il.clone());
        }
        meta::inject_meta(&mut value, &meta_info);
    }

    // Print output
    print_output(
        &value,
        args.pretty,
        args.yaml,
        use_color,
        &scheme,
        args.unbuffer,
    );

    // Exit code: magic_returncode is 0 for standard parse
    EXIT_OK + magic_returncode
}

fn main() {
    let code = run();
    process::exit(code);
}

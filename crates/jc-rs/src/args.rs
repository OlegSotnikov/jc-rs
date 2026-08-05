//! Argument parsing for jc-rs CLI.
//!
//! jc-rs uses a custom argument parser (not clap/argparse) to match jc's behavior.
//! Key behaviors:
//! - Short options can be combined: `-pq`, `-dd`
//! - Long options map to single-char codes
//! - `--parser-name` args starting with `--` that aren't known options → parser names
//! - Slice syntax: `[start]:[end]` (e.g. `1:5`, `-3:`, `:10`)
//! - Magic mode: non-option args that aren't a parser form the command to run

use std::collections::HashMap;

/// Parsed CLI arguments.
#[derive(Debug, Default)]
pub struct Args {
    // Display / output options
    pub pretty: bool,
    pub yaml: bool,
    pub raw: bool,
    pub mono: bool,
    pub force_color: bool,

    // Behavior options
    pub quiet: bool,             // -q
    pub ignore_exceptions: bool, // -qq (double quiet)
    pub slurp: bool,
    pub unbuffer: bool,
    pub meta_out: bool, // -M / --meta-out

    // Debug
    pub debug: bool,
    pub verbose_debug: bool, // -dd

    // Info modes (exit after printing)
    pub about: bool,
    pub version: bool,
    pub help: bool,
    pub show_hidden: bool,     // -hh
    pub show_categories: bool, // -hhh
    pub list_parsers: bool,    // -l / --list
    pub list_all: bool,        // -la / --list-all

    // Completions
    pub bash_comp: bool, // -B / --bash-comp
    pub zsh_comp: bool,  // -Z / --zsh-comp
    pub fish_comp: bool, // -F / --fish-comp

    // Parser selected via --parser-name
    pub parser_name: Option<String>,

    // Slice syntax  e.g. "1:5" or "-3:"
    pub slice_str: Option<String>,

    // Magic mode: the remainder command words (e.g. ["ls", "-al"])
    pub magic_command: Option<Vec<String>>,

    // Extra args (after parser name, or unrecognised)
    pub extra_args: Vec<String>,
}

/// Long-option → single-char code map (mirrors jc's cli_data.py).
fn long_options_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("--about", "a");
    m.insert("--force-color", "C");
    m.insert("--debug", "d");
    m.insert("--help", "h");
    m.insert("--monochrome", "m");
    m.insert("--meta-out", "M");
    m.insert("--pretty", "p");
    m.insert("--quiet", "q");
    m.insert("--raw", "r");
    m.insert("--slurp", "s");
    m.insert("--unbuffer", "u");
    m.insert("--version", "v");
    m.insert("--yaml-out", "y");
    m.insert("--bash-comp", "B");
    m.insert("--zsh-comp", "Z");
    m.insert("--fish-comp", "F");
    m.insert("--list", "l");
    m.insert("--list-all", "L");
    m
}

/// Returns true if the string looks like a slice: optional-sign digits colon optional-sign digits.
pub fn is_slice(s: &str) -> bool {
    // Must contain exactly one ':'
    if !s.contains(':') {
        return false;
    }
    // Pattern: -?[0-9]*:-?[0-9]*

    {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return false;
        }
        let left = parts[0];
        let right = parts[1];
        let valid_part = |p: &str| {
            if p.is_empty() {
                return true;
            }
            let p = if p.starts_with('-') { &p[1..] } else { p };
            !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())
        };
        valid_part(left) && valid_part(right)
    }
}

/// Parse argv (excluding argv[0]) into an `Args` struct.
///
/// This mirrors jc's two-phase parsing:
/// 1. Try magic-mode: scan leading flags, then collect remaining words as the command.
/// 2. Fall back to standard mode: find `--parser-name` args.
pub fn parse_args(argv: &[String]) -> Args {
    let long_opts = long_options_map();
    let mut args = Args::default();

    // Collect raw option chars (with count for -dd, -hh, -hhh, -qq)
    let mut option_chars: Vec<char> = Vec::new();

    // -----------------------------------------------------------------------
    // Phase 1: magic-mode scan
    // Walk argv looking for flags and a non-flag word (the command to run).
    // -----------------------------------------------------------------------
    let mut magic_options: Vec<char> = Vec::new();
    let remaining = argv.to_vec();
    let mut slice_found: Option<String> = None;
    let mut magic_mode = false;
    let mut magic_start_idx: Option<usize> = None;

    // Check if a real parser-name is given (--word that is NOT a known option)
    // jc: "if args[1].startswith('--') and args[1] not in long_options_map: bail"
    let has_explicit_parser = argv
        .first()
        .map(|a| a.starts_with("--") && !long_opts.contains_key(a.as_str()))
        .unwrap_or(false);

    if !has_explicit_parser {
        // Try magic mode: consume options at the front, stop at first non-option non-slice word
        let mut idx = 0;
        let mut valid_magic = true;
        while idx < remaining.len() {
            let arg = &remaining[idx];
            if let Some(chars) = long_opts.get(arg.as_str()) {
                magic_options.extend(chars.chars());
                idx += 1;
                continue;
            }
            // slice?
            if is_slice(arg) {
                slice_found = Some(arg.clone());
                idx += 1;
                continue;
            }
            if arg.starts_with("--") {
                // unknown long option → this is a parser name, not magic
                valid_magic = false;
                break;
            }
            if arg.starts_with('-') && arg.len() > 1 {
                // short option cluster
                let chars: Vec<char> = arg[1..].chars().collect();
                magic_options.extend(chars);
                idx += 1;
                continue;
            }
            // First non-option word → magic command starts here
            magic_start_idx = Some(idx);
            break;
        }

        if valid_magic && let Some(start) = magic_start_idx {
            let cmd_words: Vec<String> = remaining[start..].to_vec();
            if !cmd_words.is_empty() {
                args.magic_command = Some(cmd_words);
                magic_mode = true;
                option_chars.extend(&magic_options);
                if let Some(s) = slice_found {
                    args.slice_str = Some(s);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2, standard mode: if magic_mode didn't fire
    // -----------------------------------------------------------------------
    if !magic_mode {
        option_chars.clear();
        // reset slice
        args.slice_str = None;

        for arg in argv {
            // known long option?
            if let Some(chars) = long_opts.get(arg.as_str()) {
                option_chars.extend(chars.chars());
                continue;
            }
            // slice?
            if is_slice(arg) {
                args.slice_str = Some(arg.clone());
                continue;
            }
            // parser name: --word not in long opts
            if arg.starts_with("--") {
                // Strip leading '--', convert dashes to underscores for lookup
                let name = arg.trim_start_matches('-').to_string();
                args.parser_name = Some(name);
                continue;
            }
            // short option cluster
            if arg.starts_with('-') && arg.len() > 1 {
                option_chars.extend(arg[1..].chars());
                continue;
            }
            // otherwise: leftover arg
            args.extra_args.push(arg.clone());
        }
    }

    // -----------------------------------------------------------------------
    // Apply option chars to flags (counting for multi-flag options)
    // -----------------------------------------------------------------------
    let d_count = option_chars.iter().filter(|&&c| c == 'd').count();
    let h_count = option_chars.iter().filter(|&&c| c == 'h').count();
    let q_count = option_chars.iter().filter(|&&c| c == 'q').count();

    args.about = option_chars.contains(&'a');
    args.debug = d_count >= 1;
    args.verbose_debug = d_count >= 2;
    args.force_color = option_chars.contains(&'C');
    args.help = h_count >= 1;
    args.show_hidden = h_count >= 2;
    args.show_categories = h_count >= 3;
    args.mono = option_chars.contains(&'m');
    args.meta_out = option_chars.contains(&'M');
    args.pretty = option_chars.contains(&'p');
    args.quiet = q_count >= 1;
    args.ignore_exceptions = q_count >= 2;
    args.raw = option_chars.contains(&'r');
    args.slurp = option_chars.contains(&'s');
    args.unbuffer = option_chars.contains(&'u');
    args.version = option_chars.contains(&'v');
    args.yaml = option_chars.contains(&'y');
    args.bash_comp = option_chars.contains(&'B');
    args.zsh_comp = option_chars.contains(&'Z');
    args.fish_comp = option_chars.contains(&'F');
    args.list_parsers = option_chars.contains(&'l');
    args.list_all = option_chars.contains(&'L');

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strvec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_is_slice() {
        assert!(is_slice("1:5"));
        assert!(is_slice(":5"));
        assert!(is_slice("1:"));
        assert!(is_slice(":"));
        assert!(is_slice("-3:"));
        assert!(is_slice("-1:-1"));
        assert!(!is_slice("foo"));
        assert!(!is_slice("abc:def"));
    }

    #[test]
    fn test_parser_name() {
        let args = parse_args(&strvec(&["--df"]));
        assert_eq!(args.parser_name, Some("df".to_string()));
    }

    #[test]
    fn test_pretty_flag() {
        let args = parse_args(&strvec(&["-p", "--df"]));
        assert!(args.pretty);
        assert_eq!(args.parser_name, Some("df".to_string()));
    }

    #[test]
    fn test_magic_mode() {
        let args = parse_args(&strvec(&["-p", "ls", "-al"]));
        assert!(args.pretty);
        assert!(args.magic_command.is_some());
        let cmd = args.magic_command.unwrap();
        assert_eq!(cmd[0], "ls");
        assert_eq!(cmd[1], "-al");
    }

    #[test]
    fn test_double_debug() {
        let args = parse_args(&strvec(&["-dd", "--df"]));
        assert!(args.debug);
        assert!(args.verbose_debug);
    }

    #[test]
    fn test_slice() {
        let args = parse_args(&strvec(&["4:15", "--df"]));
        assert_eq!(args.slice_str, Some("4:15".to_string()));
    }

    // ── Additional args tests ─────────────────────────────────────────────────

    #[test]
    fn test_combined_short_flags_pq() {
        let args = parse_args(&strvec(&["-pq", "--df"]));
        assert!(args.pretty);
        assert!(args.quiet);
        assert!(!args.raw);
    }

    #[test]
    fn test_combined_short_flags_rq() {
        let args = parse_args(&strvec(&["-rq", "--df"]));
        assert!(args.raw);
        assert!(args.quiet);
        assert!(!args.pretty);
    }

    #[test]
    fn test_combined_short_flags_multiple() {
        let args = parse_args(&strvec(&["-pry", "--df"]));
        assert!(args.pretty);
        assert!(args.raw);
        assert!(args.yaml);
    }

    #[test]
    fn test_slice_both_endpoints() {
        let args = parse_args(&strvec(&["3:7", "--df"]));
        assert_eq!(args.slice_str, Some("3:7".to_string()));
    }

    #[test]
    fn test_slice_negative_only_end() {
        let args = parse_args(&strvec(&[":-3", "--df"]));
        assert_eq!(args.slice_str, Some(":-3".to_string()));
    }

    #[test]
    fn test_slice_negative_start() {
        let args = parse_args(&strvec(&["-3:", "--df"]));
        assert_eq!(args.slice_str, Some("-3:".to_string()));
    }

    #[test]
    fn test_long_option_about() {
        let args = parse_args(&strvec(&["--about"]));
        assert!(args.about);
    }

    #[test]
    fn test_long_option_version() {
        let args = parse_args(&strvec(&["--version"]));
        assert!(args.version);
    }

    #[test]
    fn test_long_option_list() {
        let args = parse_args(&strvec(&["--list"]));
        assert!(args.list_parsers);
    }

    #[test]
    fn test_long_option_list_all() {
        let args = parse_args(&strvec(&["--list-all"]));
        assert!(args.list_all);
    }

    #[test]
    fn test_parser_name_from_double_dash_prefix() {
        // --df-h → parser name "df-h" (dashes not converted)
        let args = parse_args(&strvec(&["--df-h"]));
        assert_eq!(args.parser_name, Some("df-h".to_string()));
    }

    #[test]
    fn test_triple_quiet_enables_ignore_exceptions() {
        let args = parse_args(&strvec(&["-qq", "--df"]));
        assert!(args.quiet);
        assert!(args.ignore_exceptions);
    }

    #[test]
    fn test_long_option_meta_out() {
        let args = parse_args(&strvec(&["--meta-out", "--df"]));
        assert!(args.meta_out);
    }

    #[test]
    fn test_long_option_pretty_and_yaml() {
        let args = parse_args(&strvec(&["--pretty", "--yaml-out", "--df"]));
        assert!(args.pretty);
        assert!(args.yaml);
    }

    #[test]
    fn test_magic_mode_with_multiple_flags() {
        let args = parse_args(&strvec(&["-p", "-r", "df", "-h"]));
        assert!(args.pretty);
        assert!(args.raw);
        assert!(args.magic_command.is_some());
        let cmd = args.magic_command.unwrap();
        assert_eq!(cmd[0], "df");
        assert_eq!(cmd[1], "-h");
    }

    #[test]
    fn test_is_slice_both_empty_parts() {
        assert!(is_slice(":"));
    }

    #[test]
    fn test_is_slice_rejects_non_numeric() {
        assert!(!is_slice("a:b"));
        assert!(!is_slice("1:b"));
        assert!(!is_slice("a:1"));
    }
}

/// End-to-end integration tests for the `jc-rs` CLI binary.
///
/// These tests invoke the compiled binary via `std::process::Command` and
/// verify exit codes, stdout content, and JSON/YAML validity.
///
/// `CARGO_MANIFEST_DIR` is `crates/jc-rs/` here, so workspace root is two levels up.
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root")
}

fn binary() -> PathBuf {
    workspace_root().join("target/debug/jc-rs")
}

fn read_fixture(platform: &str, name: &str) -> Vec<u8> {
    let path = workspace_root()
        .join("tests/fixtures")
        .join(platform)
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {platform}/{name}: {e}"))
}

/// Run `jc-rs` with args, optional stdin bytes. Returns (exit_code, stdout, stderr).
fn run(args: &[&str], stdin_data: Option<&[u8]>) -> (i32, String, String) {
    let mut cmd = Command::new(binary());
    cmd.args(args);

    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn jc-rs binary");

    if let Some(data) = stdin_data {
        child
            .stdin
            .take()
            .expect("stdin not captured")
            .write_all(data)
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait_with_output");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ─────────────────────────────────────────────
// 1. Basic CLI flags
// ─────────────────────────────────────────────

#[test]
fn version_exits_zero_and_prints_version() {
    let (code, out, _) = run(&["--version"], None);
    assert_eq!(code, 0, "exit code should be 0");
    // Read the version from the manifest rather than hard-coding it: a literal
    // here silently rots the moment the workspace version moves.
    let expected = env!("CARGO_PKG_VERSION");
    assert!(
        out.contains(expected),
        "expected version {expected}, got: {out}"
    );
}

#[test]
fn help_exits_zero_and_contains_usage() {
    let (code, out, _) = run(&["--help"], None);
    assert_eq!(code, 0);
    assert!(
        out.to_lowercase().contains("usage"),
        "expected 'Usage' in help output, got: {out}"
    );
}

#[test]
fn list_exits_zero_and_contains_known_parsers() {
    let (code, out, _) = run(&["--list"], None);
    assert_eq!(code, 0);
    assert!(out.contains("--df"), "--list should contain '--df'");
    assert!(out.contains("--arp"), "--list should contain '--arp'");
    assert!(out.contains("--ps"), "--list should contain '--ps'");
}

#[test]
fn about_exits_zero_and_returns_valid_json() {
    let (code, out, _) = run(&["--about"], None);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("--about output is not valid JSON: {e}\n{out}"));
    assert!(
        v.get("parser_count").is_some(),
        "expected 'parser_count' key in --about JSON"
    );
}

// ─────────────────────────────────────────────
// 2. Standard parsing via stdin
// ─────────────────────────────────────────────

#[test]
fn df_parser_produces_valid_json_array() {
    let input = read_fixture("centos-7.7", "df.out");
    let (code, out, err) = run(&["--df"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("df output is not valid JSON: {e}\n{out}"));
    let arr = v.as_array().expect("df output should be a JSON array");
    assert!(!arr.is_empty(), "df array should not be empty");
    assert!(
        arr[0].get("filesystem").is_some(),
        "expected 'filesystem' field in df entry"
    );
}

#[test]
fn arp_parser_produces_valid_json_array() {
    let input = read_fixture("centos-7.7", "arp.out");
    let (code, out, err) = run(&["--arp"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("arp output is not valid JSON: {e}\n{out}"));
    let arr = v.as_array().expect("arp output should be a JSON array");
    assert!(!arr.is_empty(), "arp array should not be empty");
}

#[test]
fn ps_parser_produces_large_json_array() {
    let input = read_fixture("centos-7.7", "ps-axu.out");
    let (code, out, err) = run(&["--ps"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("ps output is not valid JSON: {e}\n{out}"));
    let arr = v.as_array().expect("ps output should be a JSON array");
    assert!(
        arr.len() > 50,
        "expected many ps entries, got {}",
        arr.len()
    );
}

// ─────────────────────────────────────────────
// 3. Output format flags
// ─────────────────────────────────────────────

#[test]
fn pretty_flag_produces_indented_json() {
    let input = read_fixture("centos-7.7", "df.out");
    let (code, out, err) = run(&["--df", "--pretty"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        out.contains('\n'),
        "--pretty output should contain newlines"
    );
    assert!(
        out.contains("  "),
        "--pretty output should contain indentation"
    );
    // Still valid JSON
    let _: serde_json::Value =
        serde_json::from_str(&out).expect("--pretty output should be valid JSON");
}

#[test]
fn yaml_out_flag_produces_yaml() {
    let input = read_fixture("centos-7.7", "df.out");
    let (code, out, err) = run(&["--df", "--yaml-out"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    // YAML list starts with "- "
    assert!(
        out.contains("- "),
        "--yaml-out output should look like YAML list"
    );
    // Should NOT start with '[' (not JSON array)
    assert!(
        !out.trim_start().starts_with('['),
        "--yaml-out should not be JSON array syntax"
    );
}

#[test]
fn raw_flag_produces_compact_single_line_json() {
    let input = read_fixture("centos-7.7", "df.out");
    let (code, out, err) = run(&["--df", "--raw"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let stripped = out.trim();
    assert!(
        !stripped.contains('\n'),
        "--raw output should be a single line, got: {stripped}"
    );
    let _: serde_json::Value =
        serde_json::from_str(stripped).expect("--raw output should be valid JSON");
}

// ─────────────────────────────────────────────
// 4. Error handling
// ─────────────────────────────────────────────

#[test]
fn unknown_parser_exits_nonzero_with_error_message() {
    let (code, out, err) = run(&["--no-such-parser-xyz-99"], None);
    assert_ne!(code, 0, "unknown parser should exit non-zero");
    let combined = format!("{out}{err}");
    assert!(
        combined.to_lowercase().contains("unknown") || combined.to_lowercase().contains("error"),
        "expected error message for unknown parser, got: {combined}"
    );
}

#[test]
fn garbage_input_for_df_handles_gracefully() {
    let garbage = b"this is not df output\ngarbage line\n";
    let (code, out, _err) = run(&["--df"], Some(garbage));
    // Should either return empty JSON array or exit non-zero — not panic
    if code == 0 {
        let _: serde_json::Value = serde_json::from_str(&out)
            .unwrap_or_else(|e| panic!("garbage df output on exit 0 must be valid JSON: {e}"));
    }
    assert!(
        !out.contains("thread 'main' panicked"),
        "binary should not panic on garbage input"
    );
}

// ─────────────────────────────────────────────
// 5. Slicing
// ─────────────────────────────────────────────

#[test]
fn slice_flag_limits_output_count() {
    let input = read_fixture("centos-7.7", "ps-axu.out");
    // ps-axu.out has 109 entries; slicing 0:5 should give far fewer
    let (code, out, err) = run(&["--ps", "-s", "0:5"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("sliced ps output is not valid JSON: {e}\n{out}"));
    let arr = v.as_array().expect("sliced output should be array");
    assert!(
        arr.len() < 10,
        "slice 0:5 should return far fewer than 109 items, got {}",
        arr.len()
    );
    assert!(!arr.is_empty(), "sliced output should not be empty");
}

#[test]
fn slice_output_is_still_valid_json() {
    let input = read_fixture("centos-7.7", "arp.out");
    let (code, out, err) = run(&["--arp", "-s", "0:2"], Some(&input));
    assert_eq!(code, 0, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("sliced arp output is not valid JSON: {e}"));
    assert!(v.is_array(), "sliced output should be a JSON array");
}

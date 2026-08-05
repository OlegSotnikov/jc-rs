/// Tests for live streaming behaviour.
///
/// The differential corpus cannot catch any of this: its fixtures are stored as
/// arrays and it feeds the binary a file that ends immediately, so a parser that
/// buffers everything until EOF scores exactly the same as one that streams. The
/// only way to tell them apart is to hold the pipe open and watch.
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root")
        .join("target/debug/jc-rs")
}

const CLF_LINE: &str =
    r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;

/// Read lines from the child on another thread so a parser that never emits
/// cannot wedge the test.
fn reader_channel(stdout: ChildStdout) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

fn spawn(args: &[&str]) -> Child {
    Command::new(binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jc-rs")
}

#[test]
fn streaming_emits_records_before_the_input_closes() {
    let mut child = spawn(&["-u", "--clf-s"]);
    let mut stdin = child.stdin.take().expect("stdin");
    let records = reader_channel(child.stdout.take().expect("stdout"));

    // Write two lines and deliberately leave the pipe open, which is what
    // `tail -f` does. Both records must arrive anyway.
    writeln!(stdin, "{CLF_LINE}").expect("write first line");
    stdin.flush().expect("flush");
    let first = records
        .recv_timeout(Duration::from_secs(5))
        .expect("first record before EOF");

    writeln!(stdin, "{CLF_LINE}").expect("write second line");
    stdin.flush().expect("flush");
    let second = records
        .recv_timeout(Duration::from_secs(5))
        .expect("second record before EOF");

    for record in [&first, &second] {
        let value: serde_json::Value =
            serde_json::from_str(record).expect("each line is a complete JSON value");
        assert_eq!(value["host"], "127.0.0.1");
        assert_eq!(value["status"], 200);
    }

    drop(stdin);
    let _ = child.wait();
}

#[test]
fn streaming_output_is_ndjson_not_an_array() {
    let mut child = spawn(&["--clf-s"]);
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "{CLF_LINE}").expect("write");
    writeln!(stdin, "{CLF_LINE}").expect("write");
    drop(stdin);

    let out = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 2, "one JSON value per record");
    for line in lines {
        assert!(line.starts_with('{'), "each line is an object: {line}");
        serde_json::from_str::<serde_json::Value>(line).expect("valid JSON per line");
    }
    assert!(
        !stdout.trim_start().starts_with('['),
        "streaming output must not be wrapped in an array"
    );
}

#[test]
fn streaming_ignore_exceptions_marks_each_record() {
    let mut child = spawn(&["-qq", "--clf-s"]);
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "{CLF_LINE}").expect("write");
    drop(stdin);

    let out = child.wait_with_output().expect("wait");
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("valid JSON");
    assert_eq!(value["_jc_meta"]["success"], true);
}

#[test]
fn streaming_reports_a_bad_line_instead_of_dropping_it() {
    // `ls_s` rejects a line that is not `ls -l` output; under -qq that failure
    // has to surface as a record of its own.
    let mut child = spawn(&["-qq", "--ls-s"]);
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "not ls -l output at all").expect("write");
    drop(stdin);

    let out = child.wait_with_output().expect("wait");
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).expect("valid JSON");
    assert_eq!(value["_jc_meta"]["success"], false);
    assert_eq!(value["_jc_meta"]["line"], "not ls -l output at all");
    assert!(value["_jc_meta"]["error"].is_string());
}

#[test]
fn streaming_slice_stays_lazy_for_positive_ranges() {
    // `1:3` must start emitting once line 3 has been seen, without waiting for
    // the input to end.
    let mut child = spawn(&["-u", "1:3", "--clf-s"]);
    let mut stdin = child.stdin.take().expect("stdin");
    let records = reader_channel(child.stdout.take().expect("stdout"));

    for _ in 0..3 {
        writeln!(stdin, "{CLF_LINE}").expect("write");
    }
    stdin.flush().expect("flush");

    // Lines 1 and 2 (zero-based, end exclusive); line 0 is skipped.
    records
        .recv_timeout(Duration::from_secs(5))
        .expect("sliced record before EOF");
    records
        .recv_timeout(Duration::from_secs(5))
        .expect("second sliced record before EOF");

    drop(stdin);
    let _ = child.wait();
}

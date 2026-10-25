//! A reader that closes stdout early, as `knots -r src | head` does, ends the
//! report without a panic or an error, and the run exits with the status its
//! scan earns: 0 clean, 1 for a threshold violation, 3 for an incomplete scan.
//!
//! knots' stdout here is a pipe whose read end is closed before knots starts,
//! so its first write fails with EPIPE in every case. Unix only: there, Rust
//! ignores SIGPIPE, so the write returns the error instead of killing knots.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Runs knots from the repository root with stdout already closed.
fn knots_into_closed_pipe(args: &[&str]) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .args(args)
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const FORMATS: [&[&str]; 7] = [
    &[],
    &["-v"],
    &["--matrix"],
    &["--format", "json"],
    &["--format", "ndjson"],
    &["--format", "csv"],
    &["--format", "sarif"],
];

#[test]
fn a_clean_scan_exits_0_in_every_output_mode() {
    for format in FORMATS {
        let output = knots_into_closed_pipe(&[format, &["-r", "sample-files"]].concat());
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(0), "{format:?}: {err}");
        assert!(!err.contains("panicked"), "{format:?}: {err}");
        assert!(!err.contains("Broken pipe"), "{format:?}: {err}");
    }
}

#[test]
fn a_threshold_violation_still_exits_1_and_is_listed_on_stderr() {
    for format in FORMATS {
        let args = [format, &["-r", "sample-files", "--mccabe-threshold", "1"]].concat();
        let output = knots_into_closed_pipe(&args);
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(1), "{format:?}: {err}");
        assert!(err.contains("Threshold violations"), "{format:?}: {err}");
        assert!(!err.contains("panicked"), "{format:?}: {err}");
    }
}

#[test]
fn an_incomplete_scan_still_exits_3() {
    for format in FORMATS {
        let args = [format, &["-r", "tests/fixtures/incomplete/src"]].concat();
        let output = knots_into_closed_pipe(&args);
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(3), "{format:?}: {err}");
        assert!(!err.contains("panicked"), "{format:?}: {err}");
    }
}

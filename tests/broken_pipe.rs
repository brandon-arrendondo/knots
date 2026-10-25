//! A reader that closes stdout or stderr early, as `knots -r src | head` and
//! `knots -r src 2>&1 | head` do, ends that stream without a panic or an
//! error, and the run exits with the status its scan earns: 0 clean, 1 for a
//! threshold violation, 3 for an incomplete scan.
//!
//! Most tests hand knots a pipe whose read end is closed before knots starts,
//! so its first write fails with EPIPE in every case; one closes the pipe
//! after reading the start of the report. Unix only: there, Rust ignores
//! SIGPIPE, so the write returns the error instead of killing knots.
#![cfg(unix)]

use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};

fn knots() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knots"));
    command.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
    command
}

/// Runs knots from the repository root with stdout already closed.
fn knots_into_closed_pipe(args: &[&str]) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    knots()
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

/// Runs knots with stdout and stderr both writing to one closed pipe, as
/// `2>&1 | head` leaves them once head exits. A panic would exit 101.
fn knots_with_both_streams_closed(args: &[&str]) -> ExitStatus {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    knots()
        .args(args)
        .stdout(Stdio::from(writer.try_clone().unwrap()))
        .stderr(Stdio::from(writer))
        .status()
        .unwrap()
}

#[test]
fn closed_stdout_and_stderr_keep_the_scans_status() {
    for format in FORMATS {
        for (args, code) in [
            (&["-r", "sample-files"][..], 0),
            (&["-r", "sample-files", "--mccabe-threshold", "1"], 1),
            (&["-r", "tests/fixtures/incomplete/src"], 3),
        ] {
            let status = knots_with_both_streams_closed(&[format, args].concat());
            assert_eq!(status.code(), Some(code), "{format:?} {args:?}");
        }
    }
}

#[test]
fn a_closed_stderr_alone_keeps_the_scans_status_and_the_report() {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let output = knots()
        .args(["-r", "sample-files", "--mccabe-threshold", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::from(writer))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("WORST FUNCTIONS"));
}

/// The report is larger than a pipe's buffer, so knots is still writing when
/// the reader closes after its first bytes.
#[test]
fn a_reader_that_closes_mid_report_leaves_the_status_unchanged() {
    for format in [
        &["--format", "json"][..],
        &["--format", "ndjson"],
        &["--format", "csv"],
        &["--matrix"],
    ] {
        let mut child = knots()
            .args([format, &["-r", "src"]].concat())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut start = [0u8; 64];
        child.stdout.take().unwrap().read_exact(&mut start).unwrap();
        let output = child.wait_with_output().unwrap();
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(0), "{format:?}: {err}");
        assert!(!err.contains("panicked"), "{format:?}: {err}");
        assert!(!err.contains("Broken pipe"), "{format:?}: {err}");
    }
}

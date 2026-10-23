//! A scan that could not analyse a selected file exits 3 and names the file,
//! with or without a gate, and never passes as clean (docs/adr/0005).
//!
//! Fixtures, under tests/fixtures/incomplete:
//! - src/good.c, one function;
//! - src/bad.c, a syntax error that yields no function: recovered, not lost;
//! - src/binary.c, not UTF-8: lost at the read;
//! - named/A.Z, an extension no language claims: lost when named;
//! - scope/a.py, Python, which `-l c` leaves out of scope: not lost;
//! - coupling/ and stem/, an include tree whole and with one file lost;
//! - compile/, a compile_commands.json with an entry that is not on disk.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/incomplete")
}

fn knots(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture())
        .args(args)
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A path in the temp directory for this test alone, removed first.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("knots-{}-{name}", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

/// Runs `args` with `--completion-report` and returns the report.
fn completion_report(name: &str, args: &[&str]) -> (Output, serde_json::Value) {
    let path = scratch(name);
    let mut all = args.to_vec();
    let path_arg = path.to_str().unwrap();
    all.extend(["--completion-report", path_arg]);
    let output = knots(&all);
    let report = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    fs::remove_file(&path).unwrap();
    (output, report)
}

#[test]
fn an_unreadable_file_makes_the_scan_incomplete_with_or_without_a_gate() {
    for args in [
        vec!["-r", "src"],
        vec!["-r", "src", "--mccabe-threshold=15"],
    ] {
        let output = knots(&args);
        assert_eq!(output.status.code(), Some(3), "{args:?}");
        let err = stderr(&output);
        assert!(
            err.contains("Not analysed (read): src/binary.c"),
            "{args:?}: {err}"
        );
        assert!(
            err.trim_end()
                .ends_with("Incomplete scan: 1 of 3 selected file(s) were not analysed"),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn an_incomplete_scan_outranks_a_violation_and_still_lists_it() {
    let output = knots(&["-r", "src", "--mccabe-threshold=1"]);
    assert_eq!(output.status.code(), Some(3));
    let err = stderr(&output);
    assert!(err.contains("src/good.c:1:ok"), "{err}");
    assert!(
        err.contains("1 function(s) exceeded complexity thresholds"),
        "{err}"
    );
}

#[test]
fn the_files_summary_always_prints_its_counts() {
    let output = knots(&["src/good.c", "src/bad.c"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    for line in [
        "Total files selected: 2",
        "Analysed: 2",
        "Not analysed: 0",
        "Syntax errors, no functions recovered: 1",
    ] {
        assert!(out.contains(line), "{line}: {out}");
    }
    let single = stdout(&knots(&["src/good.c"]));
    assert!(
        single.contains("Files: 1 selected, 1 analysed, 0 not analysed"),
        "{single}"
    );
}

#[test]
fn a_syntax_error_file_with_no_functions_is_recovered_not_lost() {
    let (output, report) = completion_report("recovered", &["src/bad.c", "--mccabe-threshold=15"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("src/bad.c has syntax errors"));
    assert_eq!(report["complete"], true);
    assert_eq!(report["recovered_no_functions"][0], "src/bad.c");
}

#[test]
fn a_named_file_knots_cannot_analyse_is_lost() {
    for args in [vec!["named/A.Z"], vec!["named/A.Z", "src/good.c"]] {
        let (output, report) = completion_report("named", &args);
        assert_eq!(output.status.code(), Some(3), "{args:?}");
        assert_eq!(report["not_analysed"][0]["file"], "named/A.Z");
        assert_eq!(report["not_analysed"][0]["stage"], "unsupported");
    }
}

#[test]
fn a_named_path_that_does_not_exist_is_lost() {
    for args in [vec!["gone.c"], vec!["gone.c", "src/good.c"]] {
        let (output, report) = completion_report("gone", &args);
        assert_eq!(output.status.code(), Some(3), "{args:?}");
        assert_eq!(report["not_analysed"][0]["file"], "gone.c");
        assert_eq!(report["not_analysed"][0]["stage"], "missing");
    }
}

#[test]
fn a_named_file_outside_the_language_scope_is_not_lost() {
    let output = knots(&["-l", "c", "scope/a.py", "src/good.c"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn a_missing_compile_commands_entry_is_lost() {
    let path = scratch("compile");
    let output = Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture().join("compile"))
        .args(["--compile-commands", "compile_commands.json"])
        .args(["--completion-report", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(report["not_analysed"][0]["file"], "./gone.c");
    assert_eq!(report["not_analysed"][0]["stage"], "missing");
    assert_eq!(report["files"]["analysed"], 1);
}

#[test]
fn write_baseline_refuses_an_incomplete_scan_and_leaves_the_file_alone() {
    let path = scratch("baseline");
    fs::write(&path, "previous").unwrap();
    let output = knots(&[
        "-r",
        "src",
        "--baseline",
        path.to_str().unwrap(),
        "--write-baseline",
    ]);
    assert_eq!(output.status.code(), Some(3));
    assert!(stderr(&output).contains("Refusing to write a baseline from an incomplete scan"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "previous");
    fs::remove_file(&path).unwrap();
}

#[test]
fn every_format_writes_the_completion_report_and_keeps_its_stdout() {
    for format in ["text", "json", "ndjson", "csv", "sarif"] {
        let (output, report) = completion_report(format, &["-r", "src", "--format", format]);
        assert_eq!(output.status.code(), Some(3), "{format}");
        assert_eq!(report["complete"], false, "{format}");
        let files = &report["files"];
        assert_eq!(
            (
                &files["selected"],
                &files["analysed"],
                &files["not_analysed"]
            ),
            (&3.into(), &2.into(), &1.into()),
            "{format}"
        );
        assert_eq!(report["not_analysed"][0]["file"], "src/binary.c");
        assert_eq!(report["not_analysed"][0]["stage"], "read");
    }
    let json: serde_json::Value =
        serde_json::from_slice(&knots(&["-r", "src", "--format", "json"]).stdout).unwrap();
    assert!(json.is_array(), "JSON output stays a bare array");
}

#[test]
fn sarif_reports_the_incomplete_scan_in_its_invocation() {
    let output = knots(&["-r", "src", "--format", "sarif"]);
    let sarif: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let invocation = &sarif["runs"][0]["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], false);
    let ids: Vec<_> = invocation["toolExecutionNotifications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["descriptor"]["id"].clone(), n["level"].clone()))
        .collect();
    assert_eq!(
        ids,
        [
            ("knots/input-not-analysed".into(), "error".into()),
            ("knots/no-functions-recovered".into(), "warning".into())
        ]
    );
}

#[test]
fn a_complete_scan_reports_complete_and_exits_zero() {
    let (output, report) = completion_report("complete", &["src/good.c", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report["complete"], true);
    assert_eq!(report["not_analysed"], serde_json::json!([]));
}

/// Each function's `file_ce`, by file name, from an ndjson scan of `dir`.
fn file_ce(dir: &str) -> Vec<(String, u64)> {
    let output = knots(&["-r", dir, "--format", "ndjson"]);
    let mut rows: Vec<(String, u64)> = stdout(&output)
        .lines()
        .map(|line| {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            let file = Path::new(row["file"].as_str().unwrap())
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            (file, row["file_ce"].as_u64().unwrap())
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn a_lost_include_target_keeps_its_importers_ce() {
    // main.c includes b.h; losing b.h must not drop the edge.
    let whole = file_ce("coupling/whole");
    let lost = file_ce("coupling/lost");
    assert!(whole.contains(&("main.c".into(), 1)), "{whole:?}");
    assert_eq!(lost, [("main.c".into(), 1)]);
}

#[test]
fn a_lost_file_still_makes_its_stem_ambiguous() {
    // util.c and util.h share a stem, so `#include "util.h"` is ambiguous;
    // losing util.c must not make it resolve.
    let whole = file_ce("stem/whole");
    let lost = file_ce("stem/lost");
    assert!(whole.contains(&("main.c".into(), 0)), "{whole:?}");
    assert_eq!(lost, [("main.c".into(), 0)]);
}

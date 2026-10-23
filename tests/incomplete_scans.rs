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
fn a_named_path_that_does_not_exist_is_an_invocation_error() {
    for args in [vec!["gone.c"], vec!["src/good.c", "gone.c"]] {
        let path = scratch("gone");
        let mut all = args.clone();
        all.extend(["--completion-report", path.to_str().unwrap()]);
        let output = knots(&all);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(stderr(&output).contains("Path 'gone.c' does not exist"));
        assert!(
            !path.exists(),
            "{args:?}: a run that cannot start writes no report"
        );
    }
}

#[test]
fn a_directory_named_without_recursive_is_an_invocation_error() {
    for args in [vec!["src"], vec!["src/good.c", "src"]] {
        let path = scratch("directory");
        let mut all = args.clone();
        all.extend(["--completion-report", path.to_str().unwrap()]);
        let output = knots(&all);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(stderr(&output).contains("Path 'src' is a directory"));
        assert!(
            !path.exists(),
            "{args:?}: a run that cannot start writes no report"
        );
    }
}

#[test]
fn an_excluded_named_file_is_not_lost() {
    for args in [
        vec!["--exclude-path", "A\\.Z", "named/A.Z"],
        vec![
            "--exclude",
            "named/exclude-z.json",
            "named/A.Z",
            "src/good.c",
        ],
    ] {
        let output = knots(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn a_file_knots_toml_excludes_is_not_read() {
    // toml/knots.toml excludes toml/src/binary.c, which cannot be read.
    let output = Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture().join("toml"))
        .args(["-r", "src"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!stderr(&output).contains("binary.c"));
}

#[test]
fn every_selected_file_lost_still_writes_sarif_and_refuses_the_baseline() {
    let output = knots(&["named/A.Z", "--format", "sarif"]);
    assert_eq!(output.status.code(), Some(3));
    let sarif: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let invocation = &sarif["runs"][0]["invocations"][0];
    assert_eq!(invocation["exitCode"], 3);
    assert_eq!(
        invocation["toolExecutionNotifications"][0]["descriptor"]["id"],
        "knots/input-not-analysed"
    );
    let path = scratch("never-written");
    let output = knots(&[
        "named/A.Z",
        "--baseline",
        path.to_str().unwrap(),
        "--write-baseline",
    ]);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("was not written"),
        "{}",
        stderr(&output)
    );
    assert!(!path.exists());
}

#[test]
fn sarif_carries_the_exit_status() {
    let exit_code = |args: &[&str]| {
        let output = knots(args);
        let sarif: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let code = sarif["runs"][0]["invocations"][0]["exitCode"].clone();
        (output.status.code(), code)
    };
    let sarif = ["--format", "sarif"];
    assert_eq!(
        exit_code(&[&["src/good.c"][..], &sarif].concat()),
        (Some(0), 0.into())
    );
    assert_eq!(
        exit_code(&[&["src/good.c", "--mccabe-threshold=1"][..], &sarif].concat()),
        (Some(1), 1.into())
    );
    assert_eq!(
        exit_code(&[&["-r", "src"][..], &sarif].concat()),
        (Some(3), 3.into())
    );
}

/// A tree with one readable file, a dangling `.c` symlink, a dangling `.txt`
/// symlink and a directory knots cannot read, built at run time because a
/// checkout cannot carry either.
#[cfg(unix)]
fn unreachable_tree(name: &str) -> PathBuf {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("knots-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("locked")).unwrap();
    fs::write(root.join("ok.c"), "int ok(void){ return 0; }\n").unwrap();
    fs::write(root.join("locked/h.c"), "int h(void){ return 0; }\n").unwrap();
    symlink("nowhere.c", root.join("dangling.c")).unwrap();
    symlink("nowhere.txt", root.join("dangling.txt")).unwrap();
    set_mode(&root.join("locked"), 0o000);
    root
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Runs knots in `dir`.
fn knots_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

/// (file, stage) for each entry of a completion report's `not_analysed`.
fn lost_entries(report: &serde_json::Value) -> Vec<(String, String)> {
    report["not_analysed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["file"].as_str().unwrap().into(),
                l["stage"].as_str().unwrap().into(),
            )
        })
        .collect()
}

/// Scans `root` with a completion report, then with the unreachable paths
/// excluded, and removes the tree.
#[cfg(unix)]
fn scan_unreachable_tree(root: &Path) -> (Output, serde_json::Value, Output) {
    let report_path = root.with_extension("json");
    let report_arg = report_path.to_str().unwrap();
    let output = knots_in(root, &["-r", ".", "--completion-report", report_arg]);
    let excluded = [
        "-r",
        ".",
        "--exclude-path",
        "locked",
        "--exclude-path",
        "dangling",
    ];
    let excluded = knots_in(root, &excluded);
    set_mode(&root.join("locked"), 0o755);
    let report = serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(&report_path).unwrap();
    (output, report, excluded)
}

#[cfg(unix)]
#[test]
fn what_the_walk_cannot_reach_is_lost() {
    let root = unreachable_tree("walk");
    // Root can still read the locked directory, so then it is not lost.
    let readable = fs::read_dir(root.join("locked")).is_ok();
    let (output, report, excluded) = scan_unreachable_tree(&root);

    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let mut expected = vec![("./dangling.c".to_string(), "walk".to_string())];
    if !readable {
        expected.push(("./locked".into(), "walk".into()));
    }
    assert_eq!(lost_entries(&report), expected);
    assert_eq!(excluded.status.code(), Some(0), "{}", stderr(&excluded));
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
fn compile_commands_entries_obey_the_language_and_path_scope() {
    let dir = fixture().join("compile");
    let excluded = knots_in(
        &dir,
        &[
            "--compile-commands",
            "compile_commands.json",
            "--exclude-path",
            "gone",
        ],
    );
    assert_eq!(excluded.status.code(), Some(0), "{}", stderr(&excluded));
    let other_language = knots_in(
        &dir,
        &[
            "--compile-commands",
            "compile_commands.json",
            "-l",
            "python",
        ],
    );
    assert_eq!(
        other_language.status.code(),
        Some(0),
        "{}",
        stderr(&other_language)
    );
    assert!(
        !stdout(&other_language).contains("ok"),
        "good.c is out of scope"
    );
}

#[test]
fn a_run_with_no_analysable_file_says_so_and_shows_its_counts() {
    let output = knots(&["named/A.Z"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stdout(&output).contains("Analysed: 0"),
        "{}",
        stdout(&output)
    );
    assert!(stderr(&output).contains("No selected file could be analysed"));
}

#[test]
fn sarif_records_exit_1_when_the_completion_report_cannot_be_written() {
    let unwritable = std::env::temp_dir().join("knots-no-such-dir/report.json");
    let output = knots(&[
        "src/good.c",
        "--format",
        "sarif",
        "--completion-report",
        unwritable.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let sarif: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["runs"][0]["invocations"][0]["exitCode"], 1);
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

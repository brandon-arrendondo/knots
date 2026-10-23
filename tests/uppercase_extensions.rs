//! Upper-case extensions (`A.C`, `B.H`), as Windows-built source trees carry
//! them, are C in every discovery mode, with or without a language filter, and
//! an `#include` that spells one keeps its coupling edge.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/uppercase")
}

/// Each reported function's name, keyed by its file name, with its file's Ce.
fn functions(args: &[&str]) -> BTreeMap<String, (String, u64)> {
    let output = Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture())
        .args(args)
        .args(["--format", "ndjson"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(function_row)
        .collect()
}

/// One ndjson row as (file name, (function, file Ce)).
fn function_row(line: &str) -> (String, (String, u64)) {
    let row: serde_json::Value = serde_json::from_str(line).unwrap();
    let file = Path::new(row["file"].as_str().unwrap())
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let name = row["function"].as_str().unwrap().to_owned();
    (file, (name, row["file_ce"].as_u64().unwrap()))
}

#[test]
fn upper_case_c_files_are_found_with_and_without_a_language_filter() {
    for args in [vec!["-r", "up"], vec!["--language", "c", "-r", "up"]] {
        let found = functions(&args);
        assert_eq!(
            found.keys().map(String::as_str).collect::<Vec<_>>(),
            ["A.C", "B.H"],
            "{args:?}"
        );
        // `#include "B.H"` resolves to B.H, so A.C depends on one file.
        assert_eq!(found["A.C"], ("uses_helper".to_owned(), 1), "{args:?}");
        assert_eq!(found["B.H"], ("helper".to_owned(), 0), "{args:?}");
    }
}

#[test]
fn an_upper_case_c_file_named_explicitly_is_analysed() {
    for args in [vec!["up/A.C"], vec!["--language", "c", "up/A.C"]] {
        let found = functions(&args);
        assert_eq!(found["A.C"].0, "uses_helper", "{args:?}");
    }
}

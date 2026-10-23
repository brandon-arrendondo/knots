//! A threshold set with a structured `--format` gates the run exactly as it
//! does in text mode, while stdout stays one well-formed document.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scan")
}

fn knots(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn a_violation_fails_the_run_in_every_structured_format() {
    for format in ["json", "ndjson", "csv", "sarif"] {
        let output = knots(&["-r", "src", "--format", format, "--mccabe-threshold=1"]);
        assert_eq!(output.status.code(), Some(1), "{format}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("src/good.c:1:ok"), "{format}: {stderr}");
        assert!(
            !output.stdout.is_empty(),
            "{format}: the document is still written"
        );
    }
}

#[test]
fn a_structured_document_with_a_violation_still_parses() {
    let output = knots(&["-r", "src", "--format", "json", "--mccabe-threshold=1"]);
    let records: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(records[0]["function"], "ok");
}

#[test]
fn no_violation_passes_in_every_structured_format() {
    for format in ["json", "ndjson", "csv", "sarif"] {
        let output = knots(&["-r", "src", "--format", format, "--mccabe-threshold=15"]);
        assert_eq!(output.status.code(), Some(0), "{format}");
    }
}

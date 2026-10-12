//! Exercise JSON excludes through recursive, explicit and compilation-database discovery.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/excludes")
}

fn functions(filter: &str, discovery: &[&str]) -> BTreeSet<(String, String)> {
    let output = Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture())
        .args(discovery)
        .args(["--exclude", filter, "--format", "ndjson"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(function_identity)
        .collect()
}

fn function_identity(line: &str) -> (String, String) {
    let row: serde_json::Value = serde_json::from_str(line).unwrap();
    let file = Path::new(row["file"].as_str().unwrap())
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (file, row["function"].as_str().unwrap().to_owned())
}

fn expected(names: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    names
        .iter()
        .map(|(file, name)| (file.to_string(), name.to_string()))
        .collect()
}

#[test]
fn function_names_keep_files_for_all_discovery_modes() {
    for filter in ["names.json", "names-empty-files.json"] {
        for discovery in [
            vec!["-r", "."],
            vec!["--compile-commands", "compile_commands.json"],
        ] {
            assert_eq!(
                functions(filter, &discovery),
                expected(&[("first.c", "keep_complex"), ("second.c", "keep_other"),])
            );
        }
        assert_eq!(
            functions(filter, &["first.c"]),
            expected(&[("first.c", "keep_complex")])
        );
    }
}

#[test]
fn complexity_only_exclude_keeps_files_and_filters_functions() {
    assert_eq!(
        functions("bounds.json", &["-r", "."]),
        expected(&[("first.c", "drop_simple"), ("second.c", "keep_other"),])
    );
}

#[test]
fn function_name_and_complexity_exclusions_require_both() {
    assert_eq!(
        functions("names-and-bounds.json", &["-r", "."]),
        expected(&[
            ("first.c", "drop_simple"),
            ("first.c", "keep_complex"),
            ("second.c", "keep_other"),
        ])
    );
}

#[test]
fn empty_exclude_is_a_noop() {
    for filter in ["empty.json", "empty-arrays.json"] {
        assert_eq!(
            functions(filter, &["-r", "."]),
            expected(&[
                ("first.c", "drop_simple"),
                ("first.c", "drop_complex"),
                ("first.c", "keep_complex"),
                ("second.c", "keep_other"),
            ])
        );
    }
}

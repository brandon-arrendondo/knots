//! A `[[filter.exclude]]` pattern that does not compile refuses the config,
//! naming the entry, the pattern and the parser's error, instead of matching
//! nothing and leaving its exclusion silently off.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/exclude-patterns")
        .join(name)
}

fn run(name: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture(name))
        .args(["a.c", "--format", "ndjson"])
        .output()
        .unwrap()
}

#[test]
fn invalid_glob_refuses_the_config() {
    let out = run("bad-glob");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(
            "[[filter.exclude]] entry 2: file_patterns: `vendor/[a-` is not a valid glob"
        ),
        "{stderr}"
    );
    assert!(stderr.contains("unclosed character class"), "{stderr}");
    assert!(out.stdout.is_empty(), "nothing is analysed");
}

#[test]
fn invalid_regex_refuses_the_config() {
    let out = run("bad-regex");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(
            "[[filter.exclude]] entry 1: function_patterns: `^generated_(` is not a valid regular expression"
        ),
        "{stderr}"
    );
    assert!(stderr.contains("unclosed group"), "{stderr}");
}

#[test]
fn valid_patterns_exclude_what_they_match() {
    let out = run("valid");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let names: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let row: serde_json::Value = serde_json::from_str(l).unwrap();
            row["function"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(names, ["keep"]);
}

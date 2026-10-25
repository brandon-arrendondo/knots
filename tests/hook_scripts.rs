//! The shell hooks in hooks/ gate every metric they offer a threshold for,
//! ABC included: they read knots' `--format ndjson` records, so a change to
//! the text report's layout cannot switch a threshold off unnoticed.
//!
//! Each hook runs under `sh` against sample-files/nested.c, whose functions
//! have ABC magnitudes of 6.40, 6.40, 5.92 and 3.00. Unix only: the hooks are
//! POSIX sh scripts.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const KNOTS: &str = env!("CARGO_BIN_EXE_knots");

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs pre-commit-wrapper.sh from the repository root.
fn wrapper(args: &[&str]) -> Output {
    Command::new("sh")
        .arg(repo_root().join("hooks/pre-commit-wrapper.sh"))
        .arg(format!("--validator-path={KNOTS}"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .unwrap()
}

/// A git repository in the temp directory with nested.c staged.
fn staged_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knots-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::copy(
        repo_root().join("sample-files/nested.c"),
        dir.join("nested.c"),
    )
    .unwrap();
    for args in [&["init", "-q"][..], &["add", "nested.c"]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
    dir
}

/// Runs a staged-file hook in `dir`. Every threshold is set in the
/// environment, which the hooks read before git config, so the user's own
/// hooks.knots.* settings cannot change the result.
fn staged_hook(hook: &str, dir: &Path, abc: &str) -> Output {
    Command::new("sh")
        .arg(repo_root().join("hooks").join(hook))
        .current_dir(dir)
        .env("VALIDATOR_PATH", KNOTS)
        .env("MCCABE_THRESHOLD", "99")
        .env("COGNITIVE_THRESHOLD", "99")
        .env("NESTING_THRESHOLD", "99")
        .env("SLOC_THRESHOLD", "999")
        .env("ABC_THRESHOLD", abc)
        .env("RETURN_THRESHOLD", "99")
        .env("VERBOSE", "false")
        .output()
        .unwrap()
}

#[test]
fn wrapper_fails_an_abc_breach_with_or_without_verbose() {
    for verbose in [false, true] {
        let mut args = vec![
            "--abc-threshold",
            "6.0",
            "--return-threshold=9",
            "sample-files/nested.c",
        ];
        if verbose {
            args.insert(0, "--verbose");
        }
        let output = wrapper(&args);
        let out = stdout(&output);
        assert_eq!(output.status.code(), Some(1), "verbose={verbose}: {out}");
        assert!(out.contains("  Function: sample-files/nested.c:7:deeply_nested\n  ABC Magnitude: 6.40 (threshold: 6.0)\n"), "verbose={verbose}: {out}");
        assert!(
            out.contains("Found 2 complexity violation(s)"),
            "verbose={verbose}: {out}"
        );
    }
}

#[test]
fn wrapper_lists_breaches_in_source_order() {
    let output = wrapper(&[
        "--mccabe-threshold=4",
        "--abc-threshold=99",
        "sample-files/nested.c",
    ]);
    let out = stdout(&output);
    let lines: Vec<usize> = ["nested.c:7:", "nested.c:25:", "nested.c:42:"]
        .iter()
        .map(|label| {
            out.find(label)
                .unwrap_or_else(|| panic!("{label} missing: {out}"))
        })
        .collect();
    assert!(lines.windows(2).all(|w| w[0] < w[1]), "{out}");
}

#[test]
fn wrapper_passes_below_the_abc_threshold() {
    let output = wrapper(&[
        "--abc-threshold",
        "6.5",
        "--return-threshold=9",
        "sample-files/nested.c",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn pre_commit_rejects_an_abc_breach_and_names_each_function_once() {
    let dir = staged_repo("hook-pre-commit");
    let output = staged_hook("pre-commit", &dir, "6.0");
    let out = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{out}");
    assert!(
        out.contains("✗ nested.c:7:deeply_nested\x1b[0m\n  ABC Magnitude: 6.40 (threshold: 6.0)\n"),
        "{out}"
    );
    assert!(!out.contains("nested.c:nested.c"), "{out}");

    let output = staged_hook("pre-commit", &dir, "6.5");
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn warning_only_hook_reports_an_abc_breach_and_still_passes() {
    let dir = staged_repo("hook-warning-only");
    let output = staged_hook("pre-commit-warning-only", &dir, "6.0");
    let out = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(
        out.contains("⚠ nested.c:25:flattened\x1b[0m\n  ABC Magnitude: 6.40 (threshold: 6.0)\n"),
        "{out}"
    );
    assert!(out.contains("Found 2 complexity warning(s)"), "{out}");
    fs::remove_dir_all(&dir).unwrap();
}

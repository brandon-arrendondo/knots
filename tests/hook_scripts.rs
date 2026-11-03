//! The shell hooks in hooks/ gate every metric they offer a threshold for,
//! ABC included: they read knots' `--format ndjson` records, so a change to
//! the text report's layout cannot switch a threshold off unnoticed.
//!
//! Each hook runs under `sh` against sample-files/nested.c, whose functions
//! have ABC magnitudes of 6.40, 6.40, 5.92 and 3.00. Unix only: the hooks are
//! POSIX sh scripts.
//!
//! Every git and hook process starts from [`isolated`], which strips the
//! `GIT_*` variables git exports to its own hooks. `cargo test` runs as a
//! pre-commit hook here, and in a linked worktree (a commit or cherry-pick
//! there) git exports GIT_DIR and GIT_INDEX_FILE: inherited, they point
//! `git init` and `git add` in a temp directory at the outer repository.
#![cfg(unix)]

use std::collections::BTreeSet;
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

/// A command with every inherited `GIT_*` variable removed, so git and the
/// hooks act on the repository in their working directory and nothing else.
fn isolated(program: &str) -> Command {
    let mut cmd = Command::new(program);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

/// Runs pre-commit-wrapper.sh from the repository root.
fn wrapper(args: &[&str]) -> Output {
    isolated("sh")
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
    for args in [
        &["-c", "init.defaultBranch=main", "init", "-q"][..],
        &["add", "nested.c"],
    ] {
        let status = isolated("git")
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
    isolated("sh")
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

/// deeply_nested's ABC magnitude is 6.403..., shown as 6.40. The wrapper
/// compares the unrounded value, as knots does, so a threshold between the
/// two gates in both.
#[test]
fn wrapper_compares_abc_as_knots_does() {
    for (threshold, code) in [("6.403", 1), ("6.41", 0)] {
        let args = ["--abc-threshold", threshold, "--return-threshold=9"];
        let knots = isolated(KNOTS)
            .args(args)
            .arg("sample-files/nested.c")
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(knots.status.code(), Some(code), "knots at {threshold}");
        let output = wrapper(&[&args[..], &["sample-files/nested.c"]].concat());
        let out = stdout(&output);
        assert_eq!(output.status.code(), Some(code), "{threshold}: {out}");
        if code == 1 {
            assert!(
                out.contains(&format!("ABC Magnitude: 6.40 (threshold: {threshold})")),
                "{out}"
            );
        }
    }
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

/// Not a test on its own: the body the next test runs in a child process
/// with git's hook variables pointing at a sentinel repository.
#[test]
#[ignore = "run by staged_repo_leaves_an_outer_repository_untouched"]
fn staged_repo_probe() {
    let dir = staged_repo("isolation-probe");
    let output = staged_hook("pre-commit", &dir, "6.0");
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    fs::remove_dir_all(&dir).unwrap();
}

/// Runs git in `dir`, isolated, and returns its trimmed stdout.
fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = isolated("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Runs the ignored probe test in a child process with GIT_DIR,
/// GIT_INDEX_FILE and GIT_WORK_TREE naming `repo`, as git exports them to
/// a hook in a linked worktree.
fn run_probe_inside(repo: &Path) -> Output {
    let git_dir = repo.join(".git");
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "staged_repo_probe",
            "--ignored",
            "--test-threads=1",
        ])
        .env("GIT_DIR", &git_dir)
        .env("GIT_INDEX_FILE", git_dir.join("index"))
        .env("GIT_WORK_TREE", repo)
        .output()
        .unwrap()
}

/// Runs the probe as git would run `cargo test` from a hook in a linked
/// worktree, and checks that the repository git named is left as it was.
#[test]
fn staged_repo_leaves_an_outer_repository_untouched() {
    let sentinel = std::env::temp_dir().join(format!("knots-{}-sentinel", std::process::id()));
    let _ = fs::remove_dir_all(&sentinel);
    fs::create_dir_all(&sentinel).unwrap();
    git_in(&sentinel, &["-c", "init.defaultBranch=main", "init", "-q"]);
    let config_before = fs::read_to_string(sentinel.join(".git/config")).unwrap();

    let probe = run_probe_inside(&sentinel);
    let out = format!(
        "{}{}",
        stdout(&probe),
        String::from_utf8_lossy(&probe.stderr)
    );
    assert!(probe.status.success() && out.contains("1 passed"), "{out}");

    let config_after = fs::read_to_string(sentinel.join(".git/config")).unwrap();
    assert_eq!(config_after, config_before, "the sentinel's config changed");
    let staged = git_in(&sentinel, &["ls-files", "--cached"]);
    assert_eq!(staged, "", "the sentinel's index changed");
    fs::remove_dir_all(&sentinel).unwrap();
}

/// A git repository in the temp directory with `files` written and staged.
fn repo_with(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knots-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["-c", "init.defaultBranch=main", "init", "-q"]);
    for (file, content) in files {
        fs::write(dir.join(file), content).unwrap();
        git(&dir, &["add", file]);
    }
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let status = isolated("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// Runs a staged-file hook in `dir` with knots at `validator` and every
/// threshold set in the environment, McCabe at 3.
fn hook_with(hook: &str, dir: &Path, validator: &str) -> Output {
    isolated("sh")
        .arg(repo_root().join("hooks").join(hook))
        .current_dir(dir)
        .env("VALIDATOR_PATH", validator)
        .env("MCCABE_THRESHOLD", "3")
        .env("COGNITIVE_THRESHOLD", "99")
        .env("NESTING_THRESHOLD", "99")
        .env("SLOC_THRESHOLD", "999")
        .env("ABC_THRESHOLD", "99")
        .env("RETURN_THRESHOLD", "99")
        .env("VERBOSE", "false")
        .output()
        .unwrap()
}

fn all_output(output: &Output) -> String {
    format!(
        "{}{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    )
}

const OK_C: &[u8] = b"int f(int a) { return a; }\n";
/// Not UTF-8, so knots cannot read it and the scan is incomplete (exit 3).
const UNREADABLE_C: &[u8] = b"\xff\xfe int f(void) { return 1; }\n";
const NO_KNOTS: &str = "/nonexistent/knots";

fn registry_extensions() -> BTreeSet<String> {
    knots::languages()
        .iter()
        .flat_map(|l| l.extensions.iter().chain(l.explicit_only.iter()))
        .map(|e| e.to_ascii_lowercase())
        .collect()
}

/// The extensions in a hook script's `SOURCE_EXT='\.(a|b|...)$'` line.
fn hook_extensions(text: &str) -> Option<BTreeSet<String>> {
    let pattern = text
        .lines()
        .find_map(|l| l.strip_prefix("SOURCE_EXT='"))?
        .strip_suffix('\'')?;
    let inner = pattern.strip_prefix(r"\.(")?.strip_suffix(")$")?;
    Some(inner.split('|').map(str::to_string).collect())
}

#[test]
fn standalone_hooks_select_every_supported_extension_and_nothing_else() {
    for hook in ["pre-commit", "pre-commit-warning-only"] {
        let text = fs::read_to_string(repo_root().join("hooks").join(hook)).unwrap();
        let listed = hook_extensions(&text).unwrap_or_else(|| panic!("{hook}: no SOURCE_EXT"));
        assert_eq!(listed, registry_extensions(), "{hook}");
        assert!(
            text.contains(r#"grep -iE "$SOURCE_EXT""#),
            "{hook}: must match without regard to case"
        );
    }
}

/// A Go file, outside the old C/C++/Rust/Python/JavaScript selection, is
/// checked, in any letter case.
#[test]
fn pre_commit_checks_languages_beyond_the_original_six() {
    let go =
        b"func F(a int) int {\n if a > 1 { if a > 2 { if a > 3 { return 1 } } }\n return 0\n}\n";
    let dir = repo_with("hook-go", &[("a.go", go), ("B.GO", go)]);
    let output = hook_with("pre-commit", &dir, KNOTS);
    let out = all_output(&output);
    assert_eq!(output.status.code(), Some(1), "{out}");
    assert!(out.contains("a.go:1:F"), "{out}");
    assert!(out.contains("B.GO:1:F"), "{out}");
    fs::remove_dir_all(&dir).unwrap();
}

/// A staged file knots cannot read (exit 3) fails the blocking hooks, is
/// reported by the warning-only hook, and knots' own message is shown.
#[test]
fn an_unreadable_file_is_never_a_clean_result() {
    let dir = repo_with(
        "hook-unreadable",
        &[("ok.c", OK_C), ("bad.c", UNREADABLE_C)],
    );

    let output = hook_with("pre-commit", &dir, KNOTS);
    let out = all_output(&output);
    assert_eq!(output.status.code(), Some(1), "{out}");
    assert!(
        out.contains("bad.c: not analysed (knots exited 3)"),
        "{out}"
    );
    assert!(
        out.contains("knots: Warning: Not analysed (read): bad.c"),
        "{out}"
    );
    assert!(out.contains("could not analyse every staged file"), "{out}");

    let output = hook_with("pre-commit-warning-only", &dir, KNOTS);
    let out = all_output(&output);
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(
        out.contains("bad.c: not analysed (knots exited 3)"),
        "{out}"
    );
    assert!(out.contains("could not analyse 1 staged file(s)"), "{out}");
    assert!(!out.contains("All complexity checks passed"), "{out}");

    let output = isolated("sh")
        .arg(repo_root().join("hooks/pre-commit-wrapper.sh"))
        .arg(format!("--validator-path={KNOTS}"))
        .args(["ok.c", "bad.c"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let out = all_output(&output);
    assert_eq!(output.status.code(), Some(1), "{out}");
    assert!(
        out.contains("bad.c: not analysed (knots exited 3)"),
        "{out}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// knots analyses a file with syntax errors and no recoverable function and
/// exits 0, warning on stderr; the hook passes and shows that warning.
#[test]
fn knots_warnings_are_shown_not_dropped() {
    let dir = repo_with("hook-syntax", &[("syn.c", b"}}}{{{ ((( int\n")]);
    let output = hook_with("pre-commit", &dir, KNOTS);
    let out = all_output(&output);
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert!(
        out.contains("knots: Warning: syn.c has syntax errors"),
        "{out}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// With source files to check and no knots, the blocking hooks fail and the
/// warning-only hook warns; with nothing to check, all pass.
#[test]
fn a_missing_knots_fails_loudly_when_there_is_something_to_check() {
    let dir = repo_with("hook-no-knots", &[("ok.c", OK_C)]);
    let output = hook_with("pre-commit", &dir, NO_KNOTS);
    assert_eq!(output.status.code(), Some(1), "{}", all_output(&output));
    assert!(String::from_utf8_lossy(&output.stderr).contains("knots not found"));

    let output = hook_with("pre-commit-warning-only", &dir, NO_KNOTS);
    assert_eq!(output.status.code(), Some(0), "{}", all_output(&output));
    assert!(String::from_utf8_lossy(&output.stderr).contains("were not checked"));

    let wrapper = |files: &[&str]| {
        isolated("sh")
            .arg(repo_root().join("hooks/pre-commit-wrapper.sh"))
            .arg(format!("--validator-path={NO_KNOTS}"))
            .args(files)
            .current_dir(&dir)
            .output()
            .unwrap()
    };
    let output = wrapper(&["ok.c"]);
    assert_eq!(output.status.code(), Some(1), "{}", all_output(&output));
    assert!(String::from_utf8_lossy(&output.stderr).contains("knots not found"));
    assert_eq!(wrapper(&[]).status.code(), Some(0));
    fs::remove_dir_all(&dir).unwrap();

    let dir = repo_with("hook-no-sources", &[("notes.txt", b"x\n")]);
    let output = hook_with("pre-commit", &dir, NO_KNOTS);
    assert_eq!(output.status.code(), Some(0), "{}", all_output(&output));
    fs::remove_dir_all(&dir).unwrap();
}

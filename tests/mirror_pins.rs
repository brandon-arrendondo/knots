//! scripts/check_mirror_pins.sh, run against a local copy of the mirror in a
//! scratch repository, so no case needs the network. Unix only: the script
//! is POSIX sh.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A command with every inherited `GIT_*` variable removed, so git acts on
/// the repository in its working directory (cargo test runs as a git hook).
fn isolated(program: &str) -> Command {
    let mut cmd = Command::new(program);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid");
    cmd
}

fn git(dir: &Path, args: &[&str]) {
    let out = isolated("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knots-{}-pins-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A local mirror at `dir`, with one commit tagged v2.0.0.
fn local_mirror(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["-c", "init.defaultBranch=main", "init", "-q"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "knots 2.0.0"]);
    git(dir, &["tag", "v2.0.0"]);
}

/// A repository at `dir` holding the script and `docs` (path, text), staged.
fn docs_repo(dir: &Path, docs: &[(&str, &str)]) {
    fs::create_dir_all(dir.join("scripts")).unwrap();
    git(dir, &["-c", "init.defaultBranch=main", "init", "-q"]);
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check_mirror_pins.sh");
    fs::copy(&script, dir.join("scripts/check_mirror_pins.sh")).unwrap();
    for (path, text) in docs {
        fs::write(dir.join(path), text).unwrap();
    }
    git(dir, &["add", "."]);
}

/// Runs the check on `docs` against a local mirror tagged v2.0.0, or
/// against `mirror_url` when given.
fn check(name: &str, docs: &[(&str, &str)], mirror_url: Option<&str>) -> Output {
    let root = scratch(name);
    local_mirror(&root.join("mirror"));
    docs_repo(&root.join("repo"), docs);
    let local = format!("file://{}", root.join("mirror").display());
    let out = isolated("sh")
        .arg("scripts/check_mirror_pins.sh")
        .env("KNOTS_PRE_COMMIT_URL", mirror_url.unwrap_or(&local))
        .current_dir(root.join("repo"))
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    out
}

const MIRROR: &str = "https://github.com/brandon-arrendondo/knots-pre-commit";

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn quoted_pins_in_a_file_with_a_space_resolve() {
    let doc = format!("  - repo: {MIRROR}\n    rev: 'v2.0.0'\n  - repo: {MIRROR}\n    rev: \"v2.0.0\"  # pinned\n");
    let out = check("quoted", &[("my config.yaml", &doc)], None);
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(all.contains("ok       my config.yaml:2 v2.0.0"), "{all}");
    assert!(all.contains("ok       my config.yaml:4 v2.0.0"), "{all}");
}

#[test]
fn a_missing_tag_is_reported_and_fails() {
    let doc = format!("- repo: {MIRROR}\n  rev: v9.9.9\n");
    let out = check("missing", &[("a.md", &doc)], None);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("MISSING  a.md:2 v9.9.9"),
        "{}",
        text(&out)
    );
}

/// A mirror entry with no rev: must not take the next repository's.
#[test]
fn a_rev_belongs_to_the_repo_entry_above_it() {
    let doc = format!(
        "- repo: {MIRROR}\n  hooks: []\n- repo: https://github.com/pre-commit/pre-commit-hooks\n  rev: v9.9.9\n- repo: {MIRROR}\n  rev: v2.0.0\n"
    );
    let out = check("scoped", &[("a.yaml", &doc)], None);
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(!all.contains("v9.9.9"), "{all}");
    assert!(all.contains("ok       a.yaml:6 v2.0.0"), "{all}");
}

/// An unreachable mirror decides nothing: exit 2, never MISSING.
#[test]
fn an_unreachable_mirror_is_not_a_missing_tag() {
    let doc = format!("- repo: {MIRROR}\n  rev: v2.0.0\n");
    let out = check(
        "offline",
        &[("a.md", &doc)],
        Some("file:///nonexistent/mirror"),
    );
    let all = text(&out);
    assert_eq!(out.status.code(), Some(2), "{all}");
    assert!(all.contains("cannot reach mirror"), "{all}");
    assert!(!all.contains("MISSING"), "{all}");
}

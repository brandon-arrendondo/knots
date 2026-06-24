"""
Invoke tasks for knots development.

Usage:
    invoke check          # Run pre-commit hooks on all files
    invoke build          # Build in release mode
    invoke test           # Run all tests
    invoke docs           # Build HTML documentation
    invoke clean          # Remove build artifacts
    invoke bump-version   # Bump version across all files (reads Cargo.toml)

Install invoke: pip install invoke
"""

import re
from pathlib import Path

from invoke import task


def _read_cargo_version():
    cargo = Path("Cargo.toml").read_text()
    # Match the workspace-level version line (not dependency versions)
    match = re.search(r'^version = "([^"]+)"', cargo, re.MULTILINE)
    if not match:
        raise RuntimeError("Could not find version in Cargo.toml")
    return match.group(1)


@task
def bump_version(c, new_version=None):
    """Bump version across all files that reference it.

    Reads the current version from Cargo.toml, then updates it there plus
    every file that embeds a version pin. If --new-version is omitted, prints
    the current version and the list of files that would be changed.

    Args:
        new_version: Target version string, e.g. 1.8.1 (no leading 'v').
    """
    current = _read_cargo_version()

    # Files that embed a bare version number (no 'v' prefix)
    bare_version_files = [
        # (path, regex-pattern-to-match, replacement-template)
        ("Cargo.toml",        rf'^(version = "){re.escape(current)}(")',   r"\g<1>{new}\g<2>"),
        ("man/knots.1",       rf'("knots ){re.escape(current)}(")',        r"\g<1>{new}\g<2>"),
    ]

    # Files that embed a 'v'-prefixed tag (rev: vX.Y.Z or /vX.Y.Z/ in URLs)
    tagged_version_files = [
        "example-pre-commit-config.yaml",
        "hooks/PRE_COMMIT_FRAMEWORK.md",
        "docs/ci-integration.rst",
        "docs/test-complexity.rst",
    ]

    if not new_version:
        print(f"Current version: {current}")
        print("\nFiles that would be updated:")
        for path, *_ in bare_version_files:
            print(f"  {path}  (bare version)")
        for path in tagged_version_files:
            print(f"  {path}  (rev: v{current}  →  rev: vNEW)")
        print("\nRun: invoke bump-version --new-version X.Y.Z")
        return

    tag_old = f"v{current}"
    tag_new = f"v{new_version}"

    changed = []

    for path, pattern, tmpl in bare_version_files:
        p = Path(path)
        text = p.read_text()
        updated = re.sub(pattern, tmpl.format(new=new_version), text, flags=re.MULTILINE)
        if updated != text:
            p.write_text(updated)
            changed.append(path)

    for path in tagged_version_files:
        p = Path(path)
        if not p.exists():
            continue
        text = p.read_text()
        updated = text.replace(tag_old, tag_new)
        if updated != text:
            p.write_text(updated)
            changed.append(path)

    if changed:
        print(f"Bumped {current} → {new_version} in:")
        for f in changed:
            print(f"  {f}")
    else:
        print(f"No occurrences of {current} (or {tag_old}) found — nothing changed.")


@task
def publish(c, dry_run=False):
    """Publish knots and knots-test-complexity to crates.io.

    Verifies the working tree is clean and the current git tag matches
    Cargo.toml before publishing. Publishes knots first, then
    knots-test-complexity (which depends on it).

    Args:
        dry_run: Pass --dry-run to cargo publish (verify without uploading).
    """
    import subprocess

    # Ensure working tree is clean
    result = subprocess.run(
        ["git", "status", "--porcelain"], capture_output=True, text=True
    )
    if result.stdout.strip():
        raise SystemExit("Working tree is dirty — commit or stash changes before publishing.")

    version = _read_cargo_version()
    tag = f"v{version}"

    # Ensure the current commit is tagged with the expected version
    result = subprocess.run(
        ["git", "tag", "--points-at", "HEAD"], capture_output=True, text=True
    )
    tags = result.stdout.split()
    if tag not in tags:
        raise SystemExit(
            f"HEAD is not tagged {tag} — run: git tag {tag} && git push origin {tag}"
        )

    flag = " --dry-run" if dry_run else ""
    print(f"Publishing knots {version} to crates.io{' (dry run)' if dry_run else ''}...")
    c.run(f"cargo publish -p knots{flag}", pty=True)

    print(f"Publishing knots-test-complexity {version} to crates.io{' (dry run)' if dry_run else ''}...")
    c.run(f"cargo publish -p knots-test-complexity{flag}", pty=True)


@task
def check(c):
    """Run pre-commit hooks on all files."""
    c.run("pre-commit run --all-files", pty=True)


@task
def build(c, release=False):
    """Build the project.

    Args:
        release: Build in release mode (default: debug).
    """
    cmd = "cargo build --workspace"
    if release:
        cmd += " --release"
    c.run(cmd, pty=True)


@task
def test(c):
    """Run all Rust unit tests."""
    c.run("cargo test --workspace", pty=True)


@task
def docs(c, open_browser=False):
    """Build HTML documentation with Sphinx.

    Args:
        open_browser: Open the result in a browser after building.
    """
    c.run("sphinx-build -b html docs docs/_build/html", pty=True)
    if open_browser:
        import os
        c.run(f"xdg-open docs/_build/html/index.html", warn=True)
    else:
        print("Docs built: docs/_build/html/index.html")


@task
def clean(c):
    """Remove build artifacts."""
    c.run("cargo clean", pty=True)
    c.run("rm -rf docs/_build/", pty=True)

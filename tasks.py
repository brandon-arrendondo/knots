"""
Invoke tasks for knots development.

Usage:
    invoke check          # Run pre-commit hooks on all files
    invoke build          # Build in release mode
    invoke test           # Run all tests
    invoke docs           # Build HTML documentation
    invoke clean          # Remove build artifacts
    invoke bump-version   # Bump version across all files (reads Cargo.toml)
    invoke sync-languages # Sync doc language lists to src/lib.rs (--write to apply)

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


def _read_languages():
    """Parse the canonical LANGUAGES table from src/lib.rs.

    Returns a list of dicts: {name, extensions, explicit_only}. This is the
    single source of truth — the same table that powers `knots
    --supported-languages` and SUPPORTED_EXTENSIONS.
    """
    text = Path("src/lib.rs").read_text()
    block = re.search(r"pub const LANGUAGES:.*?=\s*&\[(.*?)\n\];", text, re.DOTALL)
    if not block:
        raise RuntimeError("Could not find LANGUAGES table in src/lib.rs")
    entry = re.compile(
        r'name:\s*"([^"]+)",\s*extensions:\s*&\[([^\]]*)\],'
        r'\s*explicit_only:\s*&\[([^\]]*)\]'
    )
    langs = []
    for m in entry.finditer(block.group(1)):
        langs.append(
            {
                "name": m.group(1),
                "extensions": re.findall(r'"([^"]+)"', m.group(2)),
                "explicit_only": re.findall(r'"([^"]+)"', m.group(3)),
            }
        )
    if not langs:
        raise RuntimeError("LANGUAGES table parsed as empty — check the format in src/lib.rs")
    return langs


def _render_languages(langs):
    """Build the three textual forms used throughout the docs."""
    names = [lang["name"] for lang in langs]
    if len(names) > 1:
        comma = ", ".join(names[:-1]) + ", and " + names[-1]
    else:
        comma = names[0]
    slash = "/".join(names)

    rows = ["| Language | Extensions | Explicit-only |", "|----------|------------|---------------|"]
    for lang in langs:
        exts = " ".join(f"`.{e}`" for e in lang["extensions"])
        explicit = " ".join(f"`.{e}`" for e in lang["explicit_only"]) or "—"
        rows.append(f"| {lang['name']} | {exts} | {explicit} |")
    table = "\n".join(rows)

    return comma, slash, table


# Files that embed a language enumeration, with the regex that isolates it.
# (path, pattern, replacement-template using {comma}/{slash}). Mirrors the
# bump_version file list. The CLAUDE.md table is handled separately via markers.
def _language_file_edits(comma, slash):
    return [
        ("Cargo.toml",
         r'(description = ")[A-Za-z+/]+( complexity analyzer)',
         rf"\g<1>{slash}\g<2>"),
        ("README.md",
         r"(\*\*Multi-Language\*\*: )[^—]+?( — same metrics)",
         rf"\g<1>{comma}\g<2>"),
        ("docs/alternatives.rst",
         r"(measure code complexity for )[^.\n]+(\.)",
         rf"\g<1>{comma}\g<2>"),
        ("docs/alternatives.rst",
         r"(threshold enforcement\*\* across )[^.\n]+?( in one pass)",
         rf"\g<1>{comma}\g<2>"),
        ("hooks/QUICKSTART.md",
         r"(you're committing \()[^)]+(\))",
         rf"\g<1>{comma}\g<2>"),
        (".github/workflows/build.yml",
         r"(code complexity analyzer for )[^\n]+",
         rf"\g<1>{comma}"),
    ]


@task
def sync_languages(c, write=False):
    """Sync every doc's language list to the LANGUAGES table in src/lib.rs.

    Reads the single source of truth (src/lib.rs), then rewrites the language
    enumeration in each file that embeds one — analogous to bump-version for
    the version string. Without --write it previews drift (no files touched).

    Args:
        write: Apply the changes (default: preview only).
    """
    langs = _read_languages()
    comma, slash, table = _render_languages(langs)

    print(f"Languages (from src/lib.rs): {comma}\n")

    edits = _language_file_edits(comma, slash)
    marker_block = f"<!-- BEGIN:supported-languages (generated by `invoke sync-languages`) -->\n{table}\n<!-- END:supported-languages -->"
    marker_files = ["CLAUDE.md"]

    changed = []

    for path, pattern, tmpl in edits:
        p = Path(path)
        if not p.exists():
            continue
        text = p.read_text()
        updated = re.sub(pattern, tmpl, text)
        if updated != text:
            changed.append(path)
            if write:
                p.write_text(updated)

    marker_re = re.compile(
        r"(<!-- BEGIN:supported-languages.*?-->).*?(<!-- END:supported-languages -->)",
        re.DOTALL,
    )
    for path in marker_files:
        p = Path(path)
        if not p.exists():
            continue
        text = p.read_text()
        updated = marker_re.sub(rf"\g<1>\n{table}\n\g<2>", text)
        if updated != text:
            changed.append(path)
            if write:
                p.write_text(updated)

    if not changed:
        print("All language lists are already in sync. ✓")
        return

    verb = "Updated" if write else "Would update (run with --write)"
    print(f"{verb}:")
    for path in dict.fromkeys(changed):  # dedupe, preserve order
        print(f"  {path}")


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

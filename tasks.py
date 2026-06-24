"""
Invoke tasks for knots development.

Usage:
    invoke check    # Run pre-commit hooks on all files
    invoke build    # Build in release mode
    invoke test     # Run all tests
    invoke docs     # Build HTML documentation
    invoke clean    # Remove build artifacts

Install invoke: pip install invoke
"""

from invoke import task


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

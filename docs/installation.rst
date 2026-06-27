Installation
============

From PyPI (prebuilt, no Rust toolchain)
---------------------------------------

Knots is published to PyPI as a prebuilt binary wheel, so this installs in
seconds with no compile and no Rust toolchain. Recommended via a tool installer:

::

    pipx install knots
    # or
    uv tool install knots

The companion test-quality analyzer is a separate package:

::

    pipx install knots-test-complexity

(Plain ``pip install knots`` works too, e.g. inside a virtualenv.) This is also
what the ``knots-pre-commit`` hooks use under the hood — see :doc:`ci-integration`.

From crates.io
--------------

::

    cargo install knots

This installs both ``knots`` and ``knots-test-complexity``.

From Source
-----------

::

    git clone https://github.com/brandon-arrendondo/knots.git
    cd knots
    cargo build --release
    ./target/release/knots --version

Requirements
------------

A Rust toolchain (install via `rustup <https://rustup.rs>`_). No C compiler,
Python interpreter, build system, or language server required — knots uses
tree-sitter grammars bundled as Rust dependencies.

Supported Languages
-------------------

Knots analyzes source files with these extensions:

* ``.c`` — C
* ``.cpp``, ``.cc``, ``.cxx`` — C++
* ``.hpp``, ``.hxx`` — C++ headers
* ``.rs`` — Rust
* ``.py`` — Python 3 (including decorated functions and class methods)
* ``.js``, ``.mjs``, ``.cjs`` — JavaScript (ES2015+, including class methods and generators)

Installation
============

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

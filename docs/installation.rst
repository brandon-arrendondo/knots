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

Rust 1.70 or higher. No C compiler, build system, or language server required —
knots uses tree-sitter grammars bundled as Rust dependencies.

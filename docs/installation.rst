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

This installs ``knots``. The companion is not published on crates.io;
install its prebuilt PyPI package as described above. From a source checkout,
use ``cargo install --path knots-test-complexity``.

From Source
-----------

::

    git clone https://github.com/brandon-arrendondo/knots.git
    cd knots
    cargo build --release --workspace
    ./target/release/knots --version

Requirements
------------

Source builds require a Rust toolchain (install via `rustup
<https://rustup.rs>`_) and a native C/C++ compiler for the bundled tree-sitter
grammars. Prebuilt wheels do not require that toolchain. Analysis does not
require compiling your project or running its language server.

Supported Languages
-------------------

Run ``knots --supported-languages`` for the compiled registry. It currently
contains 16 languages:

* C: ``.c``, ``.h``
* C++: ``.cpp``, ``.cc``, ``.cxx``, ``.hpp``, ``.hxx``
* Rust: ``.rs``
* Python: ``.py``
* JavaScript: ``.js``, ``.mjs``, ``.cjs``, ``.jsx``
* TypeScript: ``.ts``, ``.tsx``
* Ada: ``.adb``, ``.ada``, ``.ads``
* Go: ``.go``
* Java: ``.java``
* C#: ``.cs``
* Kotlin: ``.kt``, ``.kts``
* Swift: ``.swift``
* PHP: ``.php``
* Free-form Fortran: ``.f90``, ``.f95``, ``.f03``, ``.f08`` and uppercase variants
* Scala: ``.scala``, ``.sc``
* Lua: ``.lua``

Recursive scans include header files such as ``.h`` and ``.ads``. The
registry labels these explicit-only, but knots discovers files with its broader
parseable-extension predicate. Fixed-form Fortran (``.f``, ``.for``,
``.f77`` and uppercase variants) is not supported by the current substrate.

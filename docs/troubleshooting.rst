Troubleshooting
===============

"Path is a directory. Use -r/--recursive"
------------------------------------------

You passed a directory without ``-r``::

    knots -r path/to/directory/

"Warning: Skipping <file>: stream did not contain valid UTF-8"
--------------------------------------------------------------

The file has encoding issues. Knots continues processing other files.

Convert the file to UTF-8::

    iconv -f ISO-8859-1 -t UTF-8 file.c > file_utf8.c

Or exclude problematic files via a filter:

.. code-block:: json

    {
      "file_patterns": ["!**/legacy_encoding/**"]
    }

"No supported source files found in directory"
----------------------------------------------

Check:

- File extensions are ``.c``, ``.cpp``, ``.cc``, ``.cxx``, or ``.rs``
  (recursive mode only scans source files, not headers)
- You're pointing at the right directory
- Files aren't filtered out by an active include/exclude rule

To include ``.h`` header files:

.. code-block:: json

    {
      "file_patterns": ["**/*.c", "**/*.h"]
    }

Metrics seem lower than expected for Rust (vs. clippy)
-------------------------------------------------------

This is expected. See :doc:`alternatives` — Cognitive Complexity Algorithm
Differences. Knots implements the Campbell spec, which counts loops and
applies a nesting penalty. Clippy's ``cognitive_complexity`` lint does not
count loops and has no nesting penalty, producing scores 3–4× lower.

Clippy threshold 25 ≈ knots threshold 75–100 for equivalent strictness.

``report.txt`` not generated
-----------------------------

``report.txt`` is only generated in ``-r`` (recursive) mode::

    knots -r src/

SARIF has no results
---------------------

Knots omits SARIF results for functions with ``max(McCabe, cognitive) ≤ 10``
(the 😊 "Good" band). If all functions score in that range, the SARIF file
will contain a valid but empty results array. Use ``--include`` with
``"min_complexity": 1`` to force all functions through if needed.

Compile commands: "file not found" or missing files
----------------------------------------------------

Knots resolves paths in ``compile_commands.json`` using the ``directory``
field from each entry. If your build ran from a different working directory
than where you're running knots, paths may not resolve. Regenerate the
compile database from the project root, or ensure ``directory`` fields
contain absolute paths.

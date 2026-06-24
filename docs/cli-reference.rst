CLI Reference
=============

Synopsis
--------

::

    knots [OPTIONS] [FILE]...
    knots [OPTIONS] --compile-commands <FILE>

Positional Arguments
--------------------

``[FILE]...``
    One or more paths to source files or directories.
    Pass a directory with ``-r`` for recursive scanning.
    Mutually exclusive with ``--compile-commands``.

Options
-------

``-r``, ``--recursive``
    Recursively process all supported source files in directories. See
    :doc:`installation` for the full list of recognized extensions. Header
    files (``.h``, ``.hpp``, ``.hxx``) are skipped unless explicitly included
    via a filter. Generates ``report.txt`` in the current directory.

``-v``, ``--verbose``
    Show detailed per-function analysis including all test scoring sub-axes.

``-m``, ``--matrix``
    Show testability matrix categorization. Functions are placed into four
    quadrants: QUICK WINS, INVEST IN TESTS, ADD DOCS, REFACTOR.

``--compile-commands <FILE>``
    Use a ``compile_commands.json`` (from CMake, Bear, etc.) to get the file
    list. **C/C++ only** — compilation databases are not produced by Rust,
    Python, or JavaScript toolchains. Mutually exclusive with positional
    ``[FILE]...``.

``--include <FILE>``
    Include filter rules from a JSON file (whitelist). See ``FILTERS``.

``--exclude <FILE>``
    Exclude filter rules from a JSON file (blacklist). See ``FILTERS``.

``--format <FORMAT>``
    Output format. One of:

    .. list-table::
       :widths: 12 88

       * - ``text``
         - Human-readable output (default).
       * - ``sarif``
         - SARIF 2.1.0 JSON for VS Code / GitHub Code Scanning.
       * - ``json``
         - JSON array of per-function metrics.
       * - ``ndjson``
         - Newline-delimited JSON — one record per line; composable via ``find``/``xargs``.
       * - ``csv``
         - CSV with header row.

Threshold Flags
---------------

All threshold flags cause knots to exit with status 1 if any function
exceeds the specified value.

``--mccabe-threshold <N>``
    McCabe cyclomatic complexity.

``--cognitive-threshold <N>``
    Cognitive complexity (Campbell spec).

``--nesting-threshold <N>``
    Nesting depth.

``--sloc-threshold <N>``
    Source lines of code.

``--abc-threshold <F>``
    ABC magnitude (floating-point).

``--return-threshold <N>``
    Return statement count.

``--aird-threshold <N>``
    AIRD score. Recommended value: ``85`` (empirically validated against
    Sonnet 4.6 and Opus 4.8).

``--aicp-threshold <N>``
    AICP score.

``--external-calls-threshold <N>``
    External call count.

Informational
-------------

``-h``, ``--help``
    Print help.

``-V``, ``--version``
    Print version.

Exit Status
-----------

``0``
    Successful analysis, no threshold violations.

``1``
    Error (file not found, parse error, invalid arguments), or one or more
    threshold violations when any ``--*-threshold`` flag is set.

Complexity Indicators
---------------------

Knots uses visual emoji indicators based on ``max(McCabe, cognitive)``:

============  =============================================
``1–10``      😊 Good — low complexity, easy to maintain
``11–20``     😐 Okay — moderate complexity, monitor
``21–49``     😠 Bad — high complexity, consider refactoring
``50+``       😢 Critical — urgent refactoring needed
============  =============================================

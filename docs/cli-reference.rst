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
    via a filter. A per-function report file is written only when ``--report``
    is given.

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
    Include filter rules from a JSON file (whitelist). See :doc:`filters`.

``--exclude <FILE>``
    Exclude filter rules from a JSON file (blacklist). See :doc:`filters`.

``--exclude-path <PATTERN>``
    Exclude files whose path matches this regex. Repeatable; uses the same
    syntax as a pre-commit ``exclude:`` entry.

``--report <FILE>``
    Write a detailed per-function report to ``<FILE>``. Opt-in — omit to
    suppress the file.

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

``-j <N>``, ``--jobs <N>``
    Number of parallel analysis threads when processing multiple files.
    ``0`` (the default) auto-detects available CPU cores via
    ``std::thread::available_parallelism``; ``1`` forces sequential
    processing; any other value pins the thread-pool to that count.
    Has no effect when analyzing a single file.

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
    AIRD (AI Reasoning Difficulty) score. Recommended value: ``85``
    (empirically validated against Sonnet 4.6 and Opus 4.8). Run
    ``knots --explain aird`` for what drives the score and how to lower it.

``--aicp-threshold <N>``
    AICP (AI Context Pressure) score. Run ``knots --explain aicp`` for details.

``--external-calls-threshold <N>``
    External call count.

Baseline / Ratchet Mode
-----------------------

Adopt a threshold gate on an existing codebase without first refactoring every
pre-existing offender. Snapshot the current per-function scores once, then gate
only on **regressions** — a new function over threshold, or a baselined one
whose score got worse. See :doc:`baseline` for the full guide and file format.

``--baseline <FILE>``
    Gate against the snapshot in ``<FILE>``: a function is only reported if it
    is over threshold **and** either absent from the baseline (new) or worse on
    the offending metric than its baselined value. Functions at or below their
    baselined scores are tolerated. Without this flag, every over-threshold
    function fails (the default).

``--write-baseline``
    Snapshot the current scores to the ``--baseline`` file and exit ``0``
    without gating. Use it to adopt the gate, and to re-accept the current
    state after intentional changes. Requires ``--baseline <FILE>``; works with
    any ``--format``.

.. code-block:: bash

    # Snapshot today (run once, commit the file)
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json --write-baseline

    # Gate against it — fails only on new or worsened functions
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json

Changed-Function Scoping
------------------------

Restrict threshold gating to the functions you actually touched, so a
pre-existing over-threshold function in a file you edited does not block an
unrelated change. knots intersects each function's line range with the lines
changed (per ``git diff``) and gates only the functions that overlap. This is
complementary to ``--baseline``: scoping narrows *which functions are checked*;
the baseline decides *what counts as worse*. The two compose — gate the touched
functions, and among those, only fail on new or worsened ones.

``--since <REF>``
    Gate only functions overlapping lines changed in the working tree relative
    to the given git ref (e.g. ``HEAD``, ``main``, a commit SHA). Compares the
    current working tree against ``<REF>``. Brand-new untracked files are treated
    as entirely changed, so all of their functions are in scope. Requires a git
    repository; an unknown ref or non-repo exits ``1`` with git's error.

``--changed``
    Gate only functions you changed in the working tree (uncommitted edits plus
    untracked files). Sugar for ``--since HEAD``. Mutually exclusive with
    ``--since``.

.. code-block:: bash

    # In a pre-commit / CI gate, only fail on functions in this change
    knots -r src/ --aird-threshold 85 --changed

    # Gate everything that diverged from main (good for PR checks)
    knots -r src/ --aird-threshold 85 --since main

.. note::

   Scoping affects **gating only** — text, JSON, SARIF, NDJSON, and CSV output
   still report every analyzed function. Only the threshold check is narrowed.

Informational
-------------

``--explain <METRIC>``
    Print a terminal-friendly explanation of a metric — what it measures and
    how to lower it — then exit ``0``. No input files required. Valid metrics:
    ``mccabe``, ``cognitive``, ``nesting``, ``sloc``, ``abc``, ``returns``,
    ``aird``, ``aicp``, ``external-calls``. Handy when you meet ``AIRD 98 > 85``
    mid-commit and don't want to leave the terminal:

    .. code-block:: bash

        knots --explain aird

``-h``, ``--help``
    Print help.

``-V``, ``--version``
    Print version.

Exit Status
-----------

``0``
    Successful analysis, no threshold violations.

``1``
    Error (file not found, parse error, unreadable baseline, unknown git ref or
    not a git repository under ``--since`` / ``--changed``), or one or more
    threshold violations when any ``--*-threshold`` flag is set. In
    ``--baseline`` mode, only new or worsened functions count as violations; under
    ``--since`` / ``--changed`` only touched functions are checked.

``2``
    Invalid arguments (e.g. ``--write-baseline`` without ``--baseline``).

Complexity Indicators
---------------------

Knots uses visual emoji indicators based on ``max(McCabe, cognitive)``:

============  =============================================
``1–10``      😊 Good — low complexity, easy to maintain
``11–20``     😐 Okay — moderate complexity, monitor
``21–49``     😠 Bad — high complexity, consider refactoring
``50+``       😢 Critical — urgent refactoring needed
============  =============================================

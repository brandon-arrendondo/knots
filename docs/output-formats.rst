Output Formats
==============

All structured formats suppress normal text output — only the data goes to
stdout. Use ``--format <FORMAT>`` to select.

text (default)
--------------

Human-readable per-function output with emoji indicators and a summary block.
Suitable for interactive use and terminal review.

::

    knots src/main.c
    knots -v src/main.c

JSON
----

Pretty-printed JSON array of per-function records:

::

    knots --format json src/main.c > metrics.json
    knots -r --format json src/ > metrics.json

Each record contains these 17 fields (key order is not significant;
serialized keys come out in alphabetical order):

.. code-block:: json

    {
      "file": "src/main.c",
      "function": "process_data",
      "start_line": 42,
      "end_line": 161,
      "mccabe": 28,
      "cognitive": 45,
      "nesting": 8,
      "sloc": 120,
      "abc_magnitude": 35.71,
      "return_count": 7,
      "test_score": 18,
      "doc_score": 0,
      "aird": 87,
      "aicp": 72,
      "external_calls": 14,
      "file_ce": 0,
      "unreachable_blocks": 0
    }

NDJSON (newline-delimited JSON)
--------------------------------

One JSON object per line. Unlike ``--format json``, output from multiple
invocations concatenates cleanly without array merging — ideal for corpus
analysis via ``find``/``xargs``.

::

    # Composable across files (C/C++ project)
    find . -type f \( -name "*.c" -o -name "*.cpp" \) -print0 | xargs -0 -r knots --format ndjson > all_metrics.ndjson

    # Python project corpus
    find src/ -type f -name "*.py" -print0 | xargs -0 -r knots --format ndjson | jq 'select(.aird > 70)'

    # Rust project corpus
    find . -type f -name "*.rs" -print0 | xargs -0 -r knots --format ndjson | jq 'select(.cognitive > 20)'

    # JavaScript project corpus
    find src/ -type f \( -name "*.js" -o -name "*.mjs" \) -print0 | xargs -0 -r knots --format ndjson | jq 'select(.mccabe > 10)'

    # Parallel analysis with one writer (all supported languages)
    knots -r -j 4 --format ndjson . > metrics.ndjson

The ``xargs -r`` examples use GNU xargs to avoid running on an empty file list.
Null-delimited paths preserve filenames containing spaces. Avoid concurrent
processes appending to one NDJSON file: records can interleave.

CSV
---

Header row followed by one row per function. Column order follows the field list above (``file``, ``function``,
``start_line``, ...), not the alphabetical JSON serialization order.

::

    knots -r --format csv src/ > metrics.csv

Import directly into spreadsheets, pandas, or any SQL tool.

Score components
----------------

Add ``--score-components`` to any of ``json``, ``ndjson`` or ``csv`` to get
AIRD's and AICP's per-term contributions and pre-clamp values: extra keys
in JSON/NDJSON, trailing columns in CSV. See :ref:`score-components` for
the column list and how to use them.

::

    knots -r --format csv --score-components src/ > metrics.csv

SARIF (VS Code / GitHub Code Scanning)
---------------------------------------

Emits `SARIF 2.1.0 <https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html>`_
JSON for static-analysis tooling integration.

::

    knots --format sarif src/main.c > knots.sarif
    knots -r --format sarif src/ > knots.sarif
    knots --compile-commands compile_commands.json --format sarif > knots.sarif

One SARIF result is emitted per function whose ``max(McCabe, cognitive)``
exceeds 10. Severity follows the emoji thresholds:

===============  ===========  =====
Max complexity   SARIF level  Emoji
===============  ===========  =====
1–10             (omitted)    😊
11–20            ``note``     😐
21–49            ``warning``  😠
50+              ``error``    😢
===============  ===========  =====

Each result carries seven properties: ``mccabe``, ``cognitive``, ``nesting``,
``sloc``, ``abcMagnitude``, ``returnCount`` and ``testScore``. The fixed
complexity band is independent of threshold flags; include filters cannot
force functions at or below 10 into SARIF. Use JSON/NDJSON for all records.

**GitHub Code Scanning**: upload with ``github/codeql-action/upload-sarif@v3``
to surface findings as PR annotations.

**VS Code**: install the *SARIF Viewer* extension and open ``knots.sarif``.

The run's ``invocations[0]`` says whether the scan was complete.
``executionSuccessful`` is ``false`` when a selected file was not analysed
(see :doc:`cli-reference`, Exit Status), and ``exitCode`` is the status
knots exits with: 0, 1 for a threshold violation, or 3 for an incomplete
scan. ``toolExecutionNotifications`` holds one entry for each file not
analysed, at level ``error`` with descriptor ``knots/input-not-analysed``.
A file that parsed only with syntax errors and yielded no function gets a
``warning`` with descriptor ``knots/no-functions-recovered``. Both
descriptors are declared in ``tool.driver.notifications``, and each
notification refers to its descriptor by ``id`` and ``index``. A threshold
violation does not make ``executionSuccessful`` false. The log is written
even when every selected file was lost; it then has no results.

.. _completion-report:

Completion report
-----------------

``--completion-report <FILE>`` writes what the scan analysed to ``FILE`` as
JSON, whatever the ``--format``. The format's own output is unchanged, so
JSON stays a bare array of records::

    knots -r src --format json --completion-report completion.json

.. code-block:: json

    {
      "complete": false,
      "files": {
        "selected": 3,
        "analysed": 2,
        "not_analysed": 1,
        "recovered_no_functions": 1
      },
      "not_analysed": [
        {
          "file": "src/binary.c",
          "stage": "read",
          "reason": "stream did not contain valid UTF-8"
        }
      ],
      "recovered_no_functions": ["src/bad.c"]
    }

``complete`` is ``false`` exactly when the run exits 3. ``stage`` is one of
``read`` (unreadable, or not UTF-8), ``parse`` (no grammar could parse it),
``crash`` (its analysis panicked), ``unsupported`` (named, in a language
knots cannot analyse), ``missing`` (listed in ``compile_commands.json`` but
not on disk) and ``walk`` (a directory the recursive walk could not read, or a
dangling symlink). A ``crash`` reason carries at most 200 characters of the
panic message. ``recovered_no_functions`` lists the files that parsed only
with syntax errors and yielded no function; they count as analysed. Both
lists are sorted by file. The report is written before knots exits, including
when ``--write-baseline`` refuses an incomplete scan.

In recursive mode a file that was not analysed still counts as an import
target, so the Ce of each file that includes it is exact. Its own imports are
unknown, so it has no coupling row, and the Ca and Instability of the files it
would import, and the duplicate groups, may be undercounted; the text summary
says so.

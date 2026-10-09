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

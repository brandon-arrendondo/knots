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

Each record contains all 15 fields:

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
      "external_calls": 14
    }

NDJSON (newline-delimited JSON)
--------------------------------

One JSON object per line. Unlike ``--format json``, output from multiple
invocations concatenates cleanly without array merging — ideal for corpus
analysis via ``find``/``xargs``.

::

    # Composable across files
    find . -name "*.c" | xargs knots --format ndjson > all_metrics.ndjson

    # Pipe directly to jq
    find src/ -name "*.c" | xargs knots --format ndjson | jq 'select(.aird > 70)'

    # Parallel per-file analysis
    find . -name "*.c" | xargs -P4 -I{} sh -c 'knots --format ndjson {} >> metrics.ndjson'

    # Rust project corpus
    find . -name "*.rs" | xargs knots --format ndjson | jq 'select(.cognitive > 20)'

CSV
---

Header row followed by one row per function. Column order matches the JSON
field order.

::

    knots --format csv src/ > metrics.csv

Import directly into spreadsheets, pandas, or any SQL tool.

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

Each result carries a ``properties`` bag with all 15 metrics so downstream
tools can filter on individual values.

**GitHub Code Scanning**: upload with ``github/codeql-action/upload-sarif@v3``
to surface findings as PR annotations.

**VS Code**: install the *SARIF Viewer* extension and open ``knots.sarif``.

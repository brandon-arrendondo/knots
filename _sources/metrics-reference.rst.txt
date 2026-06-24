Metrics Reference
=================

Knots computes 13 metrics per function. All are present in JSON, NDJSON,
and CSV output.

Python Language Support
-----------------------

As of v1.7.0, knots supports Python (``.py``) in addition to C, C++, and Rust.
All 13 metrics are computed. Python-specific notes:

- **McCabe**: counts ``if``, ``elif``, ``while``, ``for``, ``except``, ``and``,
  ``or``, ``match`` (3.10+), and ternary expressions
- **Cognitive**: ``elif`` is a flat ``+1`` (no nesting penalty); ``lambda``
  increments nesting depth without adding a base cost
- **SLOC**: ``#`` comment lines are excluded
- **External calls**: attribute-form calls (``obj.method()``, ``module.func()``)
  are counted as external references
- **Limitations**: ``for_in_clause`` inside comprehensions is not counted;
  ``field_expression``-style calls are counted as external (conservative)

McCabe Cyclomatic Complexity
-----------------------------

Counts the number of linearly independent paths through a function.

- **Formula**: decision points + 1
- **Decision points**: ``if``/``elif``, ``while``, ``for``, ``do``,
  ``switch``/``case``/``match``, ternary, logical operators (``&&``/``||``,
  Python ``and``/``or``), ``except`` (Python and C++)
- **Thresholds**: ≤10 good, 11–20 moderate, 21+ consider refactoring
- **Validated**: 100% match with ``pmccabe`` output across a 32,205-function corpus

Cognitive Complexity
---------------------

Measures how difficult code is to understand, with higher weight for nesting
and structural complexity. Based on the
`G. Ann Campbell / SonarSource specification <https://www.sonarsource.com/resources/cognitive-complexity/>`_.

Key differences from McCabe:

- Nested structures add more than flat ones (nesting penalty)
- ``else``/``else if`` chains cost less than independent ``if`` chains
- ``switch`` is a single increment regardless of arm count

Validated against Mozilla's
`rust-code-analysis <https://github.com/mozilla/rust-code-analysis>`_ at
1.004× mean ratio across 11,365 Rust functions (285k lines).

Nesting Depth
-------------

Maximum depth of nested control structures (``if``, ``for``, ``while``,
``switch``, closures) within a function.

- Deep nesting (>4 levels) is strongly correlated with hard-to-maintain code
- Threshold flag: ``--nesting-threshold``

SLOC (Source Lines of Code)
----------------------------

Non-blank, non-comment lines of code within the function body.

- Useful in combination with complexity metrics
- Functions >50 SLOC often benefit from decomposition
- Threshold flag: ``--sloc-threshold``

ABC Complexity
--------------

Assignment, Branch, Condition magnitude vector.

- **A**: assignment statements
- **B**: branch statements (function calls)
- **C**: condition statements (decision points)
- **Magnitude**: ``√(A² + B² + C²)``
- Threshold flag: ``--abc-threshold`` (accepts floating-point)

Test Scoring
------------

Multi-dimensional metric assessing how difficult a function is to test
automatically. Five sub-axes, each 0–10:

==============  ==================================================================
Signature       Parameter complexity — count, types, pointer depth
Dependency      External dependencies called or referenced
Observable      Side effects, I/O, global state — how hard to observe outputs
Implementation  Internal control flow and structure — McCabe-derived
Documentation   Comment quality (-10 to 0, reduces the total score)
==============  ==================================================================

**Score ranges:**

- **≤10**: Trivial to test
- **11–20**: Simple, automatable with minimal metadata
- **21–30**: Moderate, needs good documentation
- **31+**: Complex, requires detailed specifications

See ``test_scoring.md`` in the repository root for the complete specification.

External Calls
--------------

Count of unique identifier-form call targets within the function that are
**not** defined in the same translation unit — covers out-of-file functions
and function-like macros. Measures external dependency breadth.

- Threshold flag: ``--external-calls-threshold``
- p99 across 32,205-function corpus: 20
- p90: 9, p75: 5
- Mean by AIRD band: 2.74 (low) → 8.69 (mid) → 17.40 (high)

.. note::

   For Rust, method call syntax (``self.foo()``, ``vec.push()``) via
   ``field_expression`` nodes is not yet counted. Plain function call
   syntax is counted.

AIRD — AI Reasoning Difficulty
--------------------------------

Normalized 0–100 score predicting how much reasoning effort an AI model
needs to safely modify a function. Higher = harder for an AI to modify.

.. code-block:: text

    AIRD = (cognitive/75 × 55) + (sloc/200 × 15) + (nesting/8 × 15)
         + (test_score/20 × 15) - (doc_score/10 × 15)

Ceiling values (p99 of observed distribution across 32,205 functions from
mosquitto, SQLite, curl, hostap, Lua, libcrc):

=========  =====
cognitive  75
sloc       200
nesting    8
=========  =====

Cognitive complexity is the dominant driver. SLOC, nesting, and testability
are secondary. Documentation (doc_score) reduces difficulty.

**Recommended CI threshold**: ``--aird-threshold 85``

Validated empirically against Sonnet 4.6 and Opus 4.8: functions scoring
≥85 were consistently rated significantly harder to modify than mid-band
or low-band functions.

**Distribution**: heavily right-skewed. In mature codebases, 67–88% of
functions score ≤10. The ≥76 bucket accounts for 1–2% of all functions.

AICP — AI Context Pressure
---------------------------

Normalized 0–100 score predicting how much context an AI model must load
before it can act. Complements AIRD: a function can be cheap to load but
hard to reason about, or expensive to load but trivial once context is
assembled.

.. code-block:: text

    AICP = (external_calls/20 × 60) + (sloc/200 × 40) - (doc_score/10 × 15)

External call breadth is the primary driver. The p99 ceiling of 20 external
calls is consistent across all 6 corpora.

- Threshold flag: ``--aicp-threshold``

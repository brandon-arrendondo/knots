Metrics Reference
=================

Knots computes 13 metrics per function. All are present in JSON, NDJSON,
and CSV output.

Python Language Support
-----------------------

Knots supports Python (``.py``) alongside every other language listed by ``knots --supported-languages``.
All 13 metrics are computed. Python-specific notes:

- **McCabe**: counts ``if``, ``elif``, ``while``, ``for``, ``except``, ``and``,
  ``or``, ``match`` (3.10+), and ternary expressions
- **Cognitive**: ``elif`` is a flat ``+1`` (no nesting penalty); ``lambda``
  increments nesting depth without adding a base cost
- **SLOC**: ``#`` comment lines are excluded
- **External calls**: attribute-form calls (``obj.method()``, ``module.func()``)
  are counted as external references
- **Limitations**: ``for_in_clause`` inside comprehensions is not counted

JavaScript and TypeScript Language Support
------------------------------------------

Knots supports JavaScript (``.js``, ``.mjs``, ``.cjs``, ``.jsx``) and
TypeScript (``.ts``, ``.tsx``). All 13 metrics are computed. Notes:

- **McCabe**: counts ``if``, ``while``, ``do``, ``for``, ``for...in``,
  ``for...of``, ``switch``, ``ternary`` (``?:``), ``&&``, ``||``, and ``??``
  (nullish coalescing)
- **Cognitive**: ``for...in`` and ``for...of`` are treated as loop structures
  (+1 + nesting penalty); arrow functions (``() =>``) increment nesting depth
  without adding a base cost; ``??`` chains count once per contiguous sequence
  (same as ``&&``/``||``)
- **SLOC**: ``//`` and ``/* */`` comments are excluded (same as C/C++)
- **External calls**: member-expression calls (``obj.method()``,
  ``module.func()``) are counted as external references
- **Function discovery**: ``function`` declarations, ``function`` expressions,
  class methods, generator functions, and arrow functions are discovered.
  Arrow functions are reported when a name can be inferred from the assignment
  context — ``const foo = () => {}`` reports as ``foo``,
  ``{ bar: () => {} }`` reports as ``bar``,
  ``class C { baz = () => {} }`` reports as ``baz``.
  Truly anonymous callbacks (e.g. ``array.map(x => x * 2)``) are not reported.

.. note::

   **Nested function complexity attribution**: a function's metrics include
   all code within its body, including any nested function definitions.
   If ``helper`` is defined inside ``Parent``, ``helper``'s decision points
   count toward both ``helper``'s own score *and* ``Parent``'s score. Nested
   functions also appear as standalone entries in the output.

   This means a React component with several inline event handlers will show a
   higher McCabe score than an equivalent component with those handlers hoisted
   to module scope — even though the total logic is identical. The nested form
   is not "more complex" in a meaningful sense; it is an artifact of how
   complexity is attributed. If your goal is to gate on *structural* complexity
   rather than total lines of logic, use ``--cognitive-threshold`` as the
   primary gate: cognitive complexity adds a nesting increment for nested
   functions but does not fully sum their bodies into the parent the way McCabe
   does.

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

Preprocessor Dead-Code Exclusion (C/C++, Swift, C#)
-----------------------------------------------------

Before parsing, knots blanks out preprocessor branches that a compiler would
never see, so McCabe, Cognitive, Nesting, SLOC, and ABC are all computed only
from code that can actually run:

- **C/C++**: ``#if 0`` bodies; ``#ifdef __cplusplus`` / ``#if
  defined(__cplusplus)`` branches (dead when compiled as C); and ``#ifdef
  MACRO`` / ``#if defined(MACRO)`` branches where ``MACRO``'s definedness is
  locally provable from the file — unconditionally ``#define``\ d earlier
  with no later ``#undef`` (branch always live, its ``#else`` dead), or never
  validly ``#define``\ d in scope at that point (branch always dead). A macro
  the file never mentions at all (e.g. a build-system flag like ``_WIN32``)
  is left alone — there's no local evidence either way, and guessing would
  produce false exclusions.
- **Swift**: ``#if``/``#elseif``/``#else`` branches whose condition is a
  compile-time-constant ``false`` (``true``/``false`` literals, ``!``,
  ``&&``, ``||``, parens only — no ``#define`` in Swift).
- **C#**: the same two sub-problems as C/C++ (constant-condition branches and
  locally-provable ``#define``/``#undef`` symbol definedness), plus
  short-circuit ``&&``/``||`` evaluation for free, since C#'s conditions are
  a real expression tree rather than a flat token stream.

A dead branch's own ``#if``/``#else``/``#endif`` directive lines are never
blanked — only the code inside them — so line numbers and the surrounding
function's reported ``start_line``/``end_line`` are unaffected. Every other
language is unaffected; this only applies to the four listed here.

If a function's metrics look lower than they used to after upgrading past
knots 1.16.0, this is very likely why — see the "Metrics dropped after
upgrading" entry in :doc:`troubleshooting`.

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

   For Rust, method call syntax (``self.foo()``, ``vec.push()``) is counted
   via ``field_expression`` nodes. Method names defined locally are excluded;
   external method names (e.g. standard library, third-party crate methods)
   are counted as external references.

Unreachable Blocks
-------------------

Count of dead-code basic blocks: statements written directly after a
``return`` in the same block, which can never execute. Built on
``lang_parsing_substrate``'s control-flow-graph construction, which only
models ``c``/``cpp``/``rust`` — this metric is always ``0`` for every other
supported language.

- Threshold flag: ``--unreachable-blocks-threshold``
- No violation-count baseline yet; start at ``0`` (any dead code flags).

.. note::

   Only a block reached *exclusively* via a ``Return`` control-flow edge is
   flagged. A block reached via ``Break``/``Continue`` (a loop's after-block
   or header) is genuinely reachable when that jump fires and is never
   flagged, even though the loop itself has no implicit exit test.

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

.. note::

   **AIRD calibration for JavaScript / TypeScript / React projects**: the
   ceiling values (cognitive 75, SLOC 200, nesting 8) were derived from a
   C-heavy corpus (mosquitto, SQLite, curl, hostap, Lua, libcrc). JSX render
   functions and React components can accumulate high McCabe and cognitive
   scores from inline event handlers and render callbacks while staying under
   AIRD 85 — meaning the headline ``--aird-threshold 85`` gate may pass
   functions that are genuinely hard to maintain.

   For JS/TS/React projects, pair the AIRD gate with explicit McCabe and
   cognitive thresholds::

       --aird-threshold 85 --mccabe-threshold 20 --cognitive-threshold 25

   This catches both the AI-reasoning-difficulty axis (AIRD) and the
   traditional human-maintainability axis (McCabe/Cognitive). The thresholds
   above are a reasonable starting point; tighten them incrementally as the
   codebase improves.

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

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
  ``for...of``, each non-default ``case``, ``catch``, the conditional operator
  (``?:``), ``&&``, ``||``, ``??``, optional chaining (``?.``), logical
  assignment (``||=``, ``&&=``, ``??=``) and each default value (a parameter
  default or a destructuring default). ECMA-262 evaluates each of these
  conditionally, so each is a decision; ESLint's ``complexity`` rule counts
  the same set.
- **Cognitive**: ``for...in`` and ``for...of`` are loop structures (+1 plus
  the nesting penalty); ``break``/``continue`` to a label are +1; ``&&`` and
  ``||`` count once per sequence, and ``??`` and ``?.`` add nothing (the
  whitepaper's "Ignore shorthand" covers null-coalescing operators). Arrow
  functions, function expressions, and functions and methods declared inside
  a function increase the nesting level without an increment of their own.
  As the whitepaper's Appendix A specifies for JavaScript, an outer function
  whose top level holds only declarations (variables, functions, classes, or
  a function assigned to a name) is treated as a namespace, and the functions
  in it don't nest. A function that is itself an arrow function doesn't nest
  its own body.
- **Recursion**: a bare call resolves through the enclosing functions, and
  ``this.f()``/``this.#f()`` to the method's own class, where ``this`` is the
  method's own (inside an arrow function, not a nested ``function``).
- **SLOC**: ``//`` and ``/* */`` comments are excluded (same as C/C++). The
  lines of any function nested in another, an arrow callback included, are
  left out of the outer function's SLOC.
- **TypeScript**: type syntax adds nothing. Overload signatures,
  ``declare function``, interface members and abstract methods have no body
  and are not reported as functions.
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

Ada Language Support
--------------------

Knots supports Ada (``.adb``, ``.ada``, and ``.ads`` when passed explicitly).
All 13 metrics are computed. Ada needs explicit treatment in eight places that
the general-purpose rules do not cover:

- **Logical operators**: every ``and``, ``or``, and ``xor`` in an expression
  counts, with no distinction between the short-circuit forms (``and then``,
  ``or else``) and the plain ones. McCabe adds +1 per operator; Cognitive adds
  +1 per new operator in a sequence (``A and B and C`` is +1,
  ``(A or else B) xor C`` is +2); ABC adds one condition per operator.
- **Multi-name parameters**: ``A, B, C : Integer`` is three parameters, not
  one. Each identifier before the ``:`` in a ``parameter_specification`` is
  counted (state coupling, test scoring).
- **Return statements**: both ``simple_return_statement`` and
  ``extended_return_statement`` (``return R : T do ... end return;``) count
  toward the return count. Neither adds to McCabe.
- **Expression functions**: ``function F (X : T) return T is (expr);`` is
  discovered as a function (``expression_function_declaration``), alongside
  ``subprogram_body``.
- **Loops and case alternatives (McCabe)**: a ``while`` or ``for`` loop is a
  decision; a bare ``loop`` is not (its ``exit when`` is). Each ``when``
  alternative is +1 except ``when others``, the default, as in McCabe's
  definition.
- **Exit statements**: ``exit when Condition`` is +1 to McCabe, Cognitive (flat),
  and ABC (a condition). A bare ``exit`` adds nothing.
- **Raise**: ``raise`` statements and Ada 2012 raise expressions are one
  branch to ABC. They add nothing to McCabe (an unconditional raise is not a
  decision) or to Cognitive (the specification gives ``throw`` no increment).
- **Tasking**: task bodies (``task_body``) are discovered as functions, and
  subprograms inside a protected body are discovered too. In a ``select``,
  each ``select_alternative`` is +1 to McCabe, each guard
  (``when Condition =>``) is +1 to McCabe and +1 (flat) to Cognitive, and an
  ``else`` part is +1 to McCabe. The ``selective_accept`` itself, timed and
  conditional entry calls, and asynchronous select are Cognitive nesting
  structures (+1 plus the nesting penalty); the last three are also +1 to
  McCabe.
- **Else in Cognitive**: Ada's ``if`` has no separate else node, so knots adds
  a flat +1 for the ``else`` keyword of an ``if`` statement, as the
  specification charges for ``else``. ``elsif`` is a flat +1.

Other notes:

- **McCabe**: also counts ``if``, ``elsif``, every ``loop`` statement (plain,
  ``while``, and ``for``), each ``when`` alternative of a ``case``, and each
  exception handler
- **Cognitive**: ``loop``, ``case``, and exception handlers are nesting
  structures; a ``case`` costs +1 plus the nesting penalty once, however many
  alternatives it has
- **Nesting depth**: ``if``, ``loop``, ``case``, and exception handlers
  increase it; ``select`` does not
- **SLOC**: ``--`` comment lines are excluded
- **case_statement**: Ada and C both use the node kind ``case_statement``, for a
  whole ``case`` block in Ada and for one arm in C. Knots tells them apart by
  shape (an Ada ``case_statement`` has ``case_statement_alternative``
  children), so each gets its own treatment.
- **Limitations**: ``entry`` bodies in a protected body are not discovered as
  functions, and their code is not attributed to any other function.

.. note::

   **Case/dispatch inflation**: McCabe adds +1 for each ``when`` alternative, so
   a ``case`` used as a dispatch table scores its arm count whether the arms
   are one-line mappings or nested logic. Cognitive charges the ``case`` once.
   At knots 1.17.0, two dispatch functions in the HAC corpus (``Compute`` and
   ``Eval``, from an Advent of Code puzzle, ``~/toolchain/hac`` at ``b0fa2e5``)
   score McCabe 1,682 and Cognitive 5; the AES ``Encrypt`` functions in Ada-Util
   (``~/toolchain/ada-util`` at ``bd635f5``) score McCabe 119 and Cognitive 82
   from their SubBytes ``case`` statements.

   For Ada codebases, prefer ``--cognitive-threshold`` as the primary gate. If
   you gate on McCabe, derive the threshold from the project's own
   distribution: values set for C/C++ flag routine dispatch tables. A function
   whose McCabe is far above its Cognitive score is more likely a dispatch
   table than hard logic; review it before refactoring.

McCabe Cyclomatic Complexity
-----------------------------

Counts the number of linearly independent paths through a function.

- **Formula**: decision points + 1
- **Decision points**: ``if``/``elif``, ``while``, ``for``, ``do``,
  ``case``/``match`` arms, ternary, logical operators (``&&``/``||``,
  Python ``and``/``or``), each exception handler (``except``, ``catch``).
  knots 1.18.0 and earlier didn't count ``catch`` in C++, Java, C# or
  JavaScript/TypeScript
- **switch**: one decision per non-default ``case`` label, as in McCabe's
  definition (the SonarSource whitepaper's ``getWords``: three cases and a
  ``default`` give 4), in C, C++, PHP and JavaScript/TypeScript. knots 1.18.0
  and earlier counted a ``switch`` once (pmccabe's "modified" column). Ada
  counts each ``when`` alternative except ``when others``. **Known
  limitation:** the switch forms of Java, C#, Go, Kotlin, Swift and Scala
  still count 1 per switch until they have definition probes
- **Rust** ``match``: one decision per arm but the last (arms are tried in
  order and the match is exhaustive, so four arms are three tests, as in
  ``getWords``), plus one per guard (``if`` after a pattern) and one per
  ``|`` alternative in an arm's pattern, as ``case 1: case 2:`` is two in C.
  ``let ... else`` is one decision and ``?`` one. A bare ``loop`` has no test
  and adds nothing (the ``if`` that breaks out of it is the decision), as
  for Ada's bare ``loop``. knots 1.18.0 and earlier counted a ``match`` once
  and a ``loop`` as a decision, and didn't count ``let ... else``
- **Not decision points**: unconditional transfers (``goto``, ``throw``,
  ``raise``) add nothing, as in McCabe's definition and pmccabe
- **Preprocessor (C/C++)**: a function is scored as its worst real
  configuration (ADR-0002 §3). Arms the file proves dead are blanked before
  parsing; each remaining ``#if``/``#ifdef`` chain is one choice among its
  arms, chains on the same condition (``#ifdef X`` and a later ``#ifndef X``)
  are one choice, and McCabe is the maximum over the combinations, never a
  sum of arms no build compiles together. Conditions are compared as
  written, so logically linked conditions spelled differently count as
  independent. A function with more than 64 combinations (one in the six C
  corpora) keeps every arm counted. No build facts are needed. How a user
  could supply them to narrow the configurations is an open design question
  across all of knots' languages, not only C. knots 1.18.0 and earlier
  summed every arm. pmccabe keeps the
  first arm, which accounts for nearly all remaining differences
- **Thresholds**: ≤10 good, 11–20 moderate, 21+ consider refactoring
- **Validated**: agrees with pmccabe's "traditional" count (each ``case``
  counted, McCabe's definition) on 29,852 of 30,461 C functions (98.00%)
  across six corpora pinned by commit. 597 of the 609 differing functions
  contain ``#if`` arms (above). Most of the other 12 are macro artifacts: a
  ``switch`` whose ``case`` labels come from macros (neither tool sees them;
  knots counts the switch as one decision, pmccabe as none), or an ``if``
  inside a macro argument, which pmccabe's token scan counts and a parser
  cannot. Case labels that a statement-like macro makes the parser misread
  (sqlite's ``deliberate_fall_through``) are recovered. Reproduce with
  ``validation/pmccabe_compare.py`` (see ``validation/README.md``)

Cognitive Complexity
---------------------

Measures how difficult code is to understand, with higher weight for nesting
and structural complexity. Based on the
`G. Ann Campbell / SonarSource specification <https://www.sonarsource.com/resources/cognitive-complexity/>`_.

Key differences from McCabe:

- Nested structures add more than flat ones (nesting penalty)
- ``else``/``else if`` chains cost less than independent ``if`` chains
- ``switch`` is a single increment regardless of arm count
- The conditional operator (``?:``, Python's ``x if c else y``) is +1 plus
  nesting and nests what it contains, like ``if`` (whitepaper 1.7, Appendix B).
  knots 1.18.0 and earlier did not count it
- In C/C++ the ``cognitive`` column is the function's worst real
  preprocessor configuration, as for McCabe above

Conformance is checked by ``validation/probes`` (see
``validation/probes/README.md``): every known departure from the
specification is listed, with its reason, in
``validation/probes/conformance.toml``. Follows the specification
(whitepaper 1.7, 2023) on two points other implementations skip:

- **Recursion**: +1 for each function in a recursion cycle, direct or
  indirect (Appendix B1), once per function, not per call. Cycles are found
  among the functions of one file, from the call syntax alone: a bare call
  ``f()``, a call through ``self``/``this``, and in Rust ``Self::f`` or
  ``Type::f`` inside ``Type``'s impl (called or passed as a value). A method
  called on any other receiver (``child.depth()``) is not followed, since
  its type isn't known; following it by name scored a third of Rust
  functions as recursive, almost all of them delegations like
  ``self.0.fmt(f)``. A cycle through another file is not seen either. In
  Rust a bare call resolves through the enclosing ``fn`` and ``mod``
  scopes, so same-named functions in different modules are kept apart.
- **break/continue to a label** (Rust ``break 'outer``): +1 (Appendix B1).
  A plain ``break`` or ``continue`` adds nothing.
- **throw/raise**: no increment (Appendix B1; Appendix C scores every
  ``throw`` 0). ``goto`` keeps its +1.

Compared against Mozilla's
`rust-code-analysis <https://github.com/mozilla/rust-code-analysis>`_: a
1.004× mean ratio over **17 matched high-complexity functions**, drawn from
11,365 Rust functions scanned (285k lines; the corpus was not pinned by
commit), measured before knots added the recursion increment, which
rust-code-analysis omits. Agreement with another implementation is
evidence, not ground truth: on the Rust probes rust-code-analysis also
scores the whitepaper's own logical-operator example 5 instead of 4 and a
bare ``loop`` with a nested ``if`` 1 instead of 3 (``validation/probes/TOOLS.md``).

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
Documentation   Comment quality (0 to 10, subtracted from the total score)
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
         + (state_coupling/12 × 10)

Each ratio is capped at 1 before it is weighted (a negative ``test_score`` or
``doc_score`` counts as 0), and the sum is rounded and clamped to 0–100.
``state_coupling`` is the explicit parameter count plus the number of
distinct ``self``/``this`` fields the function touches.

AIRD reads the source **as written**. Its ``cognitive`` input (and the McCabe
behind ``test_score``) counts every live ``#if`` arm, where the ``mccabe`` and
``cognitive`` columns report the worst real configuration. McCabe and
Cognitive Complexity are published definitions of one program; AIRD models an
AI working on the text, which reads, greps and cuts the file with every arm
in it and has to work out which arm a change concerns. AICP's inputs (SLOC,
external calls) are as written too. Whether the number of configurations
itself adds difficulty is a question for the validation study.

With ``--recursive``, the clamped score is then multiplied by a file-level
coupling factor and rounded and clamped again:

.. code-block:: text

    multiplier = 1 + min(file_ce/10, 1) × 0.20        (1.0 to 1.2)
    AIRD       = clamp(round(AIRD_base × multiplier), 0, 100)

``file_ce`` is the number of corpus-internal files the function's file
imports. Outside ``--recursive`` it is 0, the multiplier is 1.0, and AIRD is
the base score. ``--score-components`` reports every term (see
`Score components`_).

How an import becomes a file
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

For most languages an import resolves by name: its path prefix, a known
source extension and any ``::``/``.`` qualifier are stripped, and what is
left must match the stem of exactly one corpus file (``#include "util.h"``
finds ``util.h``; ``import com.example.Helper`` finds ``Helper.java``). A
name two corpus files share, or none, adds no edge, and neither does a
library outside the corpus.

Rust resolves through the crate's module tree instead, because a ``use``
usually names an item or a group (``use crate::a::{B, c::D}``), not a
module, and every ``mod.rs`` shares one stem. Each ``use`` is expanded to
one path per imported name (an alias or a glob counts as its path), and
each path resolves to the deepest *module file* it passes through:
``crate::coupling::FileCoupling`` counts ``coupling.rs``, and
``crate::Root`` counts the crate root (``lib.rs``). A path may start with
``crate``, ``self``, ``super``, a child module (edition 2018), or the
library name of any analyzed package, which is how ``main.rs`` or a test
reaches its library and one workspace member reaches another. A library
name two analyzed packages share (vendored copies, fixtures) names
neither. Module ``a::b`` is ``a/b.rs`` or ``a/b/mod.rs`` below the crate
root's directory, and a file there counts as the module whether or not a
``mod b;`` declares it. Another crate's root file (``tests/x.rs``,
``src/bin/x.rs``) never counts as a module.

Crate roots come from the nearest ``Cargo.toml``, read only for the
library's name, the declared ``[lib]`` and ``[[bin]]`` paths and the build
script: no build and no ``cargo metadata``. Cargo's defaults add
``src/lib.rs``, ``src/main.rs`` and every file directly in ``src/bin``,
``tests``, ``examples`` or ``benches`` (or a ``main.rs`` one directory below
them). A file below one of those directories with no ``main.rs`` of its own
(``tests/common/mod.rs``) is a module the target's roots share, so a path
from it reaches the shared modules but never a single test's root. When
``lib.rs`` and ``main.rs`` share ``src/``, a module only ``main.rs``
declares (``mod cli;``) belongs to the binary; that is the one place the
roots' ``mod`` lines are read.

A ``.rs`` file keeps the name match above when it is under no
``Cargo.toml``, when its crate root is not among the analyzed files (so
``knots -r crate/src/subdir`` resolves by name, as before), when it sits
under a ``Cargo.toml`` but outside every crate root's directory (e.g.
``pkg/scripts/x.rs``), or when it is the package's build script, which is a
crate of its own.

What counts: every ``use`` in the file, including those in its
``#[cfg(test)] mod tests`` block, as a C ``#include`` under ``#ifdef TEST``
counts. What does not: a ``mod`` declaration (it builds the tree; counting
it would give every ``lib.rs`` a Ce equal to its submodule count), ``std``
and other external crates, and the file's own inline modules, so
``use super::*`` in that test block is the file itself. ``#[path]``
attributes are not followed, an inline ``mod x { ... }`` has no file of its
own to count, and edition-2015 paths (``use y::Thing`` meaning
``crate::y::Thing``) are not modelled, so they add no edge.

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

**Evidence so far is a pilot, not a validation.** Sonnet 4.6 and Opus 4.8
rated nine functions (three per AIRD band) on one task. The top band
separated, the middle band did not, and one function falsified the gate:
``hostapd_config_read_eap_user`` scored the highest AIRD in the pilot and
was rated as easy as the low band. A pre-registered validation study is
designed but not yet run.

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


.. _score-components:

Score components
----------------

AIRD and AICP each report only their final, clamped score, and in
``--recursive`` mode AIRD also includes the file-level multiplier. To
decompose a score, for example to test one term against SLOC or cognitive
complexity, or to see which cap a saturated score is pinned by, pass
``--score-components`` with ``--format json``, ``ndjson`` or ``csv``. The
default fields stay as they are; CSV gains these as trailing columns, JSON
and NDJSON as extra keys (JSON key order is not significant):

=============================  =====================================================
Column                         Meaning
=============================  =====================================================
``state_coupling``             Input to the AIRD coupling term (parameters + fields)
``aird_cognitive``             ``min(cognitive/75, 1) × 55``
``aird_sloc``                  ``min(sloc/200, 1) × 15``
``aird_nesting``               ``min(nesting/8, 1) × 15``
``aird_test``                  ``min(max(test_score, 0)/20, 1) × 15``
``aird_doc``                   ``-min(max(doc_score, 0)/10, 1) × 15`` (≤ 0)
``aird_coupling``              ``min(state_coupling/12, 1) × 10``
``aird_raw``                   Sum of the six ``aird_*`` terms, before rounding and
                               the 0–100 clamp; always within -15 to 110
``aird_uncapped_raw``          As ``aird_raw`` but with the cognitive and SLOC ratios
                               uncapped (nesting stays capped), floored at 0: the
                               "raw AIRD (uncapped)" the text breakdown prints for
                               large functions
``aird_base``                  ``aird_raw`` rounded and clamped to 0–100: AIRD before
                               the file-level multiplier
``aird_file_ce_multiplier``    ``1 + min(file_ce/10, 1) × 0.20``; 1.0 outside
                               ``--recursive``
``aicp_external_calls``        ``min(external_calls/20, 1) × 60``
``aicp_sloc``                  ``min(sloc/200, 1) × 40``
``aicp_doc``                   ``-min(max(doc_score, 0)/10, 1) × 15`` (≤ 0)
``aicp_raw``                   Sum of the three ``aicp_*`` terms, before rounding and
                               the 0–100 clamp
=============================  =====================================================

**Two kinds of saturation.** ``aird_raw`` answers "how far past the 0–100
output clamp is this score?" Because every term is capped before it is
summed, it can exceed 100 by at most 10 and rarely does. ``aird_uncapped_raw``
answers "how far past the cognitive and SLOC caps is this function?", which is
usually what pins a score at or near its ceiling. It can exceed 100 while
``aird_base`` is below 100, and it reads 0 wherever ``aird_raw`` is negative,
so it is not a pre-clamp value at the low end.

**Recomputing a score.** With the emitted values, IEEE-754 double arithmetic
and Rust's rounding (``f64::round``, half away from zero), every row satisfies:

.. code-block:: text

    aird = clamp(round(aird_base × aird_file_ce_multiplier), 0, 100)
    aird_base = clamp(round(aird_raw), 0, 100)
    aicp = clamp(round(aicp_raw), 0, 100)

The component columns are written at full precision in all three formats
(CSV included), because many raw sums land within floating-point error of a
.5 tie. For those rows, round-half-to-even (the default in Python, NumPy and
R), decimal arithmetic, or recomputing the terms from the default integer
columns can give a different score than knots reports. **Analyses should use
the emitted** ``aird``, ``aird_base`` **and** ``aicp`` **as outcomes, not
recompute them.**

**The terms are not independent inputs.** ``test_score`` includes an
implementation sub-score mapped from McCabe complexity and already subtracts
``doc_score``. So McCabe feeds AIRD through ``aird_test``, documentation
counts twice (inside ``aird_test`` and again as ``aird_doc``), and SLOC and
cognitive complexity are themselves terms. A model that regresses AIRD on
SLOC, cognitive complexity or McCabe has those predictors partly inside the
outcome. In practice ``doc_score`` takes only a few values (0, 2 and 4 are
typical), so ``aird_doc`` is usually 0, -3 or -6.

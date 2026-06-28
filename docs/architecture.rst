Architecture
============

This document describes knots' internal structure, its key invariants, and
the known coupling hotspots and extensibility concerns as of v1.12.0.

.. contents:: Contents
   :local:
   :depth: 2

---------------------------------------------------------------------------

Overview
--------

knots is a single Rust binary backed by a small library.  The workspace has
one additional member — ``knots-test-complexity`` — which is a separate CLI
that reuses the library's metric calculations for a different reporting use
case.

The codebase has three conceptual layers:

::

    ┌────────────────────────────────────────────────────────────┐
    │  CLI + pipeline  (src/main.rs, ~3 450 lines)               │
    │  file discovery · function discovery · threshold enforcement│
    │  baseline · git diff integration                           │
    ├────────────────────────────────────────────────────────────┤
    │  Output formatting  (src/output.rs, ~770 lines)            │
    │  text/SARIF/JSON/NDJSON/CSV · AIRD display helpers        │
    ├────────────────────────────────────────────────────────────┤
    │  Metric calculation  (src/complexity.rs, ~3 300 lines)     │
    │  pure tree-sitter traversal — no I/O                       │
    ├────────────────────────────────────────────────────────────┤
    │  Language registry  (src/lib.rs, ~210 lines)               │
    │  LANGUAGES table · SUPPORTED_EXTENSIONS · grammar dispatch │
    └────────────────────────────────────────────────────────────┘

---------------------------------------------------------------------------

Layer 1 — Language Registry (``src/lib.rs``)
--------------------------------------------

``LANGUAGES`` is the single source of truth for all supported languages.
``SUPPORTED_EXTENSIONS`` mirrors its recursive-discovery extensions; a test
in the same file enforces that the two never drift.

``language_for_file(path)`` maps a file extension to a tree-sitter
``Language``.  ``is_source_extension`` / ``is_parseable_extension`` gate
recursive discovery vs. explicit-file parsing respectively (e.g. ``.h`` and
``.ads`` are parseable but not discovered recursively).

All grammars are re-exported as ``pub use tree_sitter_*`` so tests and
workspace members can reach them without adding their own direct dependencies.

---------------------------------------------------------------------------

Layer 2 — Metric Calculation (``src/complexity.rs``)
-----------------------------------------------------

Each metric follows the same shape::

    pub fn calculate_X(node: Node, source_code: &[u8]) -> T {
        // initialise accumulator
        visit_node_X(node, source_code, &mut acc);
        acc
    }

    fn visit_node_X(node: Node, ...) {
        match node.kind() { /* language-neutral node-kind strings */ }
        for child in node.children(...) { visit_node_X(child, ...) }
    }

Metrics implemented: McCabe, Cognitive, Nesting, SLOC, ABC, Return count,
TestScoring, ExternalCalls, StateCoupling, AIRD, AICP.

SLOC dispatch
~~~~~~~~~~~~~

There are five SLOC variants, selected by comment style:

- ``calculate_sloc`` — ``//`` and ``/* */``  (C, C++, Rust, JS, TS, Go, …)
- ``calculate_sloc_python`` — additionally skips lines starting with ``#``
- ``calculate_sloc_ada`` — skips ``--`` lines
- ``calculate_sloc_lua`` — skips ``--`` lines (identical logic to Ada)
- ``calculate_sloc_fortran`` — skips ``!`` lines

Ada and Lua share the same ``--`` comment style but have separate
implementations (copy-paste duplication — see :ref:`known-issues`).

Selection happens in ``main.rs`` via ``is_python`` / ``is_ada`` /
``is_fortran`` / ``is_lua`` boolean flags derived from file extension.

TestScoring note
~~~~~~~~~~~~~~~~

``calculate_test_scoring`` includes a ``calculate_signature_complexity``
sub-function that walks the tree looking for ``function_definition`` >
``declarator`` (C-style node structure) to count parameters.  This heuristic
produces unreliable results for non-C languages; the signature score
component of TestScoring should be treated with caution for Java, Kotlin,
Swift, Scala, PHP, etc.

---------------------------------------------------------------------------

Layer 3a — CLI + Pipeline (``src/main.rs``)
-------------------------------------------

~3 450 lines. Contains:

- **CLI parsing** — ``Args`` struct (clap derive), ``ExplainMetric``,
  ``OutputFormat`` enums
- **File discovery** — ``collect_files``, ``load_compile_commands``
- **Function discovery** — ``visit_functions``, ``get_function_name``,
  ``collect_local_names_recursive``, ``is_function_kind``
- **Metric pipeline** — ``collect_function_metrics``, ``collect_all_metrics``
- **Threshold enforcement** — ``check_thresholds``, ``check_u32_threshold``,
  ``check_f64_threshold``
- **Baseline I/O** — ``BaselineEntry``, ``Baseline``, ``write_baseline``,
  ``baseline_from_metrics``
- **Git integration** — ``collect_changed_lines``, ``ChangedLines``,
  ``parse_hunk_new_range``
- **Data model** — ``FunctionMetrics`` (private struct, used throughout)

Layer 3b — Output Formatting (``src/output.rs``)
-------------------------------------------------

~770 lines. Contains all display and serialization functions extracted from
``main.rs`` (task 43):

- **Output formatting** — ``analyze_code`` (text/single-file),
  ``display_recursive_summary``, ``display_testability_matrix``,
  ``emit_sarif``, ``emit_json``, ``emit_ndjson``, ``emit_csv``,
  ``write_detailed_report``
- **AIRD display helpers** — ``format_aird_breakdown``, ``aird_drivers``,
  ``aird_tips``, ``aird_term``, ``ai_metric_pointer``
- **Utility** — ``get_complexity_emoji``, ``func_location``, ``sarif_level``,
  ``path_to_sarif_uri``

Pipeline flow
~~~~~~~~~~~~~

::

    main()
      → collect_files() / load_compile_commands()
      → parse_file()                  ← grammar from language_for_file()
      → collect_function_metrics()
          → visit_functions()         ← discovers function nodes
          → get_function_name()       ← extracts name per language
          → calculate_*()             ← all metrics, one pass per function
          → should_process_function() ← apply include/exclude filters
      → [output mode dispatch]
      → check_thresholds()

Output mode dispatch
~~~~~~~~~~~~~~~~~~~~

After building ``RunContext``, ``main()`` routes to one of:

- ``run_single_file_mode`` — calls ``analyze_code`` (prints per-function
  lines) then ``collect_function_metrics`` again for threshold checking
- ``run_multi_file_mode`` — collects across files, summary display, thresholds
- ``run_matrix_mode`` — testability matrix display, thresholds
- ``run_structured_output_mode`` — SARIF / JSON / NDJSON / CSV via
  ``collect_all_metrics``

.. _known-issues:

---------------------------------------------------------------------------

Known Issues and Coupling Hotspots
-----------------------------------

The items below are not bugs — knots is correct.  They are structural debts
that will become friction as the language count grows.

1. **main.rs monolith** *(partially resolved)*
   Output formatters and display functions were extracted into
   ``src/output.rs`` (task 43), reducing ``main.rs`` from ~4 100 to
   ~3 450 lines.  A further split of function-discovery helpers into
   ``src/discovery.rs`` would further reduce coupling.

2. **FunctionMetrics not exported from lib**
   ``FunctionMetrics`` is a private struct in ``main.rs``.
   ``knots-test-complexity`` therefore cannot reuse it and maintains its own
   parallel metric pipeline.  Moving ``FunctionMetrics`` (and
   ``collect_function_metrics``) into ``lib.rs`` would unify the two
   codepaths and let workspace members build on a shared data model.

3. **Double metric calculation in single-file mode**
   ``run_single_file_mode`` calls ``analyze_code()``, which internally calls
   ``collect_function_metrics()``, and then calls ``collect_function_metrics``
   a second time for threshold checking.  For large files this doubles parse
   and metric computation work unnecessarily.

4. **Ada/Lua SLOC duplication**
   ``calculate_sloc_ada`` and ``calculate_sloc_lua`` are byte-for-byte
   identical (both skip ``--`` comment lines).  They should share a private
   implementation.

5. **Language-specific booleans scattered across main.rs**
   ``is_python``, ``is_ada``, ``is_fortran``, ``is_lua`` booleans are
   derived from file extension strings at least twice in ``main.rs``
   (``collect_function_metrics`` and ``accumulate_nested_sloc``).  Each new
   language with a non-default SLOC mode requires another string match in
   multiple places.  The right fix is to derive the SLOC mode from the
   grammar/language type, not from extension strings.

6. **``collect_local_names_recursive`` must mirror ``visit_functions``**
   This is documented as an invariant in ``CLAUDE.md`` but there is no test
   enforcing it.  Any function node kind added to ``visit_functions`` without
   a matching addition to ``collect_local_names_recursive`` silently
   misclassifies locally-defined functions as external calls.

7. **Hand-rolled glob matching**
   ``glob_match()`` converts a glob pattern to a regex by string replacement.
   The ``ignore`` crate (already a dependency) provides glob matching;
   ``glob_match`` is redundant and subtly incomplete (e.g. ``?`` is not
   handled).

8. **``calculate_signature_complexity`` is C-specific**
   The sub-function that scores function signature complexity (used inside
   ``calculate_test_scoring``) searches for ``function_definition`` >
   ``declarator`` node structure, which is C/C++-only.  Other languages will
   produce a zero or near-zero signature score regardless of parameter count.
   This makes the TestScoring metric unreliable as a cross-language signal.

9. **No parallel file processing**
   File analysis is sequential.  For large recursive analyses (hundreds of
   files) this is a visible bottleneck.  ``rayon`` or ``std::thread`` would
   be straightforward to add since each file is independent.

---------------------------------------------------------------------------

What Works Well
---------------

- **LANGUAGES / SUPPORTED_EXTENSIONS sync test** — a compile-time-ish guard
  that prevents the extension list from drifting.
- **Baseline design** — keyed on ``(file, function)``, deliberately omitting
  line numbers, so the baseline stays stable as code moves.
- **``--since`` / ``--changed`` git integration** — cleanly isolated in
  ``ChangedLines``; does not touch the metric calculation path.
- **AIRD/AICP violation output** — breakdown line, driver hint, dual-cap tip,
  and raw-AIRD tracking are all well-thought-out for the refactoring loop.
- **``args_override_self = true``** — allows CI wrappers to append an
  override occurrence of a threshold flag without erroring; documented with a
  comment.
- **Metric layer purity** — ``complexity.rs`` has zero I/O and is trivially
  testable in isolation.

---------------------------------------------------------------------------

Adding a Language — Quick Reference
------------------------------------

See ``CLAUDE.md`` in the project root for the step-by-step checklist.  The
five files to touch in order are:

1. ``Cargo.toml`` — add the crate
2. ``src/lib.rs`` — register in LANGUAGES, SUPPORTED_EXTENSIONS,
   language_for_file, re-export
3. ``src/complexity.rs`` — add node kinds to each visitor
4. ``src/main.rs`` — visit_functions, get_function_name,
   collect_local_names_recursive
5. ``src/main.rs`` — add discovery tests

Run ``invoke sync-languages --write`` after step 2 to propagate into docs.

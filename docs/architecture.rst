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
    │  CLI + pipeline  (src/main.rs, ~2 650 lines)               │
    │  file discovery · function discovery · threshold enforcement│
    │  baseline · git diff integration                           │
    ├────────────────────────────────────────────────────────────┤
    │  Output formatting  (src/output.rs, ~750 lines)            │
    │  text/SARIF/JSON/NDJSON/CSV · AIRD display helpers        │
    ├────────────────────────────────────────────────────────────┤
    │  Metric calculation  (src/complexity.rs, ~3 300 lines)     │
    │  pure tree-sitter traversal — no I/O                       │
    ├────────────────────────────────────────────────────────────┤
    │  Language registry + data model  (src/lib.rs, ~980 lines)  │
    │  LANGUAGES table · SUPPORTED_EXTENSIONS · grammar dispatch │
    │  FunctionMetrics · collect_function_metrics · FilterRules  │
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

``lib.rs`` also owns the shared data model:

- ``FunctionMetrics`` — the output record of one analysed function (all
  metric fields, file path, line range).
- ``collect_function_metrics`` — runs ``visit_functions`` and all
  ``calculate_*`` calls for every function in a parsed tree.
- ``FilterRules`` — include/exclude filters (file globs via ``globset``,
  function regexes, complexity bounds) loaded from a JSON sidecar.

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
TestScoring, ExternalCalls, StateCoupling, AIRD, AICP, UnreachableBlocks
(C/C++/Rust only, built on the substrate's CFG rather than a direct AST walk).

SLOC dispatch
~~~~~~~~~~~~~

There are five SLOC variants, selected by comment style:

- ``calculate_sloc`` — ``//`` and ``/* */``  (C, C++, Rust, JS, TS, Go, …)
- ``calculate_sloc_python`` — additionally skips lines starting with ``#``
- ``calculate_sloc_ada`` — skips ``--`` lines (delegates to shared helper)
- ``calculate_sloc_lua`` — skips ``--`` lines (delegates to same helper)
- ``calculate_sloc_fortran`` — skips ``!`` lines

Ada and Lua share a private ``calculate_sloc_line_comment(node, src, prefix)``
helper parameterised by the comment-prefix byte sequence.

Selection is driven by ``SlocMode``, a field on ``LanguageInfo`` in the
``LANGUAGES`` table.  ``sloc_mode_for_file(path)`` performs a single
table lookup; ``collect_function_metrics`` dispatches on the result via
``match``.  No per-language boolean flags are needed.

TestScoring note
~~~~~~~~~~~~~~~~

``calculate_test_scoring`` includes a ``calculate_signature_complexity``
sub-function that scores parameter count using ``count_explicit_params``.
``count_explicit_params`` is language-neutral and handles all 16 supported
languages, so the signature component of TestScoring is reliable across the
full language set.

---------------------------------------------------------------------------

Layer 3a — CLI + Pipeline (``src/main.rs``)
-------------------------------------------

~2 650 lines. Contains:

- **CLI parsing** — ``Args`` struct (clap derive), ``ExplainMetric``,
  ``OutputFormat`` enums
- **File discovery** — ``collect_files``, ``load_compile_commands``
- **Function discovery** — ``visit_functions``, ``get_function_name``,
  ``collect_local_names_recursive``, ``is_function_kind``
- **Metric pipeline** — ``collect_all_metrics``
- **Threshold enforcement** — ``check_thresholds``, ``check_u32_threshold``,
  ``check_f64_threshold``
- **Baseline I/O** — ``BaselineEntry``, ``Baseline``, ``write_baseline``,
  ``baseline_from_metrics``
- **Git integration** — ``collect_changed_lines``, ``ChangedLines``,
  ``parse_hunk_new_range``

``FunctionMetrics`` and ``collect_function_metrics`` live in ``lib.rs`` and
are imported here.

Layer 3b — Output Formatting (``src/output.rs``)
-------------------------------------------------

~750 lines. Contains all display and serialization functions:

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
      → collect_function_metrics()    ← lib.rs; called once per file
          → visit_functions()         ← discovers function nodes
          → get_function_name()       ← extracts name per language
          → calculate_*()             ← all metrics, one pass per function
          → should_process_function() ← apply include/exclude filters
      → [output mode dispatch]
      → check_thresholds()

Output mode dispatch
~~~~~~~~~~~~~~~~~~~~

After building ``RunContext``, ``main()`` routes to one of:

- ``run_single_file_mode`` — calls ``collect_function_metrics`` once, passes
  the result slice to ``analyze_code`` (display) and ``check_thresholds``
- ``run_multi_file_mode`` — collects across files, summary display, thresholds
- ``run_matrix_mode`` — testability matrix display, thresholds
- ``run_structured_output_mode`` — SARIF / JSON / NDJSON / CSV via
  ``collect_all_metrics``

.. _known-issues:

---------------------------------------------------------------------------

Tool Suite Vision
-----------------

knots is one of three planned analysis tools sharing a common language-parsing
substrate:

- **knots** — code metrics lens (McCabe, Cognitive, SLOC, ABC, AIRD, AICP, …)
- **sqc** — CERT-C compliance / security-finding lens (the primary motivation
  for the substrate)
- **funky** — code formatting lens

Each tool asks a different question of the same parsed representation.  This
mirrors the architecture of tools like Understand (SciTools), CodeQL, and
Kythe, all of which converged on the same pattern: *parse once, query many
times.*  The parse is the expensive, language-specific step; analyses are
cheaper queries against a persistent store.

The Shared Substrate (``parse-db``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The substrate is a planned ``parse-db`` (working name) crate that would own:

- ``LanguageRegistry`` — already in ``lib.rs``; promote to shared crate
- ``FileRecord`` — path, detected language, tree-sitter tree, source bytes
- ``SymbolTable`` — definitions and references, keyed by ``NodeId``
- ``ImportGraph`` — directed edges ``(file → imported_file)``, built from
  syntactic import nodes (see below)
- ``CallGraph`` — ``(caller_fn, callee_fn, is_external)`` edges

**Storage model.** In-memory reconstruction on each invocation is acceptable
for knots today (single-file or small corpora).  sqc analysing a large C
codebase demands incremental re-parse keyed on ``(path, mtime/hash)``; SQLite
is the natural choice (embeddable, queryable, used by Understand and CodeQL for
the same reason).

**Import graph depth.** Coupling analysis only needs *syntactic* import
counting — count distinct import-source nodes per file from the tree-sitter
AST, no path resolution required.  This is far shallower than full symbol
resolution and sufficient for the Ce/Ca/Instability signal.  Full resolution
(following re-exports, handling conditional includes) is deferred until sqc
demands it for taint flow.

Relationship to LSP
~~~~~~~~~~~~~~~~~~~

A Language Server Protocol server has significant overlap with the substrate:
both parse files, maintain cross-file symbol tables, and support incremental
re-parsing.  The differences are fundamental:

- **Execution model.** LSP is a JSON-RPC protocol optimised for interactive
  single-query latency (IDE hover, completion).  The substrate serves batch
  analysis — build the full corpus model once, then run many queries.
- **Query surface.** LSP exposes a fixed set of request types
  (``textDocument/hover``, ``workspace/references``, etc.).  The substrate
  exposes arbitrary structured queries tuned to each tool's analysis needs.
- **Language coverage.** A 16-language suite would require 16 separate LSP
  servers with wildly varying capability levels (clangd is rich; most
  tree-sitter LSPs are thin).  The substrate provides one consistent API
  across all languages, which is knots' existing value proposition.
- **Ownership.** LSP servers are external dependencies whose analysis depth is
  bounded by their authors' choices.  sqc performing taint flow and CERT-C
  checking needs depth most LSP servers do not expose.

The closest real-world analogue is ``rust-analyzer`` extracted as a library
(the ``ra_ap_*`` crates) — but that is Rust-only.  The substrate here must be
language-neutral at the tree-sitter level.

Coupling and the AI Metrics
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Efferent coupling (Ce), afferent coupling (Ca), and Instability
(``Ce / (Ca + Ce)``) are Robert Martin's module-level dependency metrics.
They connect directly to knots' AI-focused metrics:

**AIRD (AI Reasoning Difficulty)** measures how much context an AI must load
to reason correctly about a function.  Coupling is a direct multiplier:

- A function in a file with high Ce depends on many other files; the AI must
  understand or infer all those external contracts.  High Ce = wide context
  requirement = higher AIRD.
- The existing ``state_coupling`` metric already captures *intra-function*
  breadth (distinct fields and parameters touched).  File-level Ce captures
  *inter-module* breadth — the same signal one abstraction level up.
- The existing ``external_calls`` metric is already a function-level Ce proxy
  and contributes to AIRD today.

**AICP (AI Code Prediction)** measures how predictable/completable the code
is.  High Ca (many dependents) correlates with stable, well-established
interfaces the AI has seen repeatedly — lower completion cost.  High Ce means
the function's behaviour is contingent on many external contracts not in the
local context window — higher completion cost.

Because coupling feeds into AIRD and AICP it belongs in knots rather than a
separate tool.  The substrate provides the import graph; knots consumes it as
one more input to the AI metrics and also surfaces Ce/Ca/Instability as
first-class per-file metrics.  The two-phase execution model this requires
(phase 1: crawl all files and build the import graph; phase 2: compute metrics
including Ce/Ca) is a natural extension of the existing ``collect_all_metrics``
pipeline under ``--recursive``.

Duplicate code detection (``--find-duplicates``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The substrate's ``fingerprint`` module hashes each function-like subtree's
shape (node kinds, ignoring identifier/literal text) so renamed or
re-parameterized copies of the same code still match — Type-1 and Type-2
clone detection.  Unlike Ce/Ca, this doesn't feed any per-function metric; it
runs as its own opt-in corpus pass (a second parse of every file) gated
behind ``--find-duplicates``, only meaningful alongside ``--recursive``, and
surfaced as a standalone ``DUPLICATE CODE`` section in the text report rather
than folded into per-function output.

Groups whose members are entirely a ``tests/pass`` vs ``tests/fail``-style
fixture pair (or ``compliant``/``noncompliant``, ``good``/``bad``,
``accept``/``reject``, ``valid``/``invalid``) are excluded by default and
counted in a summary line — these are intentionally near-identical
compliant/non-compliant examples (as in CERT-C test suites), not extraction
candidates. Pass ``--include-fixture-pairs`` to see them anyway. The
heuristic only fires when *every* member of a group sits under one of these
directory names and normalizes to the same path otherwise; a group that also
contains a genuine third duplicate is left untouched.

Groups where every member's body spans 3 lines or fewer (``TRIVIAL_BODY_LINE_SPAN``)
and the group has fewer than 4 members (``TRIVIAL_MIN_REPEAT``) are likewise
excluded by default, counted in a separate summary line. ``MIN_DUPLICATE_NODES``
(the AST-node floor) doesn't catch this case on its own: a one-line accessor
like ``fn src(&self) -> &str { self.src }`` can clear 20 AST nodes from type
annotations and field-access chains alone while still being a single line of
source. This matters most right after a real duplication fix — the
irreducible per-type accessor glue a trait-extraction produces shows up as
brand-new duplicate groups on the next run, incorrectly reading as
unresolved debt. A getter repeated 4+ times is kept regardless of size, since
that many repeats is more likely deliberate copy-paste than the unavoidable
byproduct of a single refactor. Pass ``--include-trivial-duplicates`` to see
these groups anyway.

Every non-first member of a reported group also gets a byte-diff annotation
against the group's first member: ``[byte-identical]`` for a true Type-1
clone (0% divergence — high confidence, safe to act on without opening the
files), ``[N% diff from #1]`` for a Type-2 clone or shape coincidence (renamed
identifiers, changed literals, or a same-shaped-but-different-purpose
function — worth a human look before merging), or a "too large or
unreadable" note when the body exceeds ``MAX_DIFF_CHARS`` (20,000 characters)
and the O(n\ :sup:`2`\ ) edit-distance pass was skipped. This is computed by
re-reading each member's exact byte range from disk and running a
Levenshtein edit-distance pass — separate from, and a finer-grained signal
than, the AST-shape hash that grouped them in the first place.

Each group is also tagged with a short hex ID (e.g. ``[a1b2c3d4]``) derived
from the structural-shape hash every member already shares — the same key
``duplicate_groups`` grouped on. Positional numbering (1, 2, 3...) reshuffles
between runs as files change and group sizes shrink or grow; the hex ID
doesn't, since it depends only on the duplicated shape, not which files or
how many currently exhibit it. That makes two reports diffable directly: the
same ID reappearing with fewer members confirms a group shrank after a
refactor, and an ID that vanishes entirely confirms it was fully resolved.

Interpreting results
^^^^^^^^^^^^^^^^^^^^

The filters and annotations above cut the obvious noise, but the output
still needs a human in the loop. What follows is guidance distilled from
actually acting on a real report end to end (see ``MOLDY_DUP_FEEDBACK.md``
in the repo root) — a 5.9k-line, 4-file Rust codebase where 16 reported
groups broke down as 3 genuinely worth fixing, several trivial-boilerplate
groups, and 2 that looked identical but were a trap to merge.

**Shape equality is not behavioral equality.** The hash matches AST node
kinds, not identifiers, literals, or intent. A 3-line test function that's
just one ``assert_eq!`` and a 3-line dispatch stub that forwards to a
differently-named per-language handler can hash identically to each other
purely because tree-sitter sees the same shape — read the matched bodies
before trusting a group, especially a small one that survived the trivial
filter by repeating 4+ times.

**Group size is the strongest triage signal.** A group with a handful of
members and tens of AST nodes is usually either fixture noise or a
coincidence; a group with a large ``~N AST nodes each`` figure or many
members (especially 3+ files) is the highest-signal finding and the one
most likely to represent real, worth-extracting duplication.

**"Identical AST across files" does not imply "safe to merge into one
type."** Two byte-identical groups can still be unsafe to unify: if merging
requires the matched functions' *enclosing types* to become a single type,
that breaks the moment those types have same-named-but-different-bodied
sibling methods elsewhere in their ``impl`` surface (e.g. two formatters
that both define ``emit_node``/``ws_before`` with the same name and
different bodies — fine as separate types, a collision if merged into one).
This isn't visible from the matched function alone. Before proposing a type
merge, check the rest of each type's inherent-impl surface for
same-named-but-different-bodied methods; when in doubt, the safe default
suggestion is delegation (a shared trait or helper the types each call),
not unification into one concrete type.

**The tool doesn't know whether an extraction will typecheck.** Grouping is
pure AST-shape hashing; it has no notion of ownership, generics, or trait
bounds, so it can't tell you whether the fix you have in mind — composing a
shared sub-struct vs. extracting a default trait method vs. a free function
— will actually compile. A duplicated method whose parameters include a
closure or trait-object bound by ``Self``, for instance, only works as a
default trait method, not as a delegated sub-struct field, and nothing in
this tool's output signals that distinction. Treat every group as "these
bodies are shaped the same" and nothing more; the extraction design itself
is a human (or a separate, Rust-generics-aware analysis) call.

Substrate — Ecosystem Research (June 2026)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Before building ``parse-db``, a survey was done of existing Rust crates that
might serve as the substrate or as components within it.  **The conclusion is:
build it.** No existing crate covers cross-file analysis for 16+ languages.
The table below records what was evaluated and why each was ruled in or out.

.. list-table::
   :header-rows: 1
   :widths: 22 12 12 12 12 30

   * - Crate
     - Cross-file
     - Lang-neutral
     - Maintained
     - Library API
     - Verdict
   * - ``stack-graphs``
     - Yes (name resolution only)
     - Yes
     - **No** — archived Sep 2025
     - Yes
     - Dead; do not use
   * - ``ast-grep-core``
     - No
     - Yes
     - Yes
     - Unstable
     - Wrong domain; unstable API
   * - ``salsa``
     - N/A (infrastructure)
     - Agnostic
     - Yes
     - Yes
     - Use for incrementality layer if needed
   * - ``tree-sitter-graph``
     - No (per-file DSL)
     - Yes
     - Yes
     - Yes
     - Low-level building block only
   * - ``tree-sitter-tags``
     - No
     - Yes
     - Yes
     - Yes
     - Useful for definition extraction
   * - ``tree-sitter-language-pack``
     - No
     - Yes (306 langs)
     - Yes
     - Yes
     - Grammar consolidation candidate
   * - ``rust-code-analysis``
     - No
     - Partial
     - **No** — dead since 2021
     - Yes
     - Skip
   * - ``code-graph-cli``
     - Yes
     - Partial (5 langs)
     - Yes
     - **No** — CLI only
     - Right concept; wrong interface

Notable findings:

- **``stack-graphs``** (GitHub's tree-sitter name-resolution framework, backed
  by SQLite) was the most likely "buy" candidate.  It was archived September 9,
  2025.  Dead on arrival.
- **``salsa``** (the incremental computation framework under rust-analyzer) is
  the right incrementality infrastructure, but adds significant design overhead.
  For batch-only analysis a plain ``Arc<DashMap<PathBuf, ParsedFile>>``
  populated with rayon achieves the same result without salsa's query-graph
  complexity.  Salsa earns its keep only when watch-mode or IDE-mode
  incremental re-analysis is needed — that is likely sqc's eventual
  requirement, not knots v1.
- **``tree-sitter-language-pack``** (v1.12, released June 29, 2026) bundles
  306 grammars with pre-built tags/highlight/locals queries and could replace
  all 16 individual ``tree-sitter-*`` entries in ``Cargo.toml``.  Grammar
  version alignment with existing knots dependencies must be verified before
  adopting it.
- **``tree-sitter-tags``** provides "all definitions in this file" nearly for
  free for any language that ships a ``.scm`` tags query.  Could supplement or
  simplify ``visit_functions`` for covered languages.
- **Import and call graph extraction must be built regardless.**  No crate
  covers 16 languages for this.  The implementation pattern is the same as
  ``complexity.rs`` — per-node-kind traversal rules, one ``match`` arm per
  language.

---------------------------------------------------------------------------

Known Issues and Coupling Hotspots
-----------------------------------

The items below are not bugs — knots is correct.  They are structural debts
that will become friction as the language count grows.

1. **main.rs monolith** *(partially resolved)*
   Output formatters were extracted into ``src/output.rs`` (task 43/44) and
   ``FunctionMetrics`` / ``collect_function_metrics`` moved to ``lib.rs``
   (task 44), reducing ``main.rs`` from ~4 100 to ~2 650 lines.  A further
   split of function-discovery helpers (``visit_functions``,
   ``get_function_name``, ``collect_local_names_recursive``) into
   ``src/discovery.rs`` would further reduce coupling.

---------------------------------------------------------------------------

Resolved Issues
---------------

These were identified during the architecture review and have since been
addressed.

- **FunctionMetrics not exported from lib** — moved to ``lib.rs`` along with
  ``collect_function_metrics`` (task 44/commit ``2997cd5``).
- **Double metric calculation in single-file mode** — ``run_single_file_mode``
  now calls ``collect_function_metrics`` once and passes the slice to both
  display and threshold checking (task 45).
- **Ada/Lua SLOC duplication** — ``calculate_sloc_ada`` and
  ``calculate_sloc_lua`` now share a private ``calculate_sloc_line_comment``
  helper (task 46).
- **``collect_local_names_recursive`` invariant untested** — 9 behavioural
  tests across 6 languages guard that locally-defined functions are not
  misclassified as external calls (commit ``55f69be``).
- **Hand-rolled glob matching** — ``glob_match`` replaced with
  ``globset::Glob`` (already a transitive dependency via the ``ignore`` crate),
  which handles ``?``, character classes, and path anchoring correctly
  (task 48).
- **``calculate_signature_complexity`` C-specific** — replaced the broken
  child-search heuristic with a call to the language-neutral
  ``count_explicit_params``, making the signature component of TestScoring
  correct for all 16 supported languages (task 49).
- **No parallel file processing** — ``collect_all_metrics`` now uses a
  rayon ``ThreadPoolBuilder`` when ``--jobs N`` (or ``-j N``) is given,
  ``N > 1``, and more than one file is being analysed.  ``0`` (the default)
  auto-detects available parallelism.  Each worker creates its own
  tree-sitter parser so there is no shared mutable state.  ``run_multi_file_mode``
  and ``run_matrix_mode`` were folded into the same ``collect_all_metrics``
  call, eliminating two duplicate file-processing loops.
- **Language-specific booleans scattered across main.rs** — ``is_python``,
  ``is_ada``, ``is_fortran``, ``is_lua`` replaced by a ``SlocMode`` enum
  carried on ``LanguageInfo``.  ``sloc_mode_for_file()`` derives the mode in
  one place from the ``LANGUAGES`` table; ``nested_fn_sloc`` and
  ``accumulate_nested_sloc`` now take ``SlocMode`` instead of four booleans.
  Adding a language with a non-default comment style requires only a new enum
  variant and a one-field change to its ``LANGUAGES`` entry.

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

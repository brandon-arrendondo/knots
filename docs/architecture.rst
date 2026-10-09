Architecture
============

Knots is a Rust CLI and library built on ``lang-parsing-substrate``. The
workspace also contains ``knots-test-complexity``, a separate C test-quality
heuristic that reuses the metric library.

.. contents:: Contents
   :local:
   :depth: 2

Module ownership
----------------

* ``src/lib.rs`` owns ``FunctionMetrics``, function discovery and naming,
  metric aggregation, and JSON ``FilterRules``. It re-exports substrate
  registry functions, grammars and suppression helpers for API compatibility.
* ``src/complexity.rs`` calculates metrics from tree-sitter nodes and source
  bytes. It performs no file I/O.
* ``src/configurations.rs`` combines locally live C/C++ preprocessor arms;
  ``src/recursion.rs`` identifies recursion for the Cognitive increment.
* ``src/coupling.rs`` resolves corpus imports and computes Ce/Ca/Instability;
  ``src/rust_modules.rs`` supplies Rust module and ``use`` resolution.
* ``src/main.rs`` handles arguments, file discovery, parsing, parallel
  collection, thresholds, baselines and changed-line scoping.
* ``src/config.rs`` loads ``knots.toml`` and applies consumer-specific settings.
* ``src/output.rs`` handles text, matrix, SARIF, JSON, NDJSON and CSV output.
* ``src/duplicates.rs`` and ``src/duplicate_diff.rs`` handle duplicate
  filtering, snapshots and comparisons using substrate fingerprints.

Language registry and discovery
-------------------------------

The language registry and grammar dependencies live in the substrate, not
in local ``LANGUAGES`` or ``SUPPORTED_EXTENSIONS`` constants. Knots
re-exports ``languages()``, ``language_for_file()``, ``LanguageInfo``,
``SlocMode`` and extension predicates. ``knots --supported-languages`` shows
the registry compiled into the binary.

``is_source_extension`` covers primary extensions; ``is_parseable_extension``
also covers registry entries labelled explicit-only. Knots uses the latter
for recursive discovery as well as explicit files, so ``.h`` and ``.ads``
are scanned by default. C header contents are checked for unambiguous C++
syntax before falling back to the C grammar. Filters can narrow the file set.

Analysis pipeline
-----------------

The CLI discovers paths or reads a compilation database, loads configuration
and gate context, then selects single-file, multi-file text, matrix or
structured output. The metric library discovers functions and calculates
one record for each selected function. Named functions are the default;
``--count-anonymous-closures`` adds synthesized anonymous entries.

Multi-file collection uses rayon with a parser owned by each worker. Results
are sorted by file and function. Unreadable or unparseable files currently
produce warnings and are skipped; this is not complete-scan containment or
a guarantee that status 0 covers every discovered input. See
:doc:`cli-reference` for exit-status details.

In recursive mode, a separate corpus pass builds the import graph and applies
file Ce to AIRD. Duplicate detection adds another opt-in parse pass. These
are consumer-owned aggregations, not a persistent shared parse database.

Thresholds run separately from output. Baselines key scores by
``(file, function)`` so line movement does not invalidate a snapshot;
same-named functions in one file share a key. ``--since`` and ``--changed``
restrict gate checks without narrowing output. Inline suppression bypasses
threshold checks while measured values remain in output and snapshots.

Metric attribution
------------------

The published definitions and recorded departures govern counting; another
tool's result is comparison evidence. See :doc:`metrics-reference` and the
`ADR index
<https://github.com/brandon-arrendondo/knots/blob/main/docs/adr/README.md>`_.

For C/C++, McCabe and reported Cognitive are each the maximum over real
preprocessor configurations. AIRD uses Cognitive with all locally live arms
as written, plus recursion. Function regions, identifier resolution and
unknown cases are interpreted in knots, where their metric meaning belongs.

SLOC uses tree-sitter tokens for the default and Python modes: comment tokens
are omitted, but comment markers inside strings remain code. Ada, Lua and
Fortran use the shared line-comment helper: ``--`` for Ada/Lua and ``!`` for
Fortran.
``function_sloc`` subtracts nested functions' own lines from their enclosing
function. ``SlocMode`` dispatch comes from substrate metadata.

Preprocessor dead-code awareness
--------------------------------

Before parsing C, C++, Swift or C#, ``parse_file`` blanks lines the substrate
proves preprocessor-dead. Newlines and byte offsets are preserved. It returns
both the tree and the effective source; downstream metrics, imports and
fingerprints must use that source with the tree. Other languages pass through.

Blanking also prevents dead ``extern "C"`` braces from distorting tree-sitter
recovery when a header is read as C. Recovery nodes alone are not a reason to
suppress valid metrics. See :doc:`metrics-reference` for supported conditions
and the limitations of local proofs.

Coupling and AI scores
----------------------

Efferent coupling (Ce) counts outbound dependencies; afferent coupling (Ca)
counts inbound dependencies. The recursive corpus graph also reports
Instability, ``Ce / (Ca + Ce)``. Import extraction comes from the substrate;
corpus resolution and Rust module handling stay in knots.

AIRD (AI Reasoning Difficulty) includes function-level state coupling and,
in recursive mode, a capped file Ce multiplier. AICP (AI Context Pressure)
uses external-call breadth, SLOC and documentation. External calls do not
feed AIRD, and Ca does not feed AICP. These are calibrated signals, not
proofs of AI performance; formulas and their evidence are in
:doc:`metrics-reference`.

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

``--tier <function|block>`` selects the granularity. ``function`` (the
default) fingerprints whole function-like subtrees, as above. ``block``
fingerprints loop/conditional/switch-shaped subtrees *inside* functions
instead, via the substrate's ``block_fingerprints`` — for the narrower case
where a caller already has one flagged region (e.g. a aurora-lint CERT-C
violation) and wants to search the corpus for other structurally similar
regions, not run a corpus-wide clone search. A ``block``-tier match can span
two functions that are otherwise unrelated (different statements before and
after an identical loop body), which is exactly the point: it surfaces
sub-function recurrence that ``function``-tier duplicate detection cannot
see. ``Block``-tier members have no function name, so the text report falls
back to the tree-sitter node kind (e.g. ``<for_statement>``) in their place.
The two tiers never cross-match — the substrate's ``duplicate_groups`` groups
by ``(hash, tier)``, not hash alone, so a small flagged loop can't
coincidentally group with some unrelated file's whole one-line function. Like
the function tier, this is still exact-hash matching only: a near-miss that
differs by one added or removed statement (a Type-3 clone) still won't match
at either granularity.

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
clone (0% divergence; still inspect its context before extracting it), ``[N% diff from #1]`` for a Type-2 clone or shape coincidence (renamed
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

That comparison doesn't have to be done by hand. ``--dump-duplicates <FILE>``
(alongside ``--find-duplicates``) writes a JSON snapshot of the current
groups — their stable IDs, member counts, node counts, and member labels —
in addition to the normal text report. ``--diff-duplicates BEFORE AFTER``
takes two such snapshots (no corpus files needed; it exits after printing
the summary) and reports each group as resolved (present before, gone
after), new (absent before, present after), changed (same ID, different
member count — shrank or grew), or unchanged. This turns the "run, extract
N members, refactor, re-run, confirm it shrank or vanished" workflow into
two dumps and one diff instead of two full text reports and a manual grep
for a size marker:

.. code-block:: shell

    knots -r src --find-duplicates --dump-duplicates before.json
    # ... refactor ...
    knots -r src --find-duplicates --dump-duplicates after.json
    knots --diff-duplicates before.json after.json

Interpreting results
^^^^^^^^^^^^^^^^^^^^

The filters and annotations above reduce noise, but every reported group
still needs contextual review before extraction. Check the matched bodies,
the surrounding types and the proposed shared interface.

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

Adding a language
-----------------

Follow the detailed checklist in `CLAUDE.md
<https://github.com/brandon-arrendondo/knots/blob/main/CLAUDE.md>`_:

1. Add and test registry metadata, grammar dispatch and dependencies in
   ``lang_parsing_substrate``; release parsing support there first.
2. Adopt that substrate release in knots and maintain compatibility re-exports.
3. Extend metric node-kind handling and function discovery in
   ``src/complexity.rs`` and ``src/lib.rs``; add definition and discovery probes.
4. Run ``cargo test --workspace`` and ``pre-commit run --all-files``. Compare
   affected counts against the previous release before retaining published
   agreement figures.
5. Review language documentation against ``knots --supported-languages``.

``invoke sync-languages`` still expects the removed local registry constant.
Until the generator is repaired, update tables from the compiled registry;
do not recreate a second registry in knots.

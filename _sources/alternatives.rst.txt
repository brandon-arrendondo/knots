Alternatives Comparison
========================

Several tools measure code complexity for C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, Java, C#, Kotlin, Swift, PHP, Fortran, Scala, and Lua.
This page compares knots against the most commonly used alternatives, with
empirical validation data where available.

Comparison Scope
----------------

Knots' compiled registry currently contains 16 languages; run
``knots --supported-languages`` for the complete extension list. It offers
traditional metrics, AIRD/AICP, threshold gates, baselines and changed-function
scoping. Outputs are text, JSON, NDJSON, CSV and SARIF; see
:doc:`output-formats` for their different selection and field behavior.
The companion C test-quality heuristic is described in :doc:`test-complexity`.

Compare actual counts using the `pinned definition probes
<https://github.com/brandon-arrendondo/knots/blob/main/validation/probes/TOOLS.md>`_,
which name tool versions and report differences from the definitions. A
shared metric name or a feature checklist does not establish conformance.

For each alternative's current language, metric and export support, consult
its primary documentation:

- `lizard <https://github.com/terryyin/lizard>`_
- `rust-code-analysis <https://github.com/mozilla/rust-code-analysis>`_;
  its library and CLI use tree-sitter, and analysis does not require building
  the project being measured.
- `Clippy <https://github.com/rust-lang/rust-clippy>`_; distinguish native
  Cargo diagnostics from formats produced by external converters.

Cognitive Complexity: Algorithm Differences
-------------------------------------------

The algorithms differ despite sharing the name. A historical comparison
scanned 11,365 Rust functions and paired 17 high-complexity functions: knots
and rust-code-analysis had a mean ratio of 1.004 and median 1.000. The corpus
was not pinned, and the comparison preceded knots' recursion increment,
which rust-code-analysis omits. It does not establish current conformance;
see :doc:`metrics-reference` and the `definition probes
<https://github.com/brandon-arrendondo/knots/blob/main/validation/probes/TOOLS.md>`_.

In that sample, Clippy's scores were lower (mean ratio 0.29, range
0.16–0.50). Its lint uses different counting rules for loops, nesting,
closures, guards and returns. These sample ratios do not give a portable
threshold conversion. Calibrate each tool on the project's own functions;
knots' authority is the published definition, not agreement with a tool.

When to Choose Each Tool
-------------------------

**Choose knots when:**

- You need **AI cost signals** (AIRD/AICP) to gate AI-assisted workflows or
  identify functions that are expensive to modify with an LLM
- You want **CI threshold enforcement** across C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, Java, C#, Kotlin, Swift, PHP, Fortran, Scala, and Lua in one pass
- You want **SARIF output** for PR annotations in GitHub Code Scanning
- You want **NDJSON corpus analysis** composable via ``find``/``xargs``
- You want a **pre-commit hook** that works out of the box
- You want **test quality enforcement** via the companion ``knots-test-complexity``

**Choose an alternative when:**

- **lizard**: its supported-language set covers your project, or you already use it and
  don't need AI metrics
- **rust-code-analysis**: you need Halstead metrics or Maintainability Index
  for Rust/Java/Python
- **clippy**: you want deep Rust semantic analysis, idiomatic style
  enforcement, and unsafe lints; note that its ``cognitive_complexity`` lint
  uses a simplified algorithm (see above) with a very different scale

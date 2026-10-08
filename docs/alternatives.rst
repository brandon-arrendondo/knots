Alternatives Comparison
========================

Several tools measure code complexity for C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, Java, C#, Kotlin, Swift, PHP, Fortran, Scala, and Lua.
This page compares knots against the most commonly used alternatives, with
empirical validation data where available.

Feature Comparison
------------------

.. list-table::
   :header-rows: 1
   :widths: 36 12 12 15 12

   * - Feature
     - knots
     - `lizard <https://github.com/terryyin/lizard>`_
     - `rust-code-analysis <https://github.com/mozilla/rust-code-analysis>`_
     - `clippy <https://github.com/rust-lang/rust-clippy>`_
   * - **Languages**
     -
     -
     -
     -
   * - C / C++
     - ✓
     - ✓
     - ✓
     - ✗
   * - Rust
     - ✓
     - ✓
     - ✓
     - ✓
   * - Python
     - ✓
     - ✓
     - ✓
     - ✗
   * - JavaScript
     - ✓
     - ✓
     - ✓
     - ✗
   * - TypeScript
     - ✓
     - ✓
     - ✓
     - ✗
   * - Go
     - ✓
     - ✓
     - ✗
     - ✗
   * - Java
     - ✓
     - ✓
     - ✓
     - ✗
   * - C#
     - ✓
     - ✓
     - ✗
     - ✗
   * - Ada
     - ✓
     - ✗
     - ✗
     - ✗
   * - Swift
     - ✓
     - ✗
     - ✗
     - ✗
   * - Language count
     - 12
     - ~25
     - 11
     - 1 (Rust only)
   * - **Metrics**
     -
     -
     -
     -
   * - McCabe cyclomatic
     - ✓
     - ✓
     - ✓
     - lint only
   * - Cognitive complexity (Campbell spec)
     - ✓
     - ✓
     - ✓
     - ✗ (see note)
   * - Nesting depth
     - ✓
     - ✗
     - ✗
     - ✗
   * - SLOC
     - ✓
     - ✓
     - ✓
     - ✗
   * - ABC complexity
     - ✓
     - ✗
     - ✗
     - ✗
   * - Halstead / MI
     - ✗
     - ✗
     - ✓
     - ✗
   * - Test scoring
     - ✓
     - ✗
     - ✗
     - ✗
   * - AIRD (AI reasoning difficulty)
     - ✓
     - ✗
     - ✗
     - ✗
   * - AICP (AI context pressure)
     - ✓
     - ✗
     - ✗
     - ✗
   * - External call count
     - ✓
     - ✗
     - ✗
     - ✗
   * - **Output**
     -
     -
     -
     -
   * - Human-readable text
     - ✓
     - ✓
     - ✓
     - ✓
   * - JSON
     - ✓
     - ✓
     - ✓
     - ✗
   * - NDJSON (find/xargs composable)
     - ✓
     - ✗
     - ✗
     - ✗
   * - CSV
     - ✓
     - ✓
     - ✗
     - ✗
   * - SARIF (VS Code / GitHub)
     - ✓
     - ✗
     - ✗
     - ✓
   * - Testability matrix
     - ✓
     - ✗
     - ✗
     - ✗
   * - **Integration**
     -
     -
     -
     -
   * - CI threshold flags
     - ✓
     - ✓
     - partial
     - ✓
   * - Pre-commit hook (native)
     - ✓
     - manual
     - ✗
     - manual
   * - No compiler / build required
     - ✓
     - ✓
     - ✗
     - ✗
   * - pmccabe-compatible output
     - ✓
     - ✓
     - ✗
     - ✗
   * - Tree-sitter based
     - ✓
     - ✗
     - ✗
     - ✗

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

- **lizard**: you need 30+ language support, or you're already using it and
  don't need AI metrics
- **rust-code-analysis**: you need Halstead metrics or Maintainability Index
  for Rust/Java/Python
- **clippy**: you want deep Rust semantic analysis, idiomatic style
  enforcement, and unsafe lints; note that its ``cognitive_complexity`` lint
  uses a simplified algorithm (see above) with a very different scale

Alternatives Comparison
========================

Several tools measure code complexity for C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, and Java.
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
   * - 30+ other languages
     - ✗
     - ✓
     - ✓
     - ✗
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

All four tools claim to measure "cognitive complexity," but they are **not
computing the same thing**. This was validated empirically against 285k lines
of Rust in a real-world codebase (11,365 functions from ``tools_sqc``).

**knots and rust-code-analysis produce essentially identical scores** (mean
ratio 1.004, median 1.000 across 17 matched high-complexity functions). Both
implement the `G. Ann Campbell cognitive complexity specification
<https://www.sonarsource.com/resources/cognitive-complexity/>`_: loops,
conditionals, and match expressions each add ``1 + nesting_level``, where
nesting level accumulates through nested control flow including closures.

**Clippy's ``cognitive_complexity`` lint uses a fundamentally different
algorithm** that diverges from the Campbell spec in three major ways (sourced
from ``clippy_lints/src/cognitive_complexity.rs``):

.. list-table::
   :header-rows: 1
   :widths: 30 35 35

   * - Aspect
     - Campbell spec / knots / RCA
     - Clippy
   * - ``for`` / ``while`` / ``loop``
     - +1 + nesting_level each
     - **not counted**
   * - Nesting penalty
     - accumulated per level
     - **no nesting penalty**
   * - Closures
     - increase nesting level for inner code
     - **analyzed as separate functions**
   * - ``match``
     - +1 + nesting_level
     - +1 if >1 arms (flat, no nesting)
   * - Guard clauses
     - not counted separately
     - +1 per arm guard
   * - ``return`` statements
     - not counted
     - +1 (minus adjustment for Result types)

The practical result: **clippy reports 3–4× lower scores** than knots or
rust-code-analysis for the same functions (mean 0.29×, range 0.16×–0.50×).
A function that knots scores at ~75 will typically score ~25 in clippy —
right at clippy's default threshold of 25.

**Threshold equivalence**: clippy threshold 25 ≈ knots/RCA threshold 75–100.

Neither implementation is wrong in an absolute sense; they measure different
things under the same name. Knots and rust-code-analysis follow the published
spec; clippy's lint is a simplified heuristic tuned to fire at
Rust-idiomatic complexity boundaries.

When to Choose Each Tool
-------------------------

**Choose knots when:**

- You need **AI cost signals** (AIRD/AICP) to gate AI-assisted workflows or
  identify functions that are expensive to modify with an LLM
- You want **CI threshold enforcement** across C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, and Java in one pass
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

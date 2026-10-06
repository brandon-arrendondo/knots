# Knots

A fast multi-language code complexity analyzer built on tree-sitter. Knots measures
traditional complexity metrics alongside two AI-specific cost scores — AIRD (AI
Reasoning Difficulty) and AICP (AI Context Pressure) — to identify which functions
are genuinely expensive to modify with AI assistance.

## Features

- **Multiple Complexity Metrics**: McCabe, Cognitive, Nesting Depth, SLOC, ABC, Test Scoring
- **AI Cost Metrics**: AIRD (reasoning difficulty) and AICP (context pressure) — ceilings calibrated on six open-source C codebases; predictive validation is a pilot so far (see [Metrics Reference](docs/metrics-reference.rst))
- **Multi-Language**: C, C++, Rust, Python, JavaScript, TypeScript, Ada, Go, Java, C#, Kotlin, Swift, PHP, Fortran, Scala, and Lua — same metrics and thresholds across all supported languages
- **Testability Matrix**: Categorize functions by complexity and testability
- **Multiple Output Formats**: text, SARIF, JSON, NDJSON (find/xargs-composable), CSV
- **CI Threshold Enforcement**: exit 1 on any threshold violation; recommended `--aird-threshold 85`
- **Pre-commit Hook**: native integration, no shim scripts required
- **Validated**: McCabe agrees with pmccabe's traditional (per-`case`) count on 98.00% of 30,461 C functions across six pinned corpora (nearly all differences are in functions with `#if` arms); Cognitive follows the SonarSource specification, including the recursion increment other implementations omit. See [Metrics Reference](docs/metrics-reference.rst)

## Installation

Prebuilt binary from PyPI (no Rust toolchain, installs in seconds):

```bash
pipx install knots          # or: uv tool install knots
```

From crates.io:

```bash
cargo install knots
```

Or from source:

```bash
git clone https://github.com/brandon-arrendondo/knots.git
cd knots
cargo build --release
```

No C compiler or build system required.

## Quick Start

```bash
# Single file
knots src/main.c

# Recursive directory
knots -r src/

# CI gate — fail if any function has AIRD > 85
knots -r src/ --aird-threshold 85

# Adopt the gate on a legacy codebase: snapshot today, then fail only on regressions
knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json --write-baseline
knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json

# Gate only the functions you actually touched (no new debt in this change)
knots -r src/ --aird-threshold 85 --changed

# SARIF for GitHub Code Scanning
knots -r --format sarif src/ > knots.sarif

# Corpus analysis — one JSON record per function
find . -name "*.c" -o -name "*.rs" | xargs knots --format ndjson > metrics.ndjson

# Testability matrix
knots -m src/main.c
```

## Complexity Indicators

Based on `max(McCabe, cognitive)`:

| Range  | Indicator  | Meaning                               |
|--------|------------|---------------------------------------|
| 1–10   | 😊 Good    | Low complexity, easy to maintain      |
| 11–20  | 😐 Okay    | Moderate complexity, monitor          |
| 21–49  | 😠 Bad     | High complexity, consider refactoring |
| 50+    | 😢 Critical| Urgent refactoring needed             |

## Command-Line Options

```
knots [OPTIONS] [FILE]...
knots [OPTIONS] --compile-commands <FILE>

Options:
  -r, --recursive                   Recursively process all supported source files in directories
  -v, --verbose                     Show detailed per-function analysis
  -m, --matrix                      Show testability matrix categorization
  --compile-commands <FILE>         Use compile_commands.json to get file list
  --include <FILE>                  Include filter rules from JSON file (whitelist)
  --exclude <FILE>                  Exclude filter rules from JSON file (blacklist)
  --exclude-path <PATTERN>          Exclude files whose path matches this regex (repeatable)
  --format <FORMAT>                 text (default) | sarif | json | ndjson | csv
  --mccabe-threshold <N>            Exit 1 if any function exceeds this McCabe complexity
  --cognitive-threshold <N>         Exit 1 if any function exceeds this cognitive complexity
  --nesting-threshold <N>           Exit 1 if any function exceeds this nesting depth
  --sloc-threshold <N>              Exit 1 if any function exceeds this SLOC count
  --abc-threshold <F>               Exit 1 if any function exceeds this ABC magnitude
  --return-threshold <N>            Exit 1 if any function exceeds this return count
  --aird-threshold <N>              Exit 1 if any function exceeds this AIRD (AI Reasoning Difficulty) score (recommended: 85)
  --aicp-threshold <N>              Exit 1 if any function exceeds this AICP (AI Context Pressure) score
  --external-calls-threshold <N>    Exit 1 if any function exceeds this external call count
  --unreachable-blocks-threshold <N> Exit 1 if any function has more than this many unreachable (dead-code) basic blocks (C/C++/Rust only)
  --report <FILE>                   Write a detailed per-function report to this file (opt-in)
  --baseline <FILE>                 Ratchet mode: gate only on regressions vs. this snapshot (see docs/baseline.rst)
  --write-baseline                  Snapshot current scores to --baseline and exit without gating
  --since <REF>                     Gate only functions overlapping lines changed since this git ref
  --changed                         Gate only functions changed in the working tree (sugar for --since HEAD)
  --explain <METRIC>                Explain a metric (e.g. aird, aicp) and how to lower it, then exit
  --find-duplicates                 Report structurally duplicated functions across the corpus (--recursive only)
  --include-fixture-pairs           Keep tests/pass vs tests/fail fixture pairs in --find-duplicates output (excluded by default)
  --include-trivial-duplicates      Keep small-body, low-repeat groups (getters, one-assert tests) in --find-duplicates output (excluded by default)
  --dump-duplicates <FILE>          Write a JSON snapshot of --find-duplicates results, for later comparison via --diff-duplicates
  --diff-duplicates <BEFORE> <AFTER>  Compare two --dump-duplicates snapshots and summarize resolved/new/shrank/grew groups; exits without needing corpus files
  -j, --jobs <N>                    Parallel analysis threads (0 = auto-detect, 1 = sequential, default: 0)
  -h, --help                        Print help
  -V, --version                     Print version
```

## Documentation

Full documentation is in the `docs/` directory (Sphinx/RST):

- [Installation](docs/installation.rst)
- [Quick Start & Usage Examples](docs/quick-start.rst)
- [CLI Reference](docs/cli-reference.rst)
- [Metrics Reference](docs/metrics-reference.rst) — AIRD/AICP formulas, corpus validation
- [Output Formats](docs/output-formats.rst) — JSON schema, SARIF, NDJSON corpus patterns
- [CI Integration](docs/ci-integration.rst) — GitHub Actions, pre-commit hook
- [Baseline / Ratchet Mode](docs/baseline.rst) — adopt the gate on legacy code, fail only on regressions
- [Filter Rules](docs/filters.rst) — `--include`/`--exclude` whitelists/blacklists
- [knots.toml & Inline Suppression](docs/config.rst) — TOML thresholds/exclusion, `tools:off`/`tools:suppress` comments
- [Test Quality Analysis](docs/test-complexity.rst) — knots-test-complexity companion tool
- [Alternatives Comparison](docs/alternatives.rst) — vs. lizard, rust-code-analysis, clippy; cognitive algorithm differences
- [Troubleshooting](docs/troubleshooting.rst)

## Acknowledgements

knots' numbers are checked against independent tools, and several of its
fixes were found by disagreeing with one of them. The comparison tables are in
[validation/probes/TOOLS.md](validation/probes/TOOLS.md) and the corpus
results in [validation/README.md](validation/README.md).

- **[pmccabe](https://packages.debian.org/sid/pmccabe)** (Paul Bame,
  Hewlett-Packard): the reference for McCabe complexity on C. Its separate
  "traditional" and "modified" columns let knots' `switch` counting, and much
  else, be checked function by function across six C corpora.
- **[SLOCCount](https://dwheeler.com/sloccount/)** (David A. Wheeler): the
  reference for physical SLOC, built on the same definition knots follows.
- **[cloc](https://github.com/AlDanial/cloc)** (Al Danial): the most widely
  used line counter, and the second SLOC reference.
- **SLOC, also:** [tokei](https://github.com/XAMPPRocky/tokei) and
  [scc](https://github.com/boyter/scc).
- **McCabe and Cognitive Complexity, other languages:**
  - [lizard](https://github.com/terryyin/lizard) (Terry Yin);
  - [rust-code-analysis](https://github.com/mozilla/rust-code-analysis) (Mozilla);
  - clang-tidy's `readability-function-cognitive-complexity` (LLVM);
  - AdaCore's gnatmetric;
  - Clippy's `cognitive_complexity` lint (the Rust project);
  - [ESLint](https://eslint.org/)'s `complexity` rule;
  - [eslint-plugin-sonarjs](https://github.com/SonarSource/SonarJS) (SonarSource).

The metrics themselves follow their published definitions:
- T. J. McCabe, "A Complexity Measure" (1976), and G. J. Myers' extension to
  compound conditions (1977);
- G. Ann Campbell, *Cognitive Complexity* (SonarSource, version 1.7, 2023);
- R. E. Park, *Software Size Measurement* (CMU/SEI-92-TR-20, 1992);
- J. Fitzpatrick's ABC metric (1997).

knots parses every language with [tree-sitter](https://tree-sitter.github.io/)
and its community grammars. The validation corpora are open-source projects,
pinned in [validation/README.md](validation/README.md).

## Upstream contributions

Fixes in other metric tools that came out of reading their sources against the
published definitions, listed once the maintainers have merged them:

- **[multimetric](https://github.com/priv-kweihmann/multimetric)**: the
  `--bugpredict old` Halstead estimate multiplied effort by 2/3 instead of
  raising it to the 2/3 power (Halstead's E^(2/3)/3000). Reported in
  [issue #213](https://github.com/priv-kweihmann/multimetric/issues/213) and
  fixed by the maintainer in
  [PR #214](https://github.com/priv-kweihmann/multimetric/pull/214).

## License

MIT

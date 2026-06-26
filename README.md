# Knots

A fast multi-language code complexity analyzer built on tree-sitter. Knots measures
traditional complexity metrics alongside two AI-specific cost scores — AIRD (AI
Reasoning Difficulty) and AICP (AI Context Pressure) — to identify which functions
are genuinely expensive to modify with AI assistance.

## Features

- **Multiple Complexity Metrics**: McCabe, Cognitive, Nesting Depth, SLOC, ABC, Test Scoring
- **AI Cost Metrics**: AIRD (reasoning difficulty) and AICP (context pressure) — corpus-validated against 32,205 functions across 6 open-source C codebases
- **Multi-Language**: C, C++, Rust, Python, and JavaScript — same metrics and thresholds across all supported languages
- **Testability Matrix**: Categorize functions by complexity and testability
- **Multiple Output Formats**: text, SARIF, JSON, NDJSON (find/xargs-composable), CSV
- **CI Threshold Enforcement**: exit 1 on any threshold violation; recommended `--aird-threshold 85`
- **Pre-commit Hook**: native integration, no shim scripts required
- **Validated**: McCabe matches pmccabe exactly; Cognitive matches Mozilla rust-code-analysis at 1.004× mean ratio (11,365 Rust functions)

## Installation

```bash
cargo install knots
```

Or from source:

```bash
git clone https://github.com/brandon-arrendondo/knots.git
cd knots
cargo build --release
```

Requires Rust 1.70+. No C compiler or build system required.

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
  -r, --recursive                   Recursively process all C/C++/Rust/Python/JavaScript source files
  -v, --verbose                     Show detailed per-function analysis
  -m, --matrix                      Show testability matrix categorization
  --compile-commands <FILE>         Use compile_commands.json to get file list
  --include <FILE>                  Include filter rules from JSON file (whitelist)
  --exclude <FILE>                  Exclude filter rules from JSON file (blacklist)
  --format <FORMAT>                 text (default) | sarif | json | ndjson | csv
  --mccabe-threshold <N>            Exit 1 if any function exceeds this McCabe complexity
  --cognitive-threshold <N>         Exit 1 if any function exceeds this cognitive complexity
  --nesting-threshold <N>           Exit 1 if any function exceeds this nesting depth
  --sloc-threshold <N>              Exit 1 if any function exceeds this SLOC count
  --abc-threshold <F>               Exit 1 if any function exceeds this ABC magnitude
  --return-threshold <N>            Exit 1 if any function exceeds this return count
  --aird-threshold <N>              Exit 1 if any function exceeds this AIRD score (recommended: 85)
  --aicp-threshold <N>              Exit 1 if any function exceeds this AICP score
  --external-calls-threshold <N>    Exit 1 if any function exceeds this external call count
  --baseline <FILE>                 Ratchet mode: gate only on regressions vs. this snapshot (see docs/baseline.rst)
  --write-baseline                  Snapshot current scores to --baseline and exit without gating
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
- [Test Quality Analysis](docs/test-complexity.rst) — knots-test-complexity companion tool
- [Alternatives Comparison](docs/alternatives.rst) — vs. lizard, rust-code-analysis, clippy; cognitive algorithm differences
- [Troubleshooting](docs/troubleshooting.rst)

## License

MIT

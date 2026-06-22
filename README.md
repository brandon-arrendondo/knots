# Knots

A fast C/C++/Rust code complexity analyzer built on tree-sitter. Knots measures
traditional complexity metrics alongside two AI-specific cost scores — AIRD (AI Reasoning
Difficulty) and AICP (AI Context Pressure) — to help you identify which functions are
genuinely expensive to modify with AI assistance.

## Features

- 🎯 **Multiple Complexity Metrics**: McCabe, Cognitive, Nesting Depth, SLOC, ABC, Test Scoring
- 🤖 **AI Cost Metrics**: AIRD (reasoning difficulty) and AICP (context pressure) — corpus-validated against 32,205 functions across 6 open-source C codebases
- 🦀 **Multi-Language**: C, C++, and Rust — same metrics and thresholds across all three
- 📊 **Testability Matrix**: Categorize functions by complexity and testability
- 🔄 **Recursive Directory Scanning**: Analyze entire codebases at once
- 🎨 **Visual Indicators**: Easy-to-understand emoji-based complexity ratings
- 🔍 **Flexible Filtering**: Include/exclude files and functions with JSON-based rules
- ⚡ **Fast & Accurate**: Built on tree-sitter for reliable AST-based analysis
- 📤 **Multiple Output Formats**: text, SARIF, JSON, NDJSON (find/xargs-composable), CSV
- 📝 **Detailed Reports**: Generate comprehensive reports with `report.txt`
- ✅ **Validated**: McCabe complexity matches pmccabe output exactly (100% accuracy)

## Installation

### From crates.io

```bash
cargo install knots
```

### From Source

```bash
git clone https://github.com/brandon-arrendondo/knots.git
cd knots
cargo build --release
./target/release/knots --version
```

### Requirements

- Rust 1.70 or higher

## Quick Start

```bash
# Analyze a single file
knots path/to/file.c

# Recursively analyze a directory
knots -r path/to/project/

# Analyze files from compile_commands.json (CMake, Bear, etc.)
knots --compile-commands compile_commands.json

# Show detailed per-function breakdown
knots -v path/to/file.c

# Display testability matrix
knots -m path/to/file.c

# CI gate: fail if any function has AIRD > 85 (high AI reasoning difficulty)
knots -r src/ --aird-threshold 85

# Corpus analysis: one JSON record per function, composable via find/xargs
find . -name "*.c" | xargs knots --format ndjson > metrics.ndjson

# Filter analysis
knots -r . --include filter.json --exclude exclude.json
```

## Complexity Indicators

Knots uses visual emoji indicators based on the maximum of McCabe and Cognitive complexity:

- 😊 **1-10**: Good - Low complexity, easy to maintain
- 😐 **11-20**: Okay - Moderate complexity, monitor carefully
- 😠 **21-49**: Bad - High complexity, should be refactored
- 😢 **50+**: Critical - Very high complexity, urgent refactoring needed

## Command-Line Options

```
knots [OPTIONS] [FILE]...

Arguments:
  [FILE]...  Path(s) to C/C++/Rust files or directories to analyze

Options:
  -r, --recursive                   Recursively process all C/C++/Rust source files in directories
  -v, --verbose                     Show detailed per-function analysis
  -m, --matrix                      Show testability matrix categorization
  --compile-commands <FILE>         Use compile_commands.json to get list of files to analyze
  --include <FILE>                  Include filter rules from JSON file (whitelist)
  --exclude <FILE>                  Exclude filter rules from JSON file (blacklist)
  --format <FORMAT>                 Output format [default: text]
                                      text   — human-readable (default)
                                      sarif  — SARIF 2.1.0 for VS Code / GitHub Code Scanning
                                      json   — JSON array of per-function metrics
                                      ndjson — one record per line; composable via find/xargs
                                      csv    — header + rows
  --mccabe-threshold <N>            Exit 1 if any function exceeds this McCabe complexity
  --cognitive-threshold <N>         Exit 1 if any function exceeds this cognitive complexity
  --nesting-threshold <N>           Exit 1 if any function exceeds this nesting depth
  --sloc-threshold <N>              Exit 1 if any function exceeds this SLOC count
  --abc-threshold <F>               Exit 1 if any function exceeds this ABC magnitude
  --return-threshold <N>            Exit 1 if any function exceeds this return count
  --aird-threshold <N>              Exit 1 if any function exceeds this AIRD score (recommended: 85)
  --aicp-threshold <N>              Exit 1 if any function exceeds this AICP score
  --external-calls-threshold <N>    Exit 1 if any function exceeds this external call count
  -h, --help                        Print help
  -V, --version                     Print version
```

## Usage

### Single File Analysis

```bash
knots src/main.c
```

Output shows per-function metrics:
```
😊 init_system (McCabe: 3, Cognitive: 2, Nesting: 2, SLOC: 15, ABC: 4.12, Returns: 1, TestScore: 5, AIRD: 4, AICP: 8, ExtCalls: 2)
😠 process_data (McCabe: 28, Cognitive: 45, Nesting: 8, SLOC: 120, ABC: 35.71, Returns: 7, TestScore: 18, AIRD: 87, AICP: 72, ExtCalls: 14)

Summary:
  Total Functions: 2
  Average McCabe Complexity: 15.50
  Average AIRD Score: 45.50
  Average AICP Score: 40.00
  ...
```

### Verbose Mode

```bash
knots -v src/main.c
```

Shows detailed breakdown including all test scoring components:
```
Function: process_data 😠
  McCabe Complexity: 28
  Cognitive Complexity: 45
  Nesting Depth: 8
  SLOC: 120
  ABC Magnitude: 35.71
  Return Count: 7
  Test Scoring: 18 (Simple)
    - Signature: 3
    - Dependency: 5
    - Observable: 2
    - Implementation: 8
    - Documentation: 0
  AIRD Score: 87
  AICP Score: 72
  External Calls: 14
  Max Complexity: 45
```

### Recursive Directory Analysis

```bash
# Analyze all C files in a directory tree
knots -r ~/projects/myproject/
```

**Recursive mode automatically:**
- Scans all `.c`/`.cpp`/`.cc`/`.cxx`/`.rs` files recursively (skips headers by default)
- Handles UTF-8 encoding errors gracefully (skips and warns)
- Shows top 5 worst functions by complexity
- Displays totals and averages across all files
- Writes detailed per-function report to `report.txt`
- Reports file processing statistics

**Note:** Recursive mode scans source files (`.c`, `.cpp`, `.cc`, `.cxx`, `.rs`) by default. C/C++ headers are excluded because they often contain inline functions, vendor code, and simple utilities. You can still analyze a specific header file directly (e.g., `knots myheader.h`) or use filters to include headers.

**Example output:**
```
=== TOP 5 WORST FUNCTIONS ===

1. 😢 HAL_RCC_OscConfig [drivers/hal_rcc.c]
   McCabe: 71, Cognitive: 214, Nesting: 11, SLOC: 327, ABC: 134.90, Returns: 23, TestScore: 9
2. 😢 process_matrix [src/complex.c]
   McCabe: 43, Cognitive: 128, Nesting: 15, SLOC: 294, ABC: 118.35, Returns: 0, TestScore: 7

=== TOTALS & AVERAGES ===

  Total Functions: 3404
  Average McCabe Complexity: 2.02
  Average Cognitive Complexity: 1.65
  ...

Detailed per-function output written to report.txt

=== FILES PROCESSED ===

  Total files found: 165
  Successfully processed: 163
  Skipped (encoding/parse errors): 2
```

### Compile Commands Integration

Knots can analyze files specified in a `compile_commands.json` file, which is commonly generated by build systems like CMake, Bear, or Clang:

```bash
# Analyze all files in compile_commands.json
knots --compile-commands compile_commands.json

# With verbose output
knots --compile-commands compile_commands.json -v

# Show testability matrix for build files
knots --compile-commands compile_commands.json -m

# Apply filters to compile commands
knots --compile-commands compile_commands.json --include filter.json
```

**Compile commands mode automatically:**
- Reads file paths from the compilation database
- Only analyzes supported source files (`.c`, `.cpp`, `.cc`, `.cxx`, `.rs`; skips headers and other file types)
- Resolves relative paths using the `directory` field from each entry
- Respects include/exclude filters if specified
- Works with any standard `compile_commands.json` format

**Generating compile_commands.json:**

```bash
# CMake projects
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON /path/to/source

# Makefile projects with Bear
bear -- make

# Manual creation for simple projects
echo '[{"directory": "/path/to/project", "file": "src/main.c", "command": "gcc -c src/main.c"}]' > compile_commands.json
```

**Example compile_commands.json:**
```json
[
  {
    "directory": "/home/user/myproject",
    "command": "gcc -c -I./include src/main.c -o build/main.o",
    "file": "src/main.c"
  },
  {
    "directory": "/home/user/myproject", 
    "command": "gcc -c -I./include src/utils.c -o build/utils.o",
    "file": "src/utils.c"
  }
]
```

### Testability Matrix

The testability matrix categorizes functions into four quadrants to help prioritize testing and refactoring:

```bash
# Single file
knots -m src/module.c

# Entire project
knots -r -m ~/projects/myproject/
```

**Matrix Categories:**

- **📊 QUICK WINS**: Low complexity, easy to test → Automate testing
- **🎯 INVEST IN TESTS**: High complexity, easy to test → Priority for unit tests
- **📝 ADD DOCS**: Low complexity, hard to test → Needs better documentation
- **🚨 REFACTOR**: High complexity, hard to test → HIGH RISK, needs refactoring

Example output:
```
=== TESTABILITY MATRIX ===

📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!
=========================================================
  ✓ init_module [src/module.c] (McCabe: 2, TestScore: 3)
  ✓ cleanup [src/module.c] (McCabe: 1, TestScore: 2)

🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!
========================================================
  ⛔ process_matrix [src/complex.c] (McCabe: 35, TestScore: 45)
  ⛔ legacy_handler [src/old.c] (McCabe: 28, TestScore: 38)

=== SUMMARY ===

  Quick Wins:    15 functions
  Invest Tests:  8 functions
  Add Docs:      12 functions
  Refactor:      5 functions
  Total:         40 functions

=== FILES PROCESSED ===

  Total files found: 25
  Successfully processed: 25
```

### Filtering with Include/Exclude

Use JSON-based filters to focus on specific files or functions:

```bash
# Only analyze high-complexity functions
knots -r . --include filter-high-complexity.json

# Exclude vendor code
knots -r . --exclude filter-exclude-vendor.json

# Combine both
knots -r . --include include.json --exclude exclude.json
```

**Filter JSON Schema:**

```json
{
  "file_patterns": [
    "src/**/*.c",
    "lib/**/*.c",
    "!**/vendor/**",
    "!**/test_*.c"
  ],
  "function_patterns": [
    "^process_.*",
    "^handle_.*"
  ],
  "min_complexity": 10,
  "max_complexity": 50
}
```

All fields are optional. See [FILTERS.md](FILTERS.md) for comprehensive documentation.

### Structured Output Formats

All structured formats suppress text output — only the data goes to stdout.

#### JSON

```bash
knots --format json src/main.c > metrics.json
knots -r --format json src/ > metrics.json
```

Emits a pretty-printed JSON array of per-function records. Each record contains all 15
fields: `file`, `function`, `start_line`, `end_line`, `mccabe`, `cognitive`, `nesting`,
`sloc`, `abc_magnitude`, `return_count`, `test_score`, `doc_score`, `aird`, `aicp`,
`external_calls`.

#### NDJSON (newline-delimited JSON)

```bash
# Composable across files — no array merging needed
find . -name "*.c" | xargs knots --format ndjson > all_metrics.ndjson

# Pipe directly to jq
find src/ -name "*.c" | xargs knots --format ndjson | jq 'select(.aird > 70)'

# Per-file analysis in parallel
find . -name "*.c" | xargs -P4 -I{} sh -c 'knots --format ndjson {} >> metrics.ndjson'
```

One JSON object per line. Unlike `--format json`, output from multiple invocations
concatenates cleanly without array merging.

#### CSV

```bash
knots --format csv src/ > metrics.csv
```

Header + one row per function. Column order matches the JSON field order.

#### SARIF (editor & CI integration)

Knots can emit [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) JSON for VS Code (SARIF Viewer extension), GitHub Code Scanning, and other static-analysis tooling.

```bash
knots --format sarif src/main.c > knots.sarif
knots -r --format sarif src/ > knots.sarif
knots --compile-commands compile_commands.json --format sarif > knots.sarif
```

One SARIF result per function whose max(McCabe, cognitive) exceeds 10. Severity follows
the emoji thresholds:

| Max complexity | SARIF level | Emoji |
|----------------|-------------|-------|
| 1–10           | (omitted)   | 😊    |
| 11–20          | `note`      | 😐    |
| 21–49          | `warning`   | 😠    |
| 50+            | `error`     | 😢    |

Each result carries a `properties` bag with all metrics so downstream tools can filter
on individual values.

**GitHub Code Scanning:** upload with `github/codeql-action/upload-sarif@v3` to surface
findings as PR annotations.

**VS Code:** install the *SARIF Viewer* extension and open `knots.sarif`.

**Example Filters:**

1. **High Complexity Only:**
```json
{
  "min_complexity": 20
}
```

2. **Exclude Tests and Vendor Code:**
```json
{
  "file_patterns": [
    "!**/test_*.c",
    "!**/vendor/**",
    "!**/third_party/**"
  ]
}
```

3. **Focus on Specific Subsystem:**
```json
{
  "file_patterns": ["src/core/**/*.c"],
  "function_patterns": ["^(init|process|handle)_.*"],
  "min_complexity": 5
}
```

## Complexity Metrics

### McCabe Cyclomatic Complexity
Measures the number of linearly independent paths through code. Based on control flow decision points.

- **Formula**: Count decision points + 1
- **Thresholds**: ≤10 good, 11-20 okay, 21+ needs refactoring
- **Validated**: 100% match with pmccabe output

### Cognitive Complexity
Measures how difficult code is to understand, emphasizing nesting and structural complexity.

- Higher weight for nested structures
- Better indicator of maintainability than McCabe
- Based on [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)

### Nesting Depth
Maximum depth of nested control structures (if/for/while/switch).

- Deep nesting makes code hard to follow
- Threshold: >4 levels considered problematic

### SLOC (Source Lines of Code)
Counts non-blank, non-comment lines of code in a function.

- Simple metric but useful in combination
- Large functions (>50 SLOC) often need splitting

### ABC Complexity
Assignment, Branch, and Condition complexity vector.

- **A**: Assignment statements
- **B**: Branch statements (function calls)
- **C**: Condition statements
- **Magnitude**: √(A² + B² + C²)

### Test Scoring
Multi-dimensional metric assessing automated testing difficulty:

- **Signature**: Parameter complexity (0-10)
- **Dependency**: External dependencies (0-10)
- **Observable**: Side effects and observability (0-10)
- **Implementation**: Internal complexity (0-10)
- **Documentation**: Comment quality (-10 to 0, reduces difficulty)

**Score ranges:**
- **≤10**: Trivial to test
- **11-20**: Simple, automated with minimal metadata
- **21-30**: Moderate, needs good documentation
- **31+**: Complex, requires detailed specifications

See [test_scoring.md](test_scoring.md) for complete specification.

### AIRD — AI Reasoning Difficulty

Normalized 0–100 score predicting how much reasoning effort an AI model needs to safely
modify a function. Higher = more AI reasoning required.

```
AIRD = (cognitive/75 × 55) + (sloc/200 × 15) + (nesting/8 × 15) + (test_score/20 × 15) - (doc_score/10 × 15)
```

Ceilings are set at the p99 of the observed distribution across 32,205 functions from 6
open-source C codebases (mosquitto, SQLite, curl, hostap, Lua, libcrc). Cognitive
complexity is the dominant driver; SLOC, nesting, and testability are secondary.

**Recommended CI threshold:** `--aird-threshold 85` — validated empirically against
Sonnet 4.6 and Opus 4.8; functions scoring ≥85 were consistently rated significantly
harder to modify than mid-band or low-band functions.

**Distribution:** heavily right-skewed — 67–88% of functions in mature codebases score
≤10. The ≥76 bucket accounts for 1–2% of functions across all corpora.

### AICP — AI Context Pressure

Normalized 0–100 score predicting how much context an AI model must load before it can
act. Complements AIRD: a function can be cheap to load but hard to reason about, or
expensive to load but trivial once context is assembled.

```
AICP = (external_calls/20 × 60) + (sloc/200 × 40) - (doc_score/10 × 15)
```

External call breadth (unique call targets not defined in the same translation unit) is
the primary driver. The p99 ceiling of 20 external calls is consistent across all 6
corpora.

### External Calls

Count of unique identifier-form call targets in a function that are not defined in the
same translation unit (covers out-of-file functions and function-like macros). Measures
the breadth of external dependency a function pulls in.

- **Threshold flag:** `--external-calls-threshold <N>`
- **p99 across corpus:** 20; p90: 9; p75: 5
- **Mean by AIRD band:** 2.74 (low) → 8.69 (mid) → 17.40 (high)

## Test Quality Analysis (knots-test-complexity)

This workspace also includes `knots-test-complexity`, a companion tool that validates unit tests have sufficient complexity and boundary coverage to thoroughly exercise source code.

### Quick Overview

While traditional code coverage can be misleading (100% branch coverage doesn't guarantee all edge cases are tested), `knots-test-complexity` enforces that tests have adequate cyclomatic complexity relative to the source code they're testing.

```bash
# Basic usage
knots-test-complexity test/test_battery.c src/battery.c

# Custom thresholds
knots-test-complexity \
  --threshold=0.70 \
  --boundary-threshold=0.80 \
  --level=error \
  test/test_timer.c src/timer.c
```

### Key Features

- **Complexity Ratio Analysis**: Ensures test complexity is proportional to source complexity
- **Boundary Value Detection**: Validates tests cover critical boundary conditions (0, MAX, overflow)
- **Pre-commit Integration**: Enforce test quality standards in your workflow

### Pre-commit Hook

The knots-test-complexity hook is designed for **Ceedling** test framework and automatically finds source files by parsing the `TEST_SOURCE_FILE` macro:

```yaml
repos:
  - repo: https://github.com/brandon-arrendondo/knots
    rev: v0.3.0
    hooks:
      # Standard knots complexity check
      - id: knots
        args: [--mccabe-threshold=15, --cognitive-threshold=15]
        exclude: ^(Drivers/|Middlewares/)

      # Test quality validation for Ceedling projects
      - id: test-complexity
        args:
          - --threshold=0.70
          - --boundary-threshold=0.80
          - --level=error
          - --framework=ceedling
          - --test-dir=Test
```

**How it works**: The wrapper parses `TEST_SOURCE_FILE("path/to/source.c")` from your Ceedling test files to automatically locate the corresponding source code. Adjust `--test-dir` if your tests are in a different directory (test/Tests/tests/etc).

For complete documentation, see [knots-test-complexity/README.md](knots-test-complexity/README.md).

## Examples

### Example 1: Quick Health Check

```bash
# Get quick overview of worst functions
knots -r ~/myproject/ | head -20
```

### Example 2: Detailed Audit

```bash
# Generate comprehensive report
knots -r -v ~/myproject/

# Review report.txt for all functions
less report.txt
```

### Example 3: Refactoring Prioritization

```bash
# Find high-complexity, hard-to-test functions
echo '{"min_complexity": 15}' > complex.json
knots -r -m ~/myproject/ --include complex.json
```

### Example 4: CI/CD Integration

```bash
#!/bin/bash
# Fail if any function exceeds complexity threshold

echo '{"min_complexity": 51}' > fail-threshold.json
knots -r . --include fail-threshold.json > /tmp/knots-output.txt

if grep -q "Total Functions: [1-9]" /tmp/knots-output.txt; then
    echo "ERROR: Functions with complexity > 50 detected!"
    cat /tmp/knots-output.txt
    exit 1
fi
```

### Example 5: Focus on New Code

```bash
# Analyze only files modified in last commit
git diff --name-only HEAD~1 | grep -E '\.(c|cpp|cc|cxx)$' | while read file; do
    knots "$file"
done
```

### Example 6: CMake/Build System Integration

```bash
# Generate compile database and analyze
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -B build
knots --compile-commands build/compile_commands.json -m

# For Makefile projects
bear -- make clean all
knots --compile-commands compile_commands.json -v

# Focus on high-complexity functions in build
echo '{"min_complexity": 20}' > high-complexity.json
knots --compile-commands compile_commands.json --include high-complexity.json
```

## Validation

The McCabe complexity implementation has been validated against industry-standard tools:

### Validated Against:
- **[pmccabe](https://people.debian.org/~bame/pmccabe/)** - Industry standard since 1990s
- **[lizard](https://github.com/terryyin/lizard)** - Popular multi-language analyzer

### Results:
- ✓ 13/13 functions match pmccabe exactly (100% accuracy)
- ✓ Correctly implements switch/case complexity
- ✓ Handles nested structures and logical operators accurately

## Alternatives Comparison

Several tools measure code complexity for C/C++ and Rust. Here's how knots compares:

| Feature | knots | [lizard](https://github.com/terryyin/lizard) | [rust-code-analysis](https://github.com/mozilla/rust-code-analysis) | [clippy](https://github.com/rust-lang/rust-clippy) |
|---------|:-----:|:-----:|:-----:|:-----:|
| **Languages** | | | | |
| C / C++ | ✓ | ✓ | ✓ | ✗ |
| Rust | ✓ | ✓ | ✓ | ✓ |
| 30+ other languages | ✗ | ✓ | ✓ | ✗ |
| **Metrics** | | | | |
| McCabe cyclomatic | ✓ | ✓ | ✓ | lint only |
| Cognitive complexity | ✓ | ✓ | ✓ | lint only |
| Nesting depth | ✓ | ✗ | ✗ | ✗ |
| SLOC | ✓ | ✓ | ✓ | ✗ |
| ABC complexity | ✓ | ✗ | ✗ | ✗ |
| Halstead / MI | ✗ | ✗ | ✓ | ✗ |
| Test scoring | ✓ | ✗ | ✗ | ✗ |
| AIRD (AI reasoning difficulty) | ✓ | ✗ | ✗ | ✗ |
| AICP (AI context pressure) | ✓ | ✗ | ✗ | ✗ |
| External call count | ✓ | ✗ | ✗ | ✗ |
| **Output** | | | | |
| Human-readable text | ✓ | ✓ | ✓ | ✓ |
| JSON | ✓ | ✓ | ✓ | ✗ |
| NDJSON (find/xargs composable) | ✓ | ✗ | ✗ | ✗ |
| CSV | ✓ | ✓ | ✗ | ✗ |
| SARIF (VS Code / GitHub) | ✓ | ✗ | ✗ | ✓ |
| Testability matrix | ✓ | ✗ | ✗ | ✗ |
| **Integration** | | | | |
| CI threshold flags | ✓ | ✓ | partial | ✓ |
| Pre-commit hook (native) | ✓ | manual | ✗ | manual |
| No compiler / build required | ✓ | ✓ | ✗ | ✗ |
| pmccabe-compatible output | ✓ | ✓ | ✗ | ✗ |
| Tree-sitter based | ✓ | ✗ | ✗ | ✗ |

### When to choose knots

- You need **AI cost signals** (AIRD/AICP) to gate AI-assisted workflows or identify functions that are expensive to modify with an LLM
- You want **CI threshold enforcement** with `--aird-threshold 85`, `--cognitive-threshold`, etc. across C, C++, and Rust in one pass
- You want **SARIF output** to surface complexity findings as PR annotations in GitHub Code Scanning
- You want **NDJSON corpus analysis** — `find . -name "*.c" | xargs knots --format ndjson` composable without array merging
- You want a **pre-commit hook** that works out of the box without manual shim scripts
- You want **test quality enforcement** alongside complexity via the companion `knots-test-complexity` tool

### When to choose an alternative

- **lizard**: you need 30+ language support, or you're already using it and don't need AI metrics
- **rust-code-analysis**: you need Halstead metrics or Maintainability Index for Rust/Java/Python
- **clippy**: you want deep Rust semantic analysis, idiomatic style enforcement, and unsafe lints rather than raw metric numbers

## Troubleshooting

### "Path is a directory. Use -r/--recursive"

You tried to analyze a directory without `-r`:
```bash
knots -r path/to/directory/
```

### "Warning: Skipping <file>: stream did not contain valid UTF-8"

File has encoding issues. Knots continues processing other files. To fix:
```bash
# Convert to UTF-8
iconv -f ISO-8859-1 -t UTF-8 file.c > file_utf8.c

# Or exclude problematic files
knots -r . --exclude exclude-encoding-issues.json
```

### "No supported source files found in directory"

Check:
- File extensions are `.c`, `.cpp`, `.cc`, `.cxx`, or `.rs` (recursive mode only scans source files, not headers)
- You're in the right directory
- Files aren't filtered out by include/exclude rules

**Note:** To include `.h` files, use a filter:
```json
{
  "file_patterns": ["**/*.c", "**/*.h"]
}
```

## Advanced Usage

### Pre-commit Hook

Knots integrates with the [pre-commit](https://pre-commit.com) framework. Add it to your `.pre-commit-config.yaml` and pre-commit will automatically build and install knots:

```yaml
repos:
  - repo: https://github.com/brandon-arrendondo/knots
    rev: v1.4.3
    hooks:
      - id: knots          # fails on violations (default thresholds)
      # - id: knots-verbose # same thresholds, shows per-function detail
      # - id: knots-strict  # stricter thresholds (McCabe 10, Cognitive 10, ...)
```

Then run:

```bash
pre-commit install
```

Custom thresholds can be set via `args`:

```yaml
      - id: knots
        args:
          - --mccabe-threshold=20
          - --cognitive-threshold=15
```

See `example-pre-commit-config.yaml` in the repo for a full example with all options.

### Combining with Other Tools

```bash
# Generate complexity report and run static analysis
knots -r src/ > complexity.txt
cppcheck src/ 2> cppcheck.txt

# Find high-complexity functions mentioned in warnings
grep -f <(knots -r src/ | grep 😢 | cut -d' ' -f2) cppcheck.txt
```

## Contributing

Contributions are welcome! Please submit issues or pull requests.

### Development

```bash
# Clone and build
git clone https://github.com/yourusername/knots.git
cd knots
cargo build

# Run tests
cargo test

# Run examples
cargo run -- examples/complex.c
cargo run -- -r -m examples/
```

## Dependencies

- `tree-sitter` - Parser generator and incremental parsing
- `tree-sitter-c` - C language grammar
- `tree-sitter-cpp` - C++ language grammar
- `tree-sitter-rust` - Rust language grammar
- `clap` - Command-line argument parsing
- `anyhow` - Error handling
- `serde` / `serde_json` - JSON filter support
- `regex` - Pattern matching for filters
- `walkdir` - Recursive directory traversal

## See Also

- [knots-test-complexity/README.md](knots-test-complexity/README.md) - Test quality analyzer documentation
- [FILTERS.md](FILTERS.md) - Comprehensive filtering documentation
- [test_scoring.md](test_scoring.md) - Test scoring metric specification
- [filter-example-include.json](filter-example-include.json) - Example include filter
- [filter-example-exclude.json](filter-example-exclude.json) - Example exclude filter
- [knots/examples/](knots/examples/) - Sample C files with varying complexity
- [knots-test-complexity/examples/](knots-test-complexity/examples/) - Test quality examples

## License

MIT License. See LICENSE file.

## Acknowledgments

- Built with [tree-sitter](https://tree-sitter.github.io/) for accurate C/C++/Rust parsing
- Implements standard complexity metrics from software engineering research
- Cognitive Complexity based on [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)
- Inspired by pmccabe, lizard, rust-code-analysis, CodeClimate, and SonarQube

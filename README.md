# Knots

A fast, powerful C code complexity analyzer with visual indicators, built on tree-sitter. Knots helps you identify problematic code patterns, prioritize refactoring efforts, and understand testability concerns across your codebase.

## Features

- 🎯 **Multiple Complexity Metrics**: McCabe, Cognitive, Nesting Depth, SLOC, ABC, Test Scoring
- 📊 **Testability Matrix**: Categorize functions by complexity and testability
- 🔄 **Recursive Directory Scanning**: Analyze entire codebases at once
- 🎨 **Visual Indicators**: Easy-to-understand emoji-based complexity ratings
- 🔍 **Flexible Filtering**: Include/exclude files and functions with JSON-based rules
- ⚡ **Fast & Accurate**: Built on tree-sitter for reliable AST-based analysis
- 📝 **Detailed Reports**: Generate comprehensive reports with `report.txt`
- ✅ **Validated**: McCabe complexity matches pmccabe output exactly (100% accuracy)

## Installation

### From Source

```bash
git clone https://github.com/yourusername/knots.git
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

# Combine recursive and matrix modes
knots -r -m path/to/project/

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
knots [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the C file or directory to analyze

Options:
  -r, --recursive               Recursively process all C files in directories
  -v, --verbose                 Show detailed per-function analysis
  -m, --matrix                  Show testability matrix categorization
  --compile-commands <FILE>     Use compile_commands.json to get list of files to analyze
  --include <FILE>              Include filter rules from JSON file (whitelist)
  --exclude <FILE>              Exclude filter rules from JSON file (blacklist)
  -h, --help                    Print help
  -V, --version                 Print version
```

## Usage

### Single File Analysis

```bash
knots src/main.c
```

Output shows per-function metrics:
```
😊 init_system (McCabe: 3, Cognitive: 2, Nesting: 2, SLOC: 15, ABC: 4.12, Returns: 1, TestScore: 5)
😠 process_data (McCabe: 28, Cognitive: 45, Nesting: 8, SLOC: 120, ABC: 35.71, Returns: 7, TestScore: 18)

Summary:
  Total Functions: 2
  Average McCabe Complexity: 15.50
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
  Max Complexity: 45
```

### Recursive Directory Analysis

```bash
# Analyze all C files in a directory tree
knots -r ~/projects/myproject/
```

**Recursive mode automatically:**
- Scans all `.c` files recursively (skips `.h` headers by default)
- Handles UTF-8 encoding errors gracefully (skips and warns)
- Shows top 5 worst functions by complexity
- Displays totals and averages across all files
- Writes detailed per-function report to `report.txt`
- Reports file processing statistics

**Note:** Recursive mode only scans `.c` files by default because header files often contain inline functions, vendor code, and simple utilities. You can still analyze a specific header file directly (e.g., `knots myheader.h`) or use filters to include headers if needed.

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
- Only analyzes `.c` files (skips headers and other file types)
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
git diff --name-only HEAD~1 | grep '\\.c$' | while read file; do
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

### "No .c files found in directory"

Check:
- File extensions are `.c` (recursive mode only scans `.c` files, not `.h`)
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

```bash
#!/bin/bash
# .git/hooks/pre-commit

staged_files=$(git diff --cached --name-only --diff-filter=ACM | grep '\\.c$')

if [ -n "$staged_files" ]; then
    for file in $staged_files; do
        if knots "$file" | grep -q 😢; then
            echo "ERROR: High complexity detected in $file"
            knots "$file" | grep 😢
            exit 1
        fi
    done
fi
```

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
- `clap` - Command-line argument parsing
- `anyhow` - Error handling
- `serde` / `serde_json` - JSON filter support
- `regex` - Pattern matching for filters
- `walkdir` - Recursive directory traversal

## See Also

- [FILTERS.md](FILTERS.md) - Comprehensive filtering documentation
- [test_scoring.md](test_scoring.md) - Test scoring metric specification
- [filter-example-include.json](filter-example-include.json) - Example include filter
- [filter-example-exclude.json](filter-example-exclude.json) - Example exclude filter
- [examples/](examples/) - Sample C files with varying complexity

## License

MIT License. See LICENSE file.

## Acknowledgments

- Built with [tree-sitter](https://tree-sitter.github.io/) for accurate C parsing
- Implements standard complexity metrics from software engineering research
- Cognitive Complexity based on [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)
- Inspired by tools like pmccabe, Lizard, CodeClimate, and SonarQube

# Implementation Guide

This document provides detailed implementation guidance for building `tool_test_complexity`.

## Phase 1: Core Complexity Ratio (MVP)

**Goal**: Implement basic test-to-source complexity ratio checking.

### Step 1.1: Project Setup

```bash
cd /home/tvanfossen/BISSELL/ELEC_SW/tool_test_complexity
cargo init --name test-complexity
```

**Cargo.toml**:
```toml
[package]
name = "test-complexity"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test-complexity"
path = "src/main.rs"

[dependencies]
tree-sitter = "0.22"
tree-sitter-c = "0.21"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
colored = "2.0"  # For terminal colors
```

### Step 1.2: Reuse Complexity Calculator from tools_knots

Copy and adapt the complexity calculation logic:

**src/complexity.rs** (adapted from tools_knots):

```rust
use tree_sitter::{Node, Parser, Tree};
use anyhow::Result;

pub struct ComplexityMetrics {
    pub function_name: String,
    pub cyclomatic_complexity: usize,
    pub line_start: usize,
    pub line_end: usize,
}

pub struct FileComplexity {
    pub file_path: String,
    pub functions: Vec<ComplexityMetrics>,
    pub total_complexity: usize,
}

impl FileComplexity {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            functions: Vec::new(),
            total_complexity: 0,
        }
    }

    pub fn add_function(&mut self, metrics: ComplexityMetrics) {
        self.total_complexity += metrics.cyclomatic_complexity;
        self.functions.push(metrics);
    }
}

pub fn calculate_cyclomatic_complexity(node: &Node, source: &[u8]) -> usize {
    let mut complexity = 1; // Base complexity

    // Traverse the AST and count decision points
    let mut cursor = node.walk();
    let mut visit_children = true;

    loop {
        let node = cursor.node();

        if visit_children && cursor.goto_first_child() {
            continue;
        }

        // Count complexity-increasing constructs
        match node.kind() {
            "if_statement" | "while_statement" | "for_statement" |
            "do_statement" | "switch_statement" | "conditional_expression" |
            "||" | "&&" | "case_statement" => {
                complexity += 1;
            }
            _ => {}
        }

        if cursor.goto_next_sibling() {
            visit_children = true;
        } else if cursor.goto_parent() {
            visit_children = false;
        } else {
            break;
        }
    }

    complexity
}

pub fn analyze_file(file_path: &str) -> Result<FileComplexity> {
    let source_code = std::fs::read(file_path)?;

    let mut parser = Parser::new();
    let language = tree_sitter_c::language();
    parser.set_language(&language)?;

    let tree = parser.parse(&source_code, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;

    let root_node = tree.root_node();
    let mut file_complexity = FileComplexity::new(file_path.to_string());

    // Find all function definitions
    let mut cursor = root_node.walk();
    for node in root_node.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let metrics = extract_function_metrics(&node, &source_code);
            file_complexity.add_function(metrics);
        }
    }

    Ok(file_complexity)
}

fn extract_function_metrics(node: &Node, source: &[u8]) -> ComplexityMetrics {
    let function_name = extract_function_name(node, source);
    let cyclomatic_complexity = calculate_cyclomatic_complexity(node, source);
    let line_start = node.start_position().row + 1;
    let line_end = node.end_position().row + 1;

    ComplexityMetrics {
        function_name,
        cyclomatic_complexity,
        line_start,
        line_end,
    }
}

fn extract_function_name(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            // Find identifier node
            let mut declarator_cursor = child.walk();
            for declarator_child in child.children(&mut declarator_cursor) {
                if declarator_child.kind() == "identifier" {
                    let name_bytes = &source[declarator_child.byte_range()];
                    return String::from_utf8_lossy(name_bytes).to_string();
                }
            }
        }
    }
    "unknown".to_string()
}
```

### Step 1.3: CLI Interface

**src/main.rs**:

```rust
use anyhow::Result;
use clap::Parser;
use colored::*;

mod complexity;
mod analyzer;
mod reporter;

use complexity::analyze_file;
use analyzer::TestQualityAnalyzer;
use reporter::Reporter;

#[derive(Parser)]
#[command(name = "test-complexity")]
#[command(about = "Test quality analyzer for C unit tests", long_about = None)]
struct Args {
    /// Test file path (e.g., Test/test_battery_service.c)
    test_file: String,

    /// Source file path (e.g., Core/Src/modules/battery_service/battery_service.c)
    source_file: String,

    /// Minimum test-to-source complexity ratio (default: 0.70)
    #[arg(short, long, default_value = "0.70")]
    threshold: f64,

    /// Enforcement level: warn or error
    #[arg(short, long, default_value = "warn")]
    level: String,

    /// Enable boundary value checking
    #[arg(short = 'b', long, default_value = "true")]
    check_boundaries: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate inputs
    if args.threshold < 0.0 || args.threshold > 2.0 {
        eprintln!("Error: threshold must be between 0.0 and 2.0");
        std::process::exit(1);
    }

    if args.level != "warn" && args.level != "error" {
        eprintln!("Error: level must be 'warn' or 'error'");
        std::process::exit(1);
    }

    // Analyze files
    let test_complexity = analyze_file(&args.test_file)?;
    let source_complexity = analyze_file(&args.source_file)?;

    // Create analyzer
    let analyzer = TestQualityAnalyzer::new(
        test_complexity,
        source_complexity,
        args.threshold,
    );

    // Generate report
    let reporter = Reporter::new(args.verbose);
    let result = analyzer.analyze();

    reporter.print_report(&result);

    // Exit based on enforcement level and result
    if !result.passed && args.level == "error" {
        std::process::exit(1);
    }

    Ok(())
}
```

### Step 1.4: Analyzer Logic

**src/analyzer.rs**:

```rust
use crate::complexity::FileComplexity;

pub struct TestQualityAnalyzer {
    test_complexity: FileComplexity,
    source_complexity: FileComplexity,
    threshold: f64,
}

pub struct AnalysisResult {
    pub passed: bool,
    pub test_complexity: usize,
    pub source_complexity: usize,
    pub ratio: f64,
    pub threshold: f64,
    pub test_function_count: usize,
    pub source_function_count: usize,
    pub recommendations: Vec<String>,
}

impl TestQualityAnalyzer {
    pub fn new(
        test_complexity: FileComplexity,
        source_complexity: FileComplexity,
        threshold: f64,
    ) -> Self {
        Self {
            test_complexity,
            source_complexity,
            threshold,
        }
    }

    pub fn analyze(&self) -> AnalysisResult {
        let test_total = self.test_complexity.total_complexity;
        let source_total = self.source_complexity.total_complexity;

        let ratio = if source_total > 0 {
            test_total as f64 / source_total as f64
        } else {
            1.0 // No source complexity = trivial, always pass
        };

        let passed = ratio >= self.threshold;

        let mut recommendations = Vec::new();
        if !passed {
            self.generate_recommendations(&mut recommendations, ratio);
        }

        AnalysisResult {
            passed,
            test_complexity: test_total,
            source_complexity: source_total,
            ratio,
            threshold: self.threshold,
            test_function_count: self.test_complexity.functions.len(),
            source_function_count: self.source_complexity.functions.len(),
            recommendations,
        }
    }

    fn generate_recommendations(&self, recommendations: &mut Vec<String>, ratio: f64) {
        let gap_percent = ((self.threshold - ratio) * 100.0) as i32;
        let missing_complexity = (self.source_complexity.total_complexity as f64 * self.threshold) as usize
            - self.test_complexity.total_complexity;

        recommendations.push(format!(
            "Add ~{} more complexity points to tests ({} percentage points below threshold)",
            missing_complexity, gap_percent
        ));

        recommendations.push("Consider adding:".to_string());
        recommendations.push("  - Edge case tests (boundary values, overflow scenarios)".to_string());
        recommendations.push("  - Error path tests (invalid inputs, error conditions)".to_string());
        recommendations.push("  - State transition tests (different initial conditions)".to_string());
        recommendations.push("  - Parametrized tests or loops in test code".to_string());

        // Identify high-complexity source functions that might need more testing
        let mut high_complexity_funcs: Vec<_> = self.source_complexity.functions.iter()
            .filter(|f| f.cyclomatic_complexity > 5)
            .collect();
        high_complexity_funcs.sort_by_key(|f| std::cmp::Reverse(f.cyclomatic_complexity));

        if !high_complexity_funcs.is_empty() {
            recommendations.push(format!("\nComplex functions needing thorough tests:"));
            for func in high_complexity_funcs.iter().take(5) {
                recommendations.push(format!(
                    "  - {}() [complexity: {}]",
                    func.function_name,
                    func.cyclomatic_complexity
                ));
            }
        }
    }
}
```

### Step 1.5: Reporter

**src/reporter.rs**:

```rust
use colored::*;
use crate::analyzer::AnalysisResult;

pub struct Reporter {
    verbose: bool,
}

impl Reporter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn print_report(&self, result: &AnalysisResult) {
        println!("\n{}", "━".repeat(70).bright_black());
        println!("{}", "Test Quality Analysis".bold());
        println!("{}\n", "━".repeat(70).bright_black());

        // Source metrics
        println!("{}", "Source File Analysis:".bold());
        println!("  Functions: {}", result.source_function_count);
        println!("  Total Complexity: {}", result.source_complexity);

        // Test metrics
        println!("\n{}", "Test File Analysis:".bold());
        println!("  Functions: {}", result.test_function_count);
        println!("  Total Complexity: {}", result.test_complexity);

        // Ratio analysis
        println!("\n{}", "Complexity Ratio:".bold());
        let ratio_percent = (result.ratio * 100.0) as i32;
        let threshold_percent = (result.threshold * 100.0) as i32;

        let status = if result.passed {
            format!("{}% ✓", ratio_percent).green()
        } else {
            format!("{}% ✗", ratio_percent).red()
        };

        println!("  Test/Source Ratio: {} (threshold: {}%)", status, threshold_percent);
        println!("  Test Complexity: {}", result.test_complexity);
        println!("  Source Complexity: {}", result.source_complexity);
        println!("  Ratio: {}/{} = {:.2}",
            result.test_complexity,
            result.source_complexity,
            result.ratio
        );

        // Recommendations
        if !result.recommendations.is_empty() {
            println!("\n{}", "Recommendations:".bold().yellow());
            for rec in &result.recommendations {
                println!("  {}", rec);
            }
        }

        // Final result
        println!("\n{}", "━".repeat(70).bright_black());
        if result.passed {
            println!("{}", "Result: ✓ PASS".green().bold());
        } else {
            println!("{}", "Result: ✗ FAIL".red().bold());
        }
        println!("{}\n", "━".repeat(70).bright_black());
    }
}
```

### Step 1.6: Build and Test

```bash
# Build
cargo build --release

# Test with existing project
./target/release/test-complexity \
    /home/tvanfossen/BISSELL/ELEC_SW/d_stcube_ascent_main/Test/test_battery_service.c \
    /home/tvanfossen/BISSELL/ELEC_SW/d_stcube_ascent_main/Core/Src/modules/battery_service/battery_service.c \
    --threshold=0.70 \
    --level=warn \
    --verbose
```

## Phase 2: Boundary Value Detection

**Goal**: Detect boundary values in source and validate test coverage.

### Step 2.1: Boundary Detector

**src/boundary.rs**:

```rust
use tree_sitter::{Node, Parser};
use regex::Regex;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct BoundaryValue {
    pub variable_name: String,
    pub type_name: String,
    pub min_value: i64,
    pub max_value: i64,
    pub line: usize,
}

pub struct BoundaryDetector {
    boundaries: Vec<BoundaryValue>,
}

impl BoundaryDetector {
    pub fn new() -> Self {
        Self {
            boundaries: Vec::new(),
        }
    }

    pub fn detect_boundaries(&mut self, file_path: &str) -> Result<Vec<BoundaryValue>> {
        let source_code = std::fs::read(file_path)?;

        // Detect integer type declarations
        self.detect_integer_types(&source_code)?;

        // Detect range checks
        self.detect_range_checks(&source_code)?;

        Ok(self.boundaries.clone())
    }

    fn detect_integer_types(&mut self, source: &[u8]) -> Result<()> {
        let type_patterns = vec![
            ("uint8_t", 0, 255),
            ("uint16_t", 0, 65535),
            ("uint32_t", 0, 4294967295),
            ("int8_t", -128, 127),
            ("int16_t", -32768, 32767),
            ("int32_t", -2147483648, 2147483647),
        ];

        let source_str = String::from_utf8_lossy(source);

        for (type_name, min_val, max_val) in type_patterns {
            // Regex to find variable declarations
            let pattern = format!(r"\b{}\s+(\w+)\s*[;=]", type_name);
            let re = Regex::new(&pattern)?;

            for captures in re.captures_iter(&source_str) {
                if let Some(var_name) = captures.get(1) {
                    self.boundaries.push(BoundaryValue {
                        variable_name: var_name.as_str().to_string(),
                        type_name: type_name.to_string(),
                        min_value: min_val,
                        max_value: max_val,
                        line: 0, // TODO: Track line number
                    });
                }
            }
        }

        Ok(())
    }

    fn detect_range_checks(&mut self, source: &[u8]) -> Result<()> {
        let source_str = String::from_utf8_lossy(source);

        // Detect patterns like: if (x > MAX), if (x < MIN), etc.
        let range_patterns = vec![
            r"if\s*\(\s*(\w+)\s*>\s*(\w+)\s*\)",  // if (x > MAX)
            r"if\s*\(\s*(\w+)\s*<\s*(\w+)\s*\)",  // if (x < MIN)
            r"if\s*\(\s*(\w+)\s*>=\s*(\w+)\s*\)", // if (x >= threshold)
            r"if\s*\(\s*(\w+)\s*<=\s*(\w+)\s*\)", // if (x <= threshold)
        ];

        for pattern in range_patterns {
            let re = Regex::new(pattern)?;
            for captures in re.captures_iter(&source_str) {
                // Extract variable and constant
                // Add to boundaries list
                // TODO: Implement full extraction
            }
        }

        Ok(())
    }

    pub fn count_boundary_tests(&self, test_file_path: &str) -> Result<usize> {
        let source_code = std::fs::read_to_string(test_file_path)?;
        let mut count = 0;

        // Look for boundary values in test assertions
        for boundary in &self.boundaries {
            // Check for min value
            if source_code.contains(&boundary.min_value.to_string()) {
                count += 1;
            }
            // Check for max value
            if source_code.contains(&boundary.max_value.to_string()) {
                count += 1;
            }
            // Check for overflow (max + 1)
            let overflow_val = boundary.max_value.wrapping_add(1);
            if source_code.contains(&overflow_val.to_string()) {
                count += 1;
            }
        }

        Ok(count)
    }
}
```

### Step 2.2: Integration with Analyzer

Update `src/analyzer.rs` to include boundary checking:

```rust
pub struct TestQualityAnalyzer {
    // ... existing fields
    boundary_detector: Option<BoundaryDetector>,
}

impl TestQualityAnalyzer {
    pub fn with_boundaries(mut self, source_file: &str, test_file: &str) -> Result<Self> {
        let mut detector = BoundaryDetector::new();
        let boundaries = detector.detect_boundaries(source_file)?;
        let boundary_tests = detector.count_boundary_tests(test_file)?;

        // Store for analysis
        self.boundary_detector = Some(detector);
        Ok(self)
    }
}
```

## Phase 3: Pre-Commit Integration

### Step 3.1: Wrapper Script

**hooks/test-complexity-wrapper.sh**:

```bash
#!/bin/bash
#
# Pre-commit wrapper for test-complexity tool
# Finds corresponding source file for each test file and runs analysis
#

THRESHOLD=0.70
LEVEL=warn
CHECK_BOUNDARIES=true
VERBOSE=false
TOOL_PATH=${TOOL_PATH:-test-complexity}

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse arguments
FILES=()
while [[ $# -gt 0 ]]; do
    case $1 in
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --level)
            LEVEL="$2"
            shift 2
            ;;
        --check-boundaries)
            CHECK_BOUNDARIES=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        *)
            FILES+=("$1")
            shift
            ;;
    esac
done

# Check if tool is available
if ! command -v "$TOOL_PATH" &> /dev/null; then
    echo -e "${YELLOW}Warning: test-complexity not found${NC}"
    echo "Install it or set TOOL_PATH environment variable"
    exit 0  # Don't fail if tool not installed
}

# If no files provided, exit
if [ ${#FILES[@]} -eq 0 ]; then
    exit 0
fi

FAILED=0
TOTAL_VIOLATIONS=0

for test_file in "${FILES[@]}"; do
    if [ ! -f "$test_file" ]; then
        continue
    fi

    # Extract source file path from test file name
    # Pattern: Test/test_foo.c -> Core/Src/modules/foo/foo.c
    base_name=$(basename "$test_file")
    source_name=${base_name#test_}  # Remove "test_" prefix

    # Search for corresponding source file
    source_file=$(find Core/Src/modules -name "$source_name" 2>/dev/null | head -1)

    if [ -z "$source_file" ]; then
        echo -e "${YELLOW}⚠ $test_file: Cannot find source file for $source_name${NC}"
        continue
    fi

    # Build command
    CMD="$TOOL_PATH \"$test_file\" \"$source_file\" --threshold=$THRESHOLD --level=$LEVEL"

    if [ "$CHECK_BOUNDARIES" = true ]; then
        CMD="$CMD --check-boundaries"
    fi

    if [ "$VERBOSE" = true ]; then
        CMD="$CMD --verbose"
    fi

    # Run analysis
    OUTPUT=$(eval $CMD 2>&1)
    EXIT_CODE=$?

    if [ $EXIT_CODE -ne 0 ]; then
        echo -e "${RED}✗ $test_file${NC}"
        echo "$OUTPUT"
        FAILED=1
        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
    else
        if [ "$VERBOSE" = true ]; then
            echo -e "${GREEN}✓ $test_file${NC}"
            echo "$OUTPUT"
        fi
    fi
done

# Final summary
if [ $FAILED -eq 1 ]; then
    echo ""
    echo -e "${RED}Found $TOTAL_VIOLATIONS test quality violation(s)${NC}"
    if [ "$LEVEL" = "error" ]; then
        exit 1
    else
        echo -e "${YELLOW}Running in warning mode - pre-commit will pass${NC}"
        exit 0
    fi
else
    if [ "$VERBOSE" = true ]; then
        echo ""
        echo -e "${GREEN}✓ All test quality checks passed${NC}"
    fi
    exit 0
fi
```

### Step 3.2: Installation Script

**hooks/install-hook.sh**:

```bash
#!/bin/bash
#
# Install test-complexity tool and hook
#

set -e

echo "Installing test-complexity..."

# Build the tool
cargo build --release

# Install binary
if [ -w /usr/local/bin ]; then
    sudo cp target/release/test-complexity /usr/local/bin/
    echo "✓ Installed to /usr/local/bin/test-complexity"
else
    mkdir -p ~/.local/bin
    cp target/release/test-complexity ~/.local/bin/
    echo "✓ Installed to ~/.local/bin/test-complexity"
    echo "  Add ~/.local/bin to PATH if needed"
fi

echo ""
echo "Installation complete!"
echo ""
echo "To use in your project:"
echo "  1. Copy hooks/test-complexity-wrapper.sh to your project's hooks/ directory"
echo "  2. Add to .pre-commit-config.yaml:"
echo ""
echo "    - repo: local"
echo "      hooks:
echo "        - id: test-complexity"
echo "          name: Test Quality Check"
echo "          entry: hooks/test-complexity-wrapper.sh"
echo "          language: script"
echo "          files: ^Test/test_.*\.c$"
echo "          args: [--threshold=70, --level=warn]"
```

### Step 3.3: Example Pre-Commit Config

**example-pre-commit-config.yaml**:

```yaml
# Test Quality Checking with test-complexity
- repo: local
  hooks:
    # Standard test quality check (70% threshold, warning mode)
    - id: test-complexity
      name: Test Quality Check
      entry: hooks/test-complexity-wrapper.sh
      language: script
      files: ^Test/test_.*\.c$
      pass_filenames: true
      args:
        - --threshold=70
        - --level=warn

    # Uncomment for strict enforcement (80% threshold, error mode)
    # - id: test-complexity-strict
    #   name: Test Quality Check (Strict)
    #   entry: hooks/test-complexity-wrapper.sh
    #   language: script
    #   files: ^Test/test_.*\.c$
    #   pass_filenames: true
    #   args:
    #     - --threshold=80
    #     - --level=error
    #     - --verbose

    # Uncomment for verbose output during development
    # - id: test-complexity-verbose
    #   name: Test Quality Check (Verbose)
    #   entry: hooks/test-complexity-wrapper.sh
    #   language: script
    #   files: ^Test/test_.*\.c$
    #   pass_filenames: true
    #   args:
    #     - --threshold=70
    #     - --level=warn
    #     - --verbose
```

## Testing the Implementation

### Unit Tests

**tests/integration_test.rs**:

```rust
use std::process::Command;

#[test]
fn test_simple_pass() {
    let output = Command::new("cargo")
        .args(&["run", "--",
            "tests/fixtures/test_simple.c",
            "tests/fixtures/simple.c",
            "--threshold=0.70"])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success());
}

#[test]
fn test_insufficient_complexity_fail() {
    let output = Command::new("cargo")
        .args(&["run", "--",
            "tests/fixtures/test_insufficient.c",
            "tests/fixtures/complex.c",
            "--threshold=0.70",
            "--level=error"])
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success());
}
```

### Integration Test with Ascent Project

```bash
# Test with actual project
cd /home/tvanfossen/BISSELL/ELEC_SW/d_stcube_ascent_main

# Run on all test files
for test_file in Test/test_*.c; do
    echo "Testing: $test_file"
    test-complexity "$test_file" "$(find Core/Src/modules -name ${test_file#Test/test_})" \
        --threshold=0.70 \
        --level=warn \
        --verbose
done
```

## Future Enhancements

### Phase 4: State Variable Detection
- Detect `static`, `volatile`, and global variables
- Require N test scenarios per state variable
- Validate state transitions are tested

### Phase 5: Advanced Boundary Detection
- Detect array bounds
- Detect buffer overflow scenarios
- Detect loop boundary conditions
- Suggest specific test cases

### Phase 6: Integration with tools_knots
- Unified complexity checking tool
- Combined reports
- Shared configuration

### Phase 7: Mutation Testing Integration
- Use mutation testing as oracle
- Validate tests actually catch bugs
- Complement complexity-based approach

## Troubleshooting

### Common Issues

1. **Tool not found in PATH**
   ```bash
   export PATH="$HOME/.local/bin:$PATH"
   ```

2. **Source file not found**
   - Check file naming convention: `test_foo.c` → `foo.c`
   - Verify search path in wrapper script

3. **False positives**
   - Adjust threshold (lower if tests use helpers)
   - Use `--verbose` to understand scoring

4. **Integration with existing project**
   - Start with `--level=warn` to gather data
   - Review warnings, adjust threshold
   - Promote to `--level=error` after tuning

## References

- **tools_knots**: `/home/tvanfossen/BISSELL/ELEC_SW/tools_knots/`
- **tree-sitter-c**: https://github.com/tree-sitter/tree-sitter-c
- **Cognitive Complexity**: https://www.sonarsource.com/resources/cognitive-complexity/
- **McCabe Complexity**: https://en.wikipedia.org/wiki/Cyclomatic_complexity

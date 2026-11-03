# knots-test-complexity

A Rust-based test quality analyzer for C projects that validates unit tests have sufficient complexity to thoroughly exercise source code.

## Motivation

Traditional code coverage metrics (line, branch, function) can be misleading. A test can achieve 100% branch coverage with simple assertions while missing critical edge cases like:

- **Overflow scenarios**: `uint16_t` timer wrapping at 65535
- **Boundary conditions**: Off-by-one errors at array bounds
- **State transitions**: Complex state machines with temporal dependencies
- **Error paths**: Multiple error conditions not all tested

This tool enforces that tests have sufficient **cyclomatic complexity** to exercise all logical paths and **boundary value testing** to catch edge cases.

## Philosophy

> "A test with lower complexity than its source code is likely not testing all scenarios."

### Example: The Overflow Bug

```c
// Source: Cyclomatic Complexity = 2
uint16_t timer_ms = 0;

void periodic_1ms() {
    timer_ms++;  // Overflows at 65535!
}

bool is_timeout(uint16_t start_ms, uint16_t duration_ms) {
    return (timer_ms - start_ms) >= duration_ms;
}
```

**Traditional Coverage** (100% line, 100% branch):
```c
void test_timeout() {
    timer_ms = 0;
    TEST_ASSERT_TRUE(is_timeout(0, 100));   // Happy path
    TEST_ASSERT_FALSE(is_timeout(0, 1));    // Boundary
}
// PASSES coverage but MISSES overflow bug!
```

**Tool Enforcement** - Would detect:
- Test complexity (2) barely meets source complexity (2)
- Missing boundary tests: 0, 65535, wrap-around scenarios
- Missing state variation: timer_ms at different values

**Better Tests** (Higher Complexity):
```c
void test_timeout_boundaries() {
    // Boundary: timer at 0
    timer_ms = 0;
    TEST_ASSERT_TRUE(is_timeout(0, 100));

    // Boundary: timer near max
    timer_ms = 65530;
    TEST_ASSERT_TRUE(is_timeout(65520, 100));

    // CRITICAL: Overflow scenario
    timer_ms = 5;  // Wrapped from 65535
    TEST_ASSERT_TRUE(is_timeout(65530, 100));  // Catches overflow!

    // Multiple start/duration combinations
    for (int i = 0; i < 5; i++) {
        test_scenario(scenarios[i]);
    }
}
// Higher complexity test catches the bug!
```

## Features

### Core Metrics

1. **Test-to-Source Complexity Ratio**
   - Aggregate cyclomatic complexity of all test functions
   - Compare to aggregate complexity of source functions
   - Default threshold: 70% (configurable)

2. **Boundary Value Detection**
   - Detects integer types: `uint8_t`, `uint16_t`, `uint32_t`, `int8_t`, etc.
   - Identifies range checks: `if (x > MAX)`, `if (x < MIN)`
   - Counts required boundary tests
   - Validates tests cover: MIN, MIN-1, MAX, MAX+1

3. **State Variable Tracking** (Future Enhancement)
   - Identifies `static`, `volatile`, and global variables
   - Requires multiple test scenarios per state variable
   - Validates state transitions are tested

### Output Modes

- **Warning Mode** (`--level=warn`, the default when the binary is run directly): Reports violations and exits 0
- **Error Mode** (`--level=error`, what the published hook runs): Exits 1 on violations, failing the commit
- **Verbose Mode**: Shows detailed per-function complexity breakdown

## Building

```bash
cargo build --release --workspace
```

The binary will be at `target/release/knots-test-complexity`

## Installation

### From Source

```bash
cd knots
cargo build --release --workspace
# Binary is at target/release/knots-test-complexity
```

Add to your PATH or copy to a location in your PATH.

### Pre-Commit Integration

Add to your project's `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/brandon-arrendondo/knots
    rev: v2.1.0  # a released knots tag
    hooks:
      - id: test-complexity
        args:
          - --threshold=0.70
          - --boundary-threshold=0.80
          - --level=error
          - --framework=ceedling
          - --test-dir=Test
```

**Configuration Options:**

- `--threshold=0.70`: Minimum test-to-source complexity ratio (default: 0.70 = 70%)
- `--boundary-threshold=0.80`: Minimum boundary test coverage ratio (default: 0.80 = 80%)
- `--level`: Enforcement level, `warn` or `error`. At `error` a failing test
  stops the commit; at `warn` it is reported and the commit goes ahead. The
  published hook passes `--level=error`. Setting `args:` replaces that, so
  include `--level=error` in your own args to keep blocking, or
  `--level=warn` to opt out. Run directly, `knots-test-complexity` defaults
  to `warn`.
- `--no-check-boundaries`: Disable boundary value detection (enabled by default)
- `--verbose`: Show detailed per-function analysis

**Example: Strict Enforcement**
```yaml
args:
  - --threshold=0.80
  - --level=error
  - --verbose
```

**Example: Warning Only (No Boundaries)**
```yaml
args:
  - --threshold=0.70
  - --level=warn
  - --no-check-boundaries
```

## Usage

### Command Line

Analyze a test file and its corresponding source:

```bash
knots-test-complexity Test/test_battery_service.c Core/Src/modules/battery_service/battery_service.c
```

With verbose output:

```bash
knots-test-complexity -v Test/test_battery_service.c Core/Src/modules/battery_service/battery_service.c
```

With custom thresholds:

```bash
knots-test-complexity \
  --threshold=0.70 \
  --boundary-threshold=0.80 \
  --level=error \
  Test/test_timer.c Core/Src/timer.c
```

### Output Example

From the files in `examples/`:
`knots-test-complexity examples/test_timer_good.c examples/timer.c`

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Test Quality Analysis: test_timer_good.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Source File:
  File: timer.c
  Functions: 7
  Total Cyclomatic Complexity: 13
  Total Cognitive Complexity: 7

Test File:
  File: test_timer_good.c
  Functions: 7
  Total Cyclomatic Complexity: 12
  Total Cognitive Complexity: 7

Complexity Analysis:
  Test/Source Ratio: 92% ✓ (threshold: 70%)
  Test Cyclomatic Complexity: 12
  Source Cyclomatic Complexity: 13

Boundary Analysis:
  Integer Boundaries Detected: 6
  Pointer Boundaries Detected: 0
  Boundary Test Coverage: 86% ✓ (threshold: 80%)
  Test Values Found: 30

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Result: ✓ PASS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Failure Example

`knots-test-complexity --level=error examples/test_timer_bad.c examples/timer.c`
exits 1:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Test Quality Analysis: test_timer_bad.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Source File:
  File: timer.c
  Functions: 7
  Total Cyclomatic Complexity: 13
  Total Cognitive Complexity: 7

Test File:
  File: test_timer_bad.c
  Functions: 4
  Total Cyclomatic Complexity: 4
  Total Cognitive Complexity: 0

Complexity Analysis:
  Test/Source Ratio: 30% ✗ (threshold: 70%)
  Test Cyclomatic Complexity: 4
  Source Cyclomatic Complexity: 13

Boundary Analysis:
  Integer Boundaries Detected: 6
  Pointer Boundaries Detected: 0
  Boundary Test Coverage: 79% ✗ (threshold: 80%)
  Test Values Found: 12

Recommendations:
Add ~4 more complexity points to tests (39 percentage points below threshold)
Consider adding:
  - Edge case tests (boundary values, overflow scenarios)
  - Error path tests (invalid inputs, error conditions)
  - State transition tests (different initial conditions)
  - Parametrized tests or loops in test code

Missing boundary value tests:
  1. constant_200 (range_check_upper): missing values [198, 199, 201]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Result: ✗ FAIL
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Algorithm Details

### 1. Complexity Ratio Calculation

```
For each test file:
  1. Parse test file (knots picks the grammar from the file's extension)
  2. Calculate cyclomatic complexity for all test functions
     - Count: if, while, for, switch, &&, ||, ?:
     - Include test helper functions
  3. Sum total test complexity

  4. Find corresponding source file
  5. Calculate cyclomatic complexity for all source functions
  6. Sum total source complexity

  7. Calculate ratio = test_complexity / source_complexity
  8. Compare ratio >= threshold (default 70%)

  9. Report: PASS or FAIL with recommendations
```

### 2. Boundary Value Detection

```
For source file:
  1. Find all integer type declarations
     - uint8_t → boundaries: 0, 255
     - uint16_t → boundaries: 0, 65535
     - int8_t → boundaries: -128, 127

  2. Find all range checks
     - if (x > MAX) → test MAX, MAX+1
     - if (x < MIN) → test MIN-1, MIN
     - if (x >= threshold) → test threshold-1, threshold

  3. Count required boundary tests

For test file:
  1. Find all numeric literals in assertions
  2. Match literals to source boundaries
  3. Count covered boundaries

  4. Report: boundary_coverage = found / required
  5. Warn if coverage < 100%
```

### 3. Test Helper Function Handling

Test helpers ARE included in complexity calculation:

```c
// Helper complexity counts!
void simulate_frame(uint8_t id) {
    setup_mocks();
    if (id == SPECIAL) {  // +1 complexity
        special_handling();
    }
    verify_results();
}

// Test also counts
void test_multiple_frames() {
    for (int i = 0; i < 10; i++) {  // +1 complexity
        simulate_frame(i);  // Helper complexity included
    }
}
// Total test complexity = 2 (loop + helper's if)
```

This encourages well-structured tests with reusable helpers.

## Integration with knots

`knots-test-complexity` complements `knots`:

| Tool | Purpose | Applied To | Metric |
|------|---------|------------|--------|
| **knots** | Source code quality | Production code (`.c`, `.h`) | McCabe & Cognitive complexity per function |
| **knots-test-complexity** | Test quality | Test code (`test_*.c`) | Aggregate complexity ratio & boundary coverage |

**Example Combined Workflow:**

```yaml
- repo: local
  hooks:
    # Check source code complexity (per-function limits)
    - id: knots
      name: Code Complexity Check
      entry: hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      exclude: ^Test/
      args: [--mccabe-threshold=15, --cognitive-threshold=15]

    # Check test quality (aggregate complexity ratio)
    - id: test-complexity
      name: Test Quality Check
      entry: hooks/test-complexity-wrapper.sh
      language: script
      files: (^|/)(test_[^/]+|[^/]+_test)\.(c|cpp|cc|cxx)$
      args: [--threshold=0.70, --level=error]
```

### Future: Unified Tool

A future enhancement could merge both tools:

```bash
# Unified complexity tool
complexity-check --source <file.c> --test <test_file.c>
  --source-mccabe-max=15
  --source-cognitive-max=15
  --test-ratio-min=0.70
  --check-boundaries
```

## Dependencies

The crate depends on `knots` itself (for parsing and the complexity metrics),
plus `tree-sitter`, `anyhow`, `clap`, `colored` and `regex`, with versions set
in the workspace's root `Cargo.toml`.

## Project Structure

```
knots-test-complexity/
├── Cargo.toml                          # Rust project manifest
├── pyproject.toml                      # PyPI wheel build (maturin)
├── LICENSE-MIT                         # License
├── README.md                           # This file
├── src/
│   ├── main.rs                        # CLI entry point
│   ├── analyzer.rs                    # Test quality analyzer
│   ├── boundary.rs                    # Boundary value detector
│   └── reporter.rs                    # Output formatting
└── examples/
    ├── timer.c                        # Example source
    ├── test_timer_good.c              # Example: sufficient complexity
    ├── test_timer_bad.c               # Example: insufficient complexity
    ├── sensor.c                       # Example source
    ├── test_sensor_boundaries.c       # Example: boundary-value tests
    └── README.md                      # Example documentation
```

## Testing

Run the test suite:

```bash
cargo test
```

Run with verbose output:

```bash
cargo test -- --nocapture
```

## License

MIT License. See LICENSE-MIT.

## See Also

- **knots**: Source code complexity analyzer
- **pmccabe**: Industry-standard McCabe complexity tool (validation reference)
- **Cognitive Complexity**: [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)
- **Mutation Testing**: Alternative approach for test quality (future consideration)

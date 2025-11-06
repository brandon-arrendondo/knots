# tool_test_complexity

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

- **Warning Mode** (default): Reports violations but doesn't fail pre-commit
- **Error Mode**: Fails pre-commit on violations
- **Verbose Mode**: Shows detailed per-function complexity breakdown

## Building

```bash
cargo build --release
```

The binary will be at `target/release/test-complexity`

## Installation

### Quick Install

```bash
cd /home/tvanfossen/BISSELL/ELEC_SW/tool_test_complexity
./hooks/install-hook.sh
```

This installs the binary to `~/.local/bin/test-complexity`. For system-wide installation, use `./hooks/install-hook.sh --system`.

### Pre-Commit Integration

#### Local Development Setup

For local development and testing, add to your project's `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: test-complexity
        name: Test Quality Check (Complexity & Boundaries)
        entry: /home/tvanfossen/BISSELL/ELEC_SW/tool_test_complexity/hooks/test-complexity-wrapper.sh
        language: script
        files: ^Test/test_.*\.c$
        pass_filenames: true
        args:
          - --threshold=0.70
          - --level=warn
```

#### Production Deployment

When deployed as a proper pre-commit repository (future):

```yaml
repos:
  - repo: https://github.com/your-org/tool_test_complexity
    rev: v1.0.0  # Use specific version tag
    hooks:
      - id: test-complexity
        args:
          - --threshold=0.70
          - --level=warn
```

**Configuration Options:**

- `--threshold=0.70`: Minimum test-to-source complexity ratio (default: 0.70 = 70%)
- `--level=warn`: Enforcement level (`warn` or `error`, default: `warn`)
- `--no-check-boundaries`: Disable boundary value detection (enabled by default)
- `--verbose`: Show detailed per-file analysis

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
test-complexity Test/test_battery_service.c Core/Src/modules/battery_service/battery_service.c
```

With verbose output:

```bash
test-complexity -v Test/test_battery_service.c Core/Src/modules/battery_service/battery_service.c
```

Check all test files in a project:

```bash
test-complexity --check-all Test/ Core/Src/modules/
```

### Output Example

```
Analyzing Test Quality: test_battery_service.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Source File: battery_service.c
  Functions: 15
  Total Cyclomatic Complexity: 87
  Boundary Values Detected: 12
    - uint8_t variables: 4 (boundaries: 0, 255)
    - Range checks: 8 (if (x > MAX), etc.)

Test File: test_battery_service.c
  Functions: 58
  Total Cyclomatic Complexity: 74
  Boundary Tests Found: 15

Complexity Analysis:
  Test/Source Ratio: 85% ✓ (threshold: 70%)
  Test Complexity: 74
  Source Complexity: 87
  Ratio: 74/87 = 0.85

Boundary Analysis:
  Required Boundary Tests: 12
  Found Boundary Tests: 15 ✓
  Coverage: 125%

Result: ✓ PASS

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Failure Example

```
Analyzing Test Quality: test_lin_comm_service.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Source File: lin_comm_service.c
  Functions: 8
  Total Cyclomatic Complexity: 54
  Boundary Values Detected: 8

Test File: test_lin_comm_service.c
  Functions: 12
  Total Cyclomatic Complexity: 28
  Boundary Tests Found: 3

Complexity Analysis:
  Test/Source Ratio: 52% ✗ (threshold: 70%)
  Test Complexity: 28
  Source Complexity: 54
  Ratio: 28/54 = 0.52

Boundary Analysis:
  Required Boundary Tests: 8
  Found Boundary Tests: 3 ✗
  Missing Boundaries:
    - rxByteCounter: 0, 3, 11, 12 (RX_HEADER_SIZE boundaries)
    - rxFrameId: 0, 0x3F (FRAME_MASK boundary)

Recommendations:
  1. Add tests for edge cases and error paths
  2. Test boundary conditions: 0, max values, overflow
  3. Add state transition tests (4 static variables detected)
  4. Consider parametrized tests or loops in test code

Result: ✗ FAIL (--level=error)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Algorithm Details

### 1. Complexity Ratio Calculation

```
For each test file:
  1. Parse test file with tree-sitter-c
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

## Integration with tools_knots

`tool_test_complexity` complements `tools_knots`:

| Tool | Purpose | Applied To | Metric |
|------|---------|------------|--------|
| **tools_knots** | Source code quality | Production code (`.c`, `.h`) | McCabe & Cognitive complexity per function |
| **tool_test_complexity** | Test quality | Test code (`test_*.c`) | Aggregate complexity ratio & boundary coverage |

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
      files: ^Test/test_.*\.c$
      args: [--threshold=70, --level=warn]
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

```toml
[dependencies]
tree-sitter = "0.22"
tree-sitter-c = "0.21"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
regex = "1.10"
```

## Project Structure

```
tool_test_complexity/
├── Cargo.toml                          # Rust project manifest
├── Cargo.lock                          # Dependency lock file
├── README.md                           # This file
├── IMPLEMENTATION.md                   # Detailed implementation guide
├── ALGORITHM.md                        # Algorithm specifications
├── example-pre-commit-config.yaml     # Example pre-commit setup
├── hooks/
│   ├── test-complexity-wrapper.sh     # Pre-commit wrapper script
│   ├── install-hook.sh                # Installation script
│   └── README.md                      # Hook usage guide
├── src/
│   ├── main.rs                        # CLI entry point
│   ├── complexity.rs                  # Cyclomatic complexity calculator
│   ├── boundary.rs                    # Boundary value detector
│   ├── analyzer.rs                    # Test quality analyzer
│   └── reporter.rs                    # Output formatting
├── examples/
│   ├── good_test.c                    # Example: sufficient complexity
│   ├── bad_test.c                     # Example: insufficient complexity
│   └── boundary_test.c                # Example: boundary testing
└── tests/
    ├── integration_test.rs            # Integration tests
    └── fixtures/                      # Test fixtures
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

Internal BISSELL tool.

## See Also

- **tools_knots**: Source code complexity analyzer
- **pmccabe**: Industry-standard McCabe complexity tool (validation reference)
- **Cognitive Complexity**: [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)
- **Mutation Testing**: Alternative approach for test quality (future consideration)

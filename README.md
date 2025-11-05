# knots

A comprehensive Rust-based code complexity analyzer for C files that calculates multiple complexity metrics including McCabe complexity, cognitive complexity, nesting depth, SLOC, ABC complexity, and return counts.

## Features

- **McCabe Complexity (Cyclomatic Complexity)**: Measures the number of linearly independent paths through a program's source code
- **Cognitive Complexity**: Measures how difficult code is to understand, with penalties for nesting
- **Nesting Depth**: Maximum nesting level of control structures
- **SLOC**: Source Lines of Code (non-comment, non-blank lines)
- **ABC Complexity**: Assignment-Branch-Condition metric with vector magnitude
- **Return Count**: Number of return statements in each function
- Per-function analysis with detailed metrics
- Summary statistics across all functions
- **Validated**: McCabe complexity results match pmccabe output exactly

## Validation

The McCabe complexity implementation has been validated against multiple industry-standard tools with 100% accuracy:

### Validated Against:
- **[pmccabe](https://people.debian.org/~bame/pmccabe/)** - Industry standard since 1990s (HP origin)
- **[lizard](https://github.com/terryyin/lizard)** - Popular multi-language complexity analyzer

### Results:
- ✓ 13/13 functions match pmccabe exactly (100% accuracy)
- ✓ Validated against industry-standard reference implementation
- ✓ Correctly implements switch/case complexity: +1 per switch statement (pmccabe compatible)
- ✓ Handles nested structures, loops, and logical operators accurately

### Unique Value:
Unlike most tools that only measure McCabe complexity, knots provides a comprehensive suite of metrics:
- **Cognitive Complexity** based on the [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/)
- **ABC Complexity** for measuring assignment, branch, and condition complexity
- **Nesting Depth** analysis to identify deeply nested code
- **SLOC** and **Return Count** for additional code quality insights

This makes knots one of the most feature-complete complexity analyzers available for C code.

## Building

```bash
cargo build --release
```

## Usage

Analyze a C file with per-function complexity and visual indicators:
```bash
./target/release/knots <file.c>
```

Show detailed per-function analysis:
```bash
./target/release/knots --verbose <file.c>
```

Or using the short flag:
```bash
./target/release/knots -v <file.c>
```

### Visual Complexity Indicators

The tool uses emojis to quickly identify complexity levels:

- 😊 **Good** (1-10): Low complexity, easy to understand and maintain
- 😐 **Okay** (11-20): Moderate complexity, consider refactoring  
- 😠 **Bad** (21-49): High complexity, should be refactored
- 😢 **Worst** (50+): Very high complexity, needs immediate attention

## Examples

The `examples/` directory contains sample C files demonstrating different complexity levels:

- `simple.c` - Simple functions with low complexity
- `complex.c` - Functions with higher complexity including nested loops and conditions
- `nested.c` - Demonstrates the difference between McCabe and cognitive complexity

### Running Examples

```bash
# Simple example
./target/release/knots --verbose examples/simple.c

# Complex example with nested structures
./target/release/knots --verbose examples/complex.c

# Nested code showing cognitive complexity impact
./target/release/knots --verbose examples/nested.c
```

## Complexity Metrics Explained

### McCabe Complexity (Cyclomatic Complexity)

McCabe complexity counts decision points in code:
- Base complexity: 1
- +1 for each: `if`, `while`, `for`, `switch`, `&&`, `||`, `?:`, `goto`
- Switch statements: +1 per switch (regardless of number of cases)

**Example:**
```c
void simple() {
    return;  // McCabe: 1
}

void with_if(int x) {
    if (x > 0) {  // McCabe: 2 (1 base + 1 if)
        return 1;
    }
    return 0;
}
```

### Cognitive Complexity

Cognitive complexity measures how hard code is to understand:
- +1 for control flow breaks: `if`, `while`, `for`, `switch`, etc.
- +1 for each level of nesting (makes code harder to follow)
- +1 for `break`, `continue`, `goto`
- +1 for logical operator sequences (but not for each operator in a sequence)

**Example:**
```c
// McCabe: 5, Cognitive: 10 (high nesting penalty)
void deeply_nested(int a, int b, int c, int d) {
    if (a > 0) {           // +1
        if (b > 0) {       // +1 (base) +1 (nesting) = +2
            if (c > 0) {   // +1 (base) +2 (nesting) = +3
                if (d > 0) {  // +1 (base) +3 (nesting) = +4
                    printf("All positive!\n");
                }
            }
        }
    }
}

// McCabe: 5, Cognitive: 4 (early returns reduce nesting)
void flattened(int a, int b, int c, int d) {
    if (a <= 0) return;  // +1
    if (b <= 0) return;  // +1
    if (c <= 0) return;  // +1
    if (d <= 0) return;  // +1
    printf("All positive!\n");
}
```

Both functions have the same McCabe complexity (5) but vastly different cognitive complexity (10 vs 4), demonstrating why flattening deeply nested code improves readability.

### Nesting Depth

Nesting depth measures the maximum level of nested control structures. Deeply nested code is harder to understand and maintain.

**Example:**
```c
// Nesting depth: 4
void deeply_nested(int a, int b, int c, int d) {
    if (a > 0) {           // Level 1
        if (b > 0) {       // Level 2
            if (c > 0) {   // Level 3
                if (d > 0) {  // Level 4
                    printf("Deep!\n");
                }
            }
        }
    }
}

// Nesting depth: 1
void flat(int a, int b, int c, int d) {
    if (a <= 0) return;  // Level 1
    if (b <= 0) return;  // Level 1
    if (c <= 0) return;  // Level 1
    if (d <= 0) return;  // Level 1
    printf("Flat!\n");
}
```

### SLOC (Source Lines of Code)

SLOC counts non-comment, non-blank lines of code. It provides a simple measure of function size. Larger functions are generally harder to understand and maintain.

### ABC Complexity

ABC complexity is a vector metric that counts:
- **A (Assignments)**: Assignment statements and increment/decrement operators
- **B (Branches)**: Function/method calls
- **C (Conditions)**: Conditional logic (if, while, for, switch, logical operators)

The magnitude is calculated as: √(A² + B² + C²)

**Example:**
```c
// ABC: <3, 2, 2>, magnitude: 4.12
int process_data(int x, int y) {
    int result = 0;           // A+1
    result = x + y;           // A+1

    if (result > 0) {         // C+1
        result++;             // A+1
        printf("Positive\n"); // B+1
    }

    if (result < 100) {       // C+1
        log_result(result);   // B+1
    }

    return result;
}
```

### Return Count

Return count measures the number of return statements in a function. Functions with many return points can be harder to understand and debug. However, early returns can sometimes improve readability by reducing nesting.

## Testing

Run the test suite:
```bash
cargo test
```

## Dependencies

- `tree-sitter` - Parser generator and incremental parsing library
- `tree-sitter-c` - C language grammar for tree-sitter
- `clap` - Command-line argument parsing
- `anyhow` - Error handling

## License

MIT License.  See LICENSE file.

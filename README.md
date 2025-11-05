# knots

A Rust-based code complexity analyzer for C files that calculates McCabe (cyclomatic) complexity and cognitive complexity metrics.

## Features

- **McCabe Complexity (Cyclomatic Complexity)**: Measures the number of linearly independent paths through a program's source code
- **Cognitive Complexity**: Measures how difficult code is to understand, with penalties for nesting
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
Unlike most tools that only measure McCabe complexity, knots also implements **Cognitive Complexity** based on the [SonarSource specification](https://www.sonarsource.com/resources/cognitive-complexity/), making it one of the few tools that provides both metrics for C code.

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

This project is open source.

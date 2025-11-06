# knots-test-complexity Examples

This directory contains example C source files and their corresponding test files to demonstrate the `knots-test-complexity` tool and best practices for writing thorough unit tests.

## Overview

The examples show:
- ✅ **Good tests** with sufficient complexity and boundary coverage
- ❌ **Bad tests** with insufficient coverage (what NOT to do)
- 🎯 **Boundary testing** patterns and best practices
- 🐛 **Common bugs** that good tests catch

## Example Files

### 1. Timer Example (Overflow Bug Detection)

**Source:** `timer.c`
- Cyclomatic Complexity: ~10
- Boundaries: `uint16_t` timer (0-65535), `uint8_t` values (0-255)
- Contains classic overflow bug: timer wraps at 65535

**Good Test:** `test_timer_good.c`
```bash
knots-knots-test-complexity examples/test_timer_good.c examples/timer.c
```

Expected: **PASS**
- Test Complexity: ~12 (> 70% of source)
- Boundary Coverage: ~90%
- Tests overflow scenario, boundaries (0, 255, 65535)

**Bad Test:** `test_timer_bad.c`
```bash
knots-test-complexity examples/test_timer_bad.c examples/timer.c
```

Expected: **FAIL**
- Test Complexity: ~3 (< 70% of source)
- Boundary Coverage: ~20%
- Achieves 100% line coverage but **MISSES the overflow bug!**

**Key Lesson:** Traditional coverage metrics can be misleading. The bad test passes all branches but fails to catch the critical overflow bug.

---

### 2. Sensor Example (Boundary Value Testing)

**Source:** `sensor.c`
- Cyclomatic Complexity: ~10
- Focus: Integer boundaries, range checks, thresholds

**Test:** `test_sensor_boundaries.c`
```bash
knots-test-complexity examples/test_sensor_boundaries.c examples/sensor.c
```

Expected: **PASS**
- Demonstrates thorough boundary testing
- Tests: MIN, MIN-1, MAX, MAX+1 for all integer types
- Tests threshold values: value-1, value, value+1

**Key Lesson:** Boundary value analysis is critical. Test:
- 0 and MAX for all integer types
- Threshold ± 1 for all conditional checks
- Overflow/underflow scenarios

---

## Running the Examples

### Test All Examples

```bash
# Good test (should pass)
knots-test-complexity examples/test_timer_good.c examples/timer.c --verbose

# Bad test (should fail)
knots-test-complexity examples/test_timer_bad.c examples/timer.c --verbose

# Boundary test (should pass)
knots-test-complexity examples/test_sensor_boundaries.c examples/sensor.c --verbose
```

### Compare Good vs Bad Tests

Run both tests against the same source to see the difference:

```bash
echo "=== Good Test ==="
knots-test-complexity examples/test_timer_good.c examples/timer.c

echo ""
echo "=== Bad Test ==="
knots-test-complexity examples/test_timer_bad.c examples/timer.c
```

### Compile and Run the Test Code

The examples are compilable C code:

```bash
# Compile and run the good test
gcc -o test_timer_good examples/test_timer_good.c examples/timer.c
./test_timer_good

# Compile and run the bad test
gcc -o test_timer_bad examples/test_timer_bad.c examples/timer.c
./test_timer_bad

# Compile and run the boundary test
gcc -o test_sensor_boundaries examples/test_sensor_boundaries.c examples/sensor.c
./test_sensor_boundaries
```

---

## What Makes a Good Test?

### 1. Sufficient Complexity

Tests should have complexity proportional to the source code:

```c
// Source function (Complexity: 4)
int validate(int x, int y) {
    if (x < 0) return -1;
    if (y < 0) return -1;
    if (x > y) return 0;
    return 1;
}

// Bad test (Complexity: 1) - Happy path only
void test_validate_bad() {
    assert(validate(5, 10) == 1);
}

// Good test (Complexity: 4+) - All paths
void test_validate_good() {
    assert(validate(-1, 10) == -1);  // x < 0
    assert(validate(5, -1) == -1);   // y < 0
    assert(validate(15, 10) == 0);   // x > y
    assert(validate(5, 10) == 1);    // valid
    assert(validate(0, 0) == 1);     // boundary
}
```

### 2. Boundary Value Testing

For every integer type, test:
- **MIN**: 0 for unsigned, INT_MIN for signed
- **MIN-1**: Underflow case (if possible)
- **MAX**: 255 for uint8_t, 65535 for uint16_t, etc.
- **MAX+1**: Overflow case (wraps or saturates)

For every threshold/condition:
```c
if (x > THRESHOLD) { ... }

// Test these values:
THRESHOLD - 1  // Just below
THRESHOLD      // Exactly at
THRESHOLD + 1  // Just above
```

### 3. Test Edge Cases and Error Paths

```c
// Source
int divide(int a, int b) {
    if (b == 0) return -1;  // Error
    return a / b;
}

// Good test covers both paths
void test_divide() {
    assert(divide(10, 2) == 5);      // Normal case
    assert(divide(10, 0) == -1);     // Error case
    assert(divide(0, 5) == 0);       // Zero numerator
    assert(divide(INT_MAX, 1) == INT_MAX);  // Boundary
}
```

### 4. Parameterized Tests for Loops

When source has loops, tests should too:

```c
// Source
bool all_positive(int arr[], int size) {
    for (int i = 0; i < size; i++) {
        if (arr[i] <= 0) return false;
    }
    return true;
}

// Good test uses loops and multiple scenarios
void test_all_positive() {
    int test_cases[][5] = {
        {1, 2, 3, 4, 5},      // All positive
        {-1, 2, 3, 4, 5},     // First negative
        {1, 2, 3, 4, -5},     // Last negative
        {0, 0, 0, 0, 0},      // All zero
    };

    for (int i = 0; i < 4; i++) {
        bool result = all_positive(test_cases[i], 5);
        // Assert based on case
    }
}
```

---

## Common Anti-Patterns

### ❌ Testing Only Happy Paths

```c
// BAD: Only tests success case
void test_file_read() {
    char *content = read_file("test.txt");
    assert(content != NULL);
}

// GOOD: Tests error cases too
void test_file_read() {
    assert(read_file("test.txt") != NULL);     // Exists
    assert(read_file("missing.txt") == NULL);  // Missing
    assert(read_file(NULL) == NULL);           // Null path
    assert(read_file("") == NULL);             // Empty path
}
```

### ❌ Missing Boundary Tests

```c
// BAD: Only tests middle values
void test_scale() {
    assert(scale_value(50) == 127);  // Middle value only
}

// GOOD: Tests boundaries
void test_scale() {
    assert(scale_value(0) == 0);     // MIN
    assert(scale_value(255) == 255); // MAX
    assert(scale_value(1) > 0);      // MIN+1
    assert(scale_value(254) < 255);  // MAX-1
    assert(scale_value(50) == 127);  // Middle
}
```

### ❌ Missing Overflow Tests

```c
// BAD: No overflow test
void test_timer() {
    set_timer(100);
    assert(get_timer() == 100);
}

// GOOD: Tests overflow
void test_timer() {
    set_timer(65535);     // MAX
    increment_timer();
    assert(get_timer() == 0);  // Wraps to 0
}
```

---

## Tool Output Interpretation

### Passing Test

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Test Quality Analysis: test_timer_good.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Complexity Analysis:
  Test/Source Ratio: 120% ✓ (threshold: 70%)
  Test Complexity: 12
  Source Complexity: 10

Boundary Analysis:
  Boundary Test Coverage: 90% ✓
  Test Values Found: 18

Result: ✓ PASS
```

### Failing Test

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Test Quality Analysis: test_timer_bad.c
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Complexity Analysis:
  Test/Source Ratio: 30% ✗ (threshold: 70%)
  Test Complexity: 3
  Source Complexity: 10

Boundary Analysis:
  Boundary Test Coverage: 20% ✗
  Missing Boundaries:
    - timer_ms: missing values [0, 65535, overflow]
    - input: missing values [0, 255]

Recommendations:
  - Add ~4 more complexity points to tests
  - Test boundary values: 0, 255, 65535
  - Test overflow scenario (65535 -> 0)
  - Add error path tests

Result: ✗ FAIL
```

---

## Best Practices Summary

1. **Match or exceed source complexity** (>70% ratio)
2. **Test all boundaries** for every integer type
3. **Test overflow/underflow** scenarios
4. **Test error paths** not just happy paths
5. **Use loops in tests** when source has loops
6. **Test threshold values** (value-1, value, value+1)
7. **Add parametrized tests** for multiple scenarios

Remember: **"A test with lower complexity than its source code is likely not testing all scenarios."**

---

## Further Reading

- Main README: `../README.md`
- Implementation details: `../IMPLEMENTATION.md`
- Algorithm specification: `../ALGORITHM.md`
- Hook configuration: `../hooks/README.md`

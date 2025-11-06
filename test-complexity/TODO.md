# test-complexity TODO

## Current Implementation

The tool currently enforces:
1. **Cyclomatic Complexity Ratio ≥ 70%** (configurable): Tests must exercise decision paths proportional to source code
2. **Boundary Coverage ≥ 80%** (configurable): Tests must hit edge cases (0, MAX, -1, overflow values)

Cognitive complexity is **tracked and reported** but not used in pass/fail determination.

## Future Enhancements

### 1. Cognitive Complexity CEILING for Tests

**Problem**: Tests should be simple and easy to understand. Complex test logic is hard to debug and maintain.

**Proposed Feature**: Enforce that test functions stay cognitively simple.

**Implementation Options**:

#### Option A: Per-Function Ceiling
- **Fail** if any single test function has cognitive complexity > 5
- **Warn** if average test cognitive complexity > 3

#### Option B: Relative Ceiling
- **Warn** when test cognitive complexity exceeds source cognitive complexity
- This catches cases where tests are more complex than the code they're testing

#### Option C: Configurable Threshold
Add CLI options:
```bash
test-complexity test.c source.c \
  --max-test-cognitive=5 \
  --avg-test-cognitive=3
```

**Benefits**:
- Enforces test simplicity and readability
- Encourages splitting complex tests into multiple focused tests
- Aligns with testing best practices ("tests should be obvious")
- Catches overly "clever" parametrized tests

**Example**:
```c
// BAD: Cognitive complexity = 8, too complex for a test
void test_AllCases() {
    for (int i = 0; i < 10; i++) {
        if (cases[i].type == A) {
            for (int j = 0; j < subcases; j++) {
                if (nested_condition) {
                    // Too deeply nested!
                }
            }
        } else if (cases[i].type == B) {
            // More branching
        }
    }
}

// GOOD: Split into simple tests, each cognitive = 1
void test_CaseA_Subcase1() { assert(...); }
void test_CaseA_Subcase2() { assert(...); }
void test_CaseB() { assert(...); }
```

**Implementation Notes**:
- Add check in `analyzer.rs` after calculating metrics
- Report violations in recommendations section
- Make configurable via CLI argument
- Consider making it a warning by default, not a hard failure

---

### 2. Integration Test Support

Currently only analyzes unit tests. Consider:
- Support for integration test patterns
- Multiple source files per test
- Different thresholds for integration vs unit tests

---

### 3. Test Function Name Validation

Warn if test function names don't match patterns like:
- `test_*`
- `TEST_*`
- Functions that call test frameworks but aren't named appropriately

---

### 4. Mock/Stub Detection

Track ratio of mock calls to real function calls to detect:
- Over-mocking (testing the mocks, not the code)
- Under-mocking (integration test masquerading as unit test)

---

## Notes

- Cognitive complexity data is already collected and stored in `FunctionMetrics`
- Reporter already has `--verbose` mode for detailed output
- Threshold infrastructure is in place via `TestQualityAnalyzer`

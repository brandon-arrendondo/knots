// test_timer_bad.c - BAD TEST with insufficient complexity and boundary coverage
//
// Expected Result: FAIL
// - Test Complexity: ~3 (< 70% of source complexity ~10)
// - Boundary Coverage: ~20% (misses 0, 255, 65535, overflow scenarios)
// - This would get 100% line/branch coverage but MISS the overflow bug!

#include <stdint.h>
#include <stdbool.h>
#include <assert.h>
#include <stdio.h>

// Forward declarations from timer.c
void timer_init(void);
void timer_increment(void);
bool is_timeout(uint16_t start_ms, uint16_t duration_ms);
uint8_t scale_value(uint8_t input);
int validate_range(uint8_t value, uint8_t min, uint8_t max);
uint16_t get_timer_ms(void);
void set_timer_ms(uint16_t value);

// Cyclomatic Complexity: 1 - Too simple
void test_timer_init(void) {
    timer_init();
    assert(get_timer_ms() == 0);
    printf("✓ test_timer_init\n");
}

// Cyclomatic Complexity: 1 - Too simple, doesn't test boundaries
void test_timer_increment(void) {
    timer_init();
    timer_increment();
    assert(get_timer_ms() == 1);
    // MISSING: No overflow test (65535 -> 0)
    // MISSING: No boundary tests
    printf("✓ test_timer_increment\n");
}

// Cyclomatic Complexity: 1 - Too simple
void test_timeout(void) {
    set_timer_ms(100);
    assert(is_timeout(50, 50) == true);  // Happy path only
    // MISSING: Overflow scenario (timer wraps from 65535 to 5)
    // MISSING: Boundary: 0, 65535
    printf("✓ test_timeout\n");
}

// MISSING: No test for scale_value() boundaries (0, 255, 200)
// MISSING: No test for validate_range() edge cases

int main(void) {
    printf("\n=== Running Bad Tests (Should FAIL) ===\n");

    test_timer_init();
    test_timer_increment();
    test_timeout();

    printf("\n✓ All tests passed (but insufficient coverage)!\n");
    printf("This test file has:\n");
    printf("  - Low complexity (~3) << source complexity (~10)\n");
    printf("  - Poor boundary coverage (misses 0, 255, 65535)\n");
    printf("  - Would get 100%% line coverage but MISS overflow bug!\n");
    printf("  - Should FAIL test-complexity checks\n\n");

    return 0;
}

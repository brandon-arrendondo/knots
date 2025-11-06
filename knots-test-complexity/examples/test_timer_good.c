// test_timer_good.c - GOOD TEST with sufficient complexity and boundary coverage
//
// Expected Result: PASS
// - Test Complexity: ~12 (> 70% of source complexity ~10)
// - Boundary Coverage: ~90% (tests 0, 255, 65535, overflow scenarios)

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

// Cyclomatic Complexity: 1
void test_timer_init(void) {
    timer_init();
    assert(get_timer_ms() == 0);
    printf("✓ test_timer_init\n");
}

// Cyclomatic Complexity: 5
void test_timer_increment(void) {
    timer_init();

    // Test normal increment
    timer_increment();
    assert(get_timer_ms() == 1);

    // Test multiple increments with loop
    for (int i = 0; i < 10; i++) {
        timer_increment();
        if (i == 5) {
            assert(get_timer_ms() == 7);  // Check midway
        }
    }
    assert(get_timer_ms() == 11);

    // Test from different starting values
    set_timer_ms(1000);
    timer_increment();
    assert(get_timer_ms() == 1001);

    // BOUNDARY OVERFLOW: Set timer to -1 (wraps to 65535), then increment
    set_timer_ms(-1);  // -1 wraps to 65535
    assert(get_timer_ms() == 65535);
    timer_increment();
    assert(get_timer_ms() == 0);  // Overflow to 0

    printf("✓ test_timer_increment\n");
}

// Cyclomatic Complexity: 3 - Tests BOUNDARY: uint16_t overflow
void test_timer_overflow(void) {
    // CRITICAL: Test overflow scenario
    set_timer_ms(65535);  // Boundary: MAX
    timer_increment();
    assert(get_timer_ms() == 0);  // Wraps to 0

    // Test near-boundary
    set_timer_ms(65534);  // Boundary: MAX-1
    timer_increment();
    assert(get_timer_ms() == 65535);

    printf("✓ test_timer_overflow\n");
}

// Cyclomatic Complexity: 6 - Tests timeout with boundaries INCLUDING OVERFLOW
void test_timeout_boundaries(void) {
    // Boundary: 0 (MIN)
    set_timer_ms(0);
    assert(is_timeout(0, 0) == true);
    assert(is_timeout(0, 1) == false);

    // Test normal timeout
    set_timer_ms(100);
    assert(is_timeout(50, 50) == true);
    assert(is_timeout(50, 51) == false);

    // CRITICAL: Test timeout across overflow
    set_timer_ms(5);  // Timer wrapped around from 65535
    assert(is_timeout(65530, 10) == true);  // elapsed = 11ms, duration = 10ms, should timeout
    assert(is_timeout(65530, 100) == false);  // elapsed = 11ms, duration = 100ms, should NOT timeout

    // Boundary: 65535 (MAX)
    set_timer_ms(65535);
    assert(is_timeout(65500, 35) == true);

    // BOUNDARY OVERFLOW: Test with start_ms = 65536 (wraps to 0)
    set_timer_ms(100);
    assert(is_timeout(65536, 100) == true);  // 65536 wraps to 0, elapsed = 100

    // BOUNDARY OVERFLOW: Test with start_ms = -1 (wraps to 65535)
    set_timer_ms(100);
    assert(is_timeout(-1, 200) == false);  // -1 wraps to 65535, elapsed = 101

    // BOUNDARY OVERFLOW: Test with duration_ms = -1 (wraps to 65535)
    set_timer_ms(100);
    assert(is_timeout(0, -1) == false);  // -1 wraps to 65535, elapsed = 100, 100 < 65535, no timeout

    // BOUNDARY OVERFLOW: Test with duration_ms = 65536 (wraps to 0)
    set_timer_ms(100);
    assert(is_timeout(0, 65536) == true);  // 65536 wraps to 0, elapsed = 100, 100 >= 0

    printf("✓ test_timeout_boundaries\n");
}

// Cyclomatic Complexity: 6 - Tests uint8_t boundaries INCLUDING OVERFLOW
void test_scale_value_boundaries(void) {
    // Boundary: 0 (MIN)
    assert(scale_value(0) == 0);

    // Boundary: 255 (MAX for uint8_t)
    assert(scale_value(255) == 255);

    // BOUNDARY OVERFLOW: 256 wraps to 0
    assert(scale_value(256) == 0);

    // BOUNDARY OVERFLOW: -1 wraps to 255
    assert(scale_value(-1) == 255);

    // Boundary: 200 threshold
    assert(scale_value(200) == 255);
    assert(scale_value(199) < 255);

    // Test various values
    assert(scale_value(1) > 0);
    assert(scale_value(100) > 0);

    // Edge: 254 (MAX-1)
    assert(scale_value(254) == 255);

    printf("✓ test_scale_value_boundaries\n");
}

// Cyclomatic Complexity: 9 - Tests validation logic with overflow cases
void test_validate_range(void) {
    // Test invalid range (min > max)
    assert(validate_range(50, 100, 50) == -1);

    // Boundary: 0 (MIN for uint8_t)
    assert(validate_range(0, 0, 100) == 1);

    // Boundary: 255 (MAX for uint8_t)
    assert(validate_range(255, 0, 255) == 1);

    // BOUNDARY OVERFLOW: value = -1 wraps to 255
    assert(validate_range(-1, 0, 200) == 0);  // 255 > 200, should be out of range

    // BOUNDARY OVERFLOW: value = 256 wraps to 0
    assert(validate_range(256, 10, 100) == 0);  // 0 < 10, should be out of range

    // BOUNDARY OVERFLOW: min = -1 wraps to 255
    assert(validate_range(100, -1, 200) == -1);  // min=255 > max=200 → invalid range

    // BOUNDARY OVERFLOW: max = -1 wraps to 255
    assert(validate_range(100, 0, -1) == 1);  // max=255, value=100 is in range

    // Test below min
    assert(validate_range(0, 10, 100) == 0);

    // Test above max
    assert(validate_range(255, 0, 200) == 0);

    // Test within range - multiple cases
    for (int i = 10; i <= 100; i += 10) {
        int result = validate_range(i, 10, 100);
        if (i >= 10 && i <= 100) {
            assert(result == 1);
        }
    }

    // Test edge of boundaries
    assert(validate_range(9, 10, 100) == 0);   // Just below min
    assert(validate_range(10, 10, 100) == 1);  // At min
    assert(validate_range(100, 10, 100) == 1); // At max
    assert(validate_range(101, 10, 100) == 0); // Just above max

    printf("✓ test_validate_range\n");
}

int main(void) {
    printf("\n=== Running Good Tests (Should PASS) ===\n");

    test_timer_init();
    test_timer_increment();
    test_timer_overflow();
    test_timeout_boundaries();
    test_scale_value_boundaries();
    test_validate_range();

    printf("\n✓ All tests passed!\n");
    printf("This test file has:\n");
    printf("  - High complexity (~12) matching source complexity (~10)\n");
    printf("  - Good boundary coverage (0, 255, 65535, overflow cases)\n");
    printf("  - Should PASS test-complexity checks\n\n");

    return 0;
}

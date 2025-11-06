// test_sensor_boundaries.c - Demonstrates thorough boundary value testing
//
// Expected Result: PASS
// - Tests all boundaries: 0, 100, 255, 65535
// - Tests off-by-one: MIN-1, MIN, MAX, MAX+1
// - Tests threshold values: 79, 80, 81

#include <stdint.h>
#include <stdbool.h>
#include <assert.h>
#include <stdio.h>
#include <stddef.h>

// Forward declarations
uint8_t read_sensor(uint8_t raw_value);
bool is_overheating(uint8_t temperature);
int process_reading(uint16_t reading, uint8_t *output);

// Cyclomatic Complexity: 7 - Tests all uint8_t boundaries INCLUDING OVERFLOW
void test_sensor_boundaries(void) {
    // Boundary: 0 (MIN for uint8_t)
    assert(read_sensor(0) == 0);

    // Boundary: 255 (MAX for uint8_t)
    assert(read_sensor(255) == 100);  // Clamped to SENSOR_MAX

    // BOUNDARY OVERFLOW: 256 wraps to 0
    assert(read_sensor(256) == 0);

    // BOUNDARY OVERFLOW: -1 wraps to 255
    assert(read_sensor(-1) == 100);  // 255 clamped to SENSOR_MAX

    // Boundary: SENSOR_MAX (100)
    assert(read_sensor(100) == 100);

    // Boundary: SENSOR_MAX + 1 (off-by-one)
    assert(read_sensor(101) == 100);  // Should clamp

    // Boundary: SENSOR_MAX - 1 (off-by-one)
    assert(read_sensor(99) == 99);  // Should not clamp
    assert(read_sensor(98) == 98);   // Another near-boundary

    // Test with loop for multiple values
    for (int i = 50; i < 60; i++) {
        uint8_t result = read_sensor(i);
        if (i <= 100) {
            assert(result == i);
        }
    }

    printf("✓ test_sensor_boundaries\n");
}

// Cyclomatic Complexity: 4 - Tests threshold boundaries
void test_overheating_threshold(void) {
    // Boundary: TEMP_THRESHOLD - 1 (below threshold)
    assert(is_overheating(79) == false);

    // Boundary: TEMP_THRESHOLD (exactly at threshold)
    assert(is_overheating(80) == true);

    // Boundary: TEMP_THRESHOLD + 1 (above threshold)
    assert(is_overheating(81) == true);

    // Boundary: 0 (MIN)
    assert(is_overheating(0) == false);

    // Boundary: 255 (MAX)
    assert(is_overheating(255) == true);

    printf("✓ test_overheating_threshold\n");
}

// Cyclomatic Complexity: 8 - Tests uint16_t boundaries and error cases INCLUDING OVERFLOW
void test_process_reading_boundaries(void) {
    uint8_t output;

    // Boundary: NULL pointer check
    assert(process_reading(100, NULL) == -1);

    // Boundary: 0 (MIN for uint16_t)
    assert(process_reading(0, &output) == 0);
    assert(output == 0);

    // Boundary: 1 (MIN + 1)
    assert(process_reading(1, &output) == 0);
    // Note: (1 * 255) / 1000 = 0 in integer math
    assert(output == 0);

    // BOUNDARY OVERFLOW: -1 wraps to 65535
    assert(process_reading(-1, &output) == 0);
    assert(output == 255);

    // BOUNDARY OVERFLOW: 65536 wraps to 0
    assert(process_reading(65536, &output) == 0);
    assert(output == 0);

    // Boundary: 1000 (threshold)
    assert(process_reading(1000, &output) == 0);
    // Note: 1000 is NOT > 1000, so output = (1000 * 255) / 1000 = 255
    assert(output == 255);

    // Boundary: 999 (threshold - 1)
    assert(process_reading(999, &output) == 0);
    assert(output < 255);

    // Boundary: 998 (threshold - 2)
    assert(process_reading(998, &output) == 0);
    assert(output < 255);

    // Boundary: 1001 (threshold + 1)
    assert(process_reading(1001, &output) == 0);
    assert(output == 255);

    // Boundary: 65535 (MAX for uint16_t)
    assert(process_reading(65535, &output) == 0);
    assert(output == 255);

    // Boundary: 65534 (MAX - 1)
    assert(process_reading(65534, &output) == 0);
    assert(output == 255);

    // Boundary: 65533 (MAX - 2)
    assert(process_reading(65533, &output) == 0);
    assert(output == 255);

    // Test with loop for range of values
    for (int i = 500; i < 510; i++) {
        int result = process_reading(i, &output);
        if (result == 0) {
            assert(output > 0 && output < 255);
        }
    }

    printf("✓ test_process_reading_boundaries\n");
}

int main(void) {
    printf("\n=== Boundary Value Testing Examples ===\n");

    test_sensor_boundaries();
    test_overheating_threshold();
    test_process_reading_boundaries();

    printf("\n✓ All boundary tests passed!\n");
    printf("This demonstrates thorough boundary testing:\n");
    printf("  - Tests 0, MAX values for all integer types\n");
    printf("  - Tests MIN-1, MIN, MAX, MAX+1 (off-by-one)\n");
    printf("  - Tests threshold values: value-1, value, value+1\n");
    printf("  - Should PASS boundary coverage checks\n\n");

    return 0;
}

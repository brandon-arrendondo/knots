// timer.c - Source file with timer functionality
// Demonstrates overflow bugs and boundary conditions
//
// Cyclomatic Complexity: ~10
// Boundaries: uint16_t (0, 65535), uint8_t (0, 255)

#include <stdint.h>
#include <stdbool.h>

// Global timer - overflows at 65535
static uint16_t timer_ms = 0;

// Cyclomatic Complexity: 1
void timer_init(void) {
    timer_ms = 0;
}

// Cyclomatic Complexity: 1
void timer_increment(void) {
    timer_ms++;  // CRITICAL: Overflows at 65535!
}

// Cyclomatic Complexity: 2
bool is_timeout(uint16_t start_ms, uint16_t duration_ms) {
    uint16_t elapsed = timer_ms - start_ms;
    if (elapsed >= duration_ms) {
        return true;
    }
    return false;
}

// Cyclomatic Complexity: 3
uint8_t scale_value(uint8_t input) {
    if (input == 0) {
        return 0;
    } else if (input >= 200) {
        return 255;  // Max output
    } else {
        return (input * 255) / 200;
    }
}

// Cyclomatic Complexity: 4
int validate_range(uint8_t value, uint8_t min, uint8_t max) {
    if (min > max) {
        return -1;  // Invalid range
    }
    if (value < min) {
        return 0;  // Below range
    }
    if (value > max) {
        return 0;  // Above range
    }
    return 1;  // Within range
}

uint16_t get_timer_ms(void) {
    return timer_ms;
}

void set_timer_ms(uint16_t value) {
    timer_ms = value;
}

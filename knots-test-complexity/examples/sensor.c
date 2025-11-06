// sensor.c - Demonstrates boundary value testing
// Focus on integer type boundaries and range checks

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#define SENSOR_MIN 0
#define SENSOR_MAX 100
#define TEMP_THRESHOLD 80

// Cyclomatic Complexity: 3
uint8_t read_sensor(uint8_t raw_value) {
    if (raw_value > SENSOR_MAX) {
        return SENSOR_MAX;  // Clamp to max
    } else if (raw_value < SENSOR_MIN) {
        return SENSOR_MIN;  // Clamp to min
    }
    return raw_value;
}

// Cyclomatic Complexity: 2
bool is_overheating(uint8_t temperature) {
    if (temperature >= TEMP_THRESHOLD) {
        return true;
    }
    return false;
}

// Cyclomatic Complexity: 5
int process_reading(uint16_t reading, uint8_t *output) {
    if (output == NULL) {
        return -1;  // Error: null pointer
    }

    if (reading == 0) {
        *output = 0;
        return 0;  // Zero reading
    }

    if (reading > 65535) {  // Won't happen with uint16_t, but shows intent
        *output = 255;
        return 1;  // Overflow
    }

    if (reading > 1000) {
        *output = 255;  // Max output
    } else {
        *output = (reading * 255) / 1000;
    }

    return 0;  // Success
}

// C program demonstrating deep nesting and its impact on complexity

#include <stdio.h>

// Deeply nested function - shows difference between McCabe and Cognitive
// McCabe will be moderate, Cognitive will be very high due to nesting
void deeply_nested(int a, int b, int c, int d) {
    if (a > 0) {
        printf("Level 1\n");
        if (b > 0) {
            printf("Level 2\n");
            if (c > 0) {
                printf("Level 3\n");
                if (d > 0) {
                    printf("Level 4\n");
                    printf("All positive!\n");
                }
            }
        }
    }
}

// Same logic but flattened - lower cognitive complexity
// McCabe will be similar, Cognitive will be lower
void flattened(int a, int b, int c, int d) {
    if (a <= 0) return;
    printf("Level 1\n");

    if (b <= 0) return;
    printf("Level 2\n");

    if (c <= 0) return;
    printf("Level 3\n");

    if (d <= 0) return;
    printf("Level 4\n");

    printf("All positive!\n");
}

// Multiple nested loops
void triple_nested_loop(int n) {
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            for (int k = 0; k < n; k++) {
                if (i == j && j == k) {
                    printf("Found: %d\n", i);
                }
            }
        }
    }
}

int main() {
    deeply_nested(1, 2, 3, 4);
    flattened(1, 2, 3, 4);
    triple_nested_loop(5);
    return 0;
}

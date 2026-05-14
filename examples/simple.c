// Simple C program with low complexity

#include <stdio.h>

// McCabe: 1, Cognitive: 0
void print_hello() {
    printf("Hello, World!\n");
}

// McCabe: 2, Cognitive: 1
int max(int a, int b) {
    if (a > b) {
        return a;
    }
    return b;
}

// McCabe: 1, Cognitive: 0
int add(int a, int b) {
    return a + b;
}

int main() {
    print_hello();
    int result = max(5, 10);
    printf("Max: %d\n", result);
    return 0;
}

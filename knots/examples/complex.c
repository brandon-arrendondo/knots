// C program with higher complexity

#include <stdio.h>

// High complexity function with nested loops and conditions
// McCabe: High, Cognitive: Very High (due to nesting)
void process_matrix(int matrix[][10], int rows, int cols) {
    for (int i = 0; i < rows; i++) {
        for (int j = 0; j < cols; j++) {
            if (matrix[i][j] > 0) {
                if (matrix[i][j] % 2 == 0) {
                    printf("Even positive: %d\n", matrix[i][j]);
                } else {
                    printf("Odd positive: %d\n", matrix[i][j]);
                }
            } else if (matrix[i][j] < 0) {
                printf("Negative: %d\n", matrix[i][j]);
            } else {
                printf("Zero\n");
            }
        }
    }
}

// Function with switch statement
// McCabe: High (multiple cases), Cognitive: Moderate
void process_command(char cmd) {
    switch (cmd) {
        case 'a':
            printf("Add\n");
            break;
        case 's':
            printf("Subtract\n");
            break;
        case 'm':
            printf("Multiply\n");
            break;
        case 'd':
            printf("Divide\n");
            break;
        default:
            printf("Unknown command\n");
            break;
    }
}

// Function with multiple conditions
// McCabe: High (many logical operators), Cognitive: Moderate
int validate_input(int x, int y, int z) {
    if (x > 0 && y > 0 && z > 0) {
        if (x < 100 || y < 100 || z < 100) {
            return 1;
        }
    }
    return 0;
}

// Function with complex control flow
// McCabe: Very High, Cognitive: Very High
int search_with_conditions(int arr[], int size, int target) {
    int found = 0;

    for (int i = 0; i < size; i++) {
        if (arr[i] == target) {
            found = 1;
            break;
        }

        if (arr[i] > target) {
            // Early exit if array is sorted
            if (i > 0 && arr[i-1] < target) {
                break;
            }
        }

        // Additional complexity
        if (arr[i] < 0) {
            continue;
        }
    }

    return found;
}

int main() {
    int matrix[3][10] = {{1, -2, 3}, {4, 0, -6}, {7, 8, 9}};
    process_matrix(matrix, 3, 3);

    process_command('a');

    int result = validate_input(5, 10, 15);
    printf("Valid: %d\n", result);

    int arr[] = {1, 3, 5, 7, 9};
    int found = search_with_conditions(arr, 5, 5);
    printf("Found: %d\n", found);

    return 0;
}

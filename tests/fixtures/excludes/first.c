int drop_simple(void) { return 1; }

int drop_complex(int value) {
    if (value > 0) return 1;
    return 0;
}

int keep_complex(int value) {
    if (value < 0) return 1;
    return 0;
}

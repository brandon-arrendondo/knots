// Header carrying nontrivial logic on purpose — this fixture exists to
// verify that --recursive counts complexity contributed by headers
// (see HEADER_INCLUSION.md), not just .c/.cpp translation units.
#ifndef HEADER_LOGIC_H
#define HEADER_LOGIC_H

static inline int classify_value(int value) {
    if (value > 100) {
        if (value % 2 == 0) {
            return 3;
        } else {
            return 2;
        }
    } else if (value > 0) {
        return 1;
    } else if (value < 0) {
        return -1;
    }
    return 0;
}

#endif

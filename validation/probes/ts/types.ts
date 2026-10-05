// probe: type syntax adds no decisions
// source: McCabe 1976 and Cognitive 1.7 count control flow; annotations, `as`, the non-null assertion `x!`,
//         generics and an optional parameter without a default (`b?:`) have none (TypeScript erases them).
//         One if: McCabe 2, Cognitive 1.
// expect f mccabe=2 cognitive=1
function f<T extends { length: number }>(a: number, b?: T): number {
    if (a > 0) {
        return a as number;
    }
    return b!.length;
}

// probe: overload signatures, declarations and abstract members are not functions
// source: TypeScript Handbook, "Function Overloads": the signatures have no body and only the implementation
//         runs; an abstract method, an interface member and a `declare function` have no body either. parse's
//         implementation has one if: McCabe 2, Cognitive 1. The signatures have no flow graph to score:
//         knots reports parse once and nothing for external, outline or label (checked by
//         typescript_signatures_are_not_functions in src/lib.rs).
// expect parse mccabe=2 cognitive=1
// expect area mccabe=1 cognitive=0
function parse(x: string): number;
function parse(x: number): number;
function parse(x: string | number): number {
    if (typeof x === "string") {
        return parseInt(x, 10);
    }
    return x;
}

declare function external(x: number): number;

interface Shape {
    outline(): number;
}

abstract class Base {
    abstract label(): string;
}

class Square {
    constructor(private side: number) {}
    area(): number {
        return this.side * this.side;
    }
}

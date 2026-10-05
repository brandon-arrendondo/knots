// probe: nullish coalescing and optional chaining
// source: ECMA-262, "Binary Logical Operators" and "Optional Chains": `a ?? b` evaluates b only when a is null or
//         undefined, and `a?.x` evaluates the access only when a is not, so each is a two-way decision
//         (McCabe 1976; Myers 1977 for a decision inside an expression). Cognitive 1.7 "Ignore shorthand":
//         "Cognitive Complexity ignores null-coalescing operators", with `a?.myObj` as its example.
// expect g mccabe=4 cognitive=0
// expect h mccabe=4 cognitive=1
function g(a, b) {
    return a?.x ?? b?.y;
}

function h(a, b, c) {
    return (a && b) ?? c ?? 0;
}

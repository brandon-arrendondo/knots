// probe: logical assignment
// source: ECMA-262, "Assignment Operators": `a ||= b`, `a &&= b` and `a ??= b` evaluate and assign b only when
//         the test on a passes, so each is a two-way decision (McCabe 1976). Cognitive 1.7 ignores
//         null-coalescing operators, so `??=` adds nothing; it is silent on `||=` and `&&=`, so f's cognitive
//         value is not asserted.
// expect f mccabe=4
// expect g mccabe=2 cognitive=0
function f(o) {
    o.a ||= 1;
    o.b &&= 2;
    o.c ??= 3;
    return o;
}

function g(o) {
    o.c ??= 3;
    return o;
}

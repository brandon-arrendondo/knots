// probe: default parameter values
// source: ECMA-262, "FunctionDeclarationInstantiation" / "IteratorBindingInitialization": an initializer is
//         evaluated only when the argument is undefined, so each default is a two-way decision (McCabe 1976).
//         Cognitive 1.7 lists no increment for it.
// expect f mccabe=3 cognitive=0
function f(a = 1, b = a + 1) {
    return a + b;
}

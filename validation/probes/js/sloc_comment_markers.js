// probe: comment markers inside a template literal and a regular expression
// source: Park 1992 (physical lines holding code); ECMA-262, "Template Literal Lexical Components" and "Regular
//         Expression Literals": `//` and `/*` inside them are not comments. Every line of f holds code: 7.
// expect f sloc=7
function f(s) {
    const t = `
// not a comment
/* nor this`;
    const r = /\/\*/;
    return r.test(s) ? t : "";
}

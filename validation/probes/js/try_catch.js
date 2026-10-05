// probe: try / catch / finally
// source: McCabe 1976 (a handler is a decision: control reaches it or not; finally always runs);
//         Cognitive 1.7 B1-B3 (catch +1 and nests: the if inside it +2; try and finally add nothing)
// expect f mccabe=3 cognitive=3
function f(s) {
    try {
        return JSON.parse(s);
    } catch (e) {
        if (e instanceof SyntaxError) {
            return null;
        }
        throw e;
    } finally {
        s = null;
    }
}

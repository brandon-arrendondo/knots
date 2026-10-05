// probe: exception handlers
// source: McCabe 1976 (each handler is a decision: control reaches it or not); Cognitive 1.7 B1-B3 (catch +1 and
//         nests: the if inside the first handler +2; try adds nothing)
// expect f mccabe=4 cognitive=4
int f(int x)
{
    try {
        x = g(x);
    } catch (const E &e) {
        if (x > 0)
            return 1;
    } catch (...) {
        return 2;
    }
    return x;
}

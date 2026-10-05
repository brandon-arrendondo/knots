// probe: exception handlers
// source: McCabe 1976 (each handler is a decision: control reaches it or not; finally always runs);
//         Cognitive 1.7 B1-B3 (catch +1 and nests: the if inside the first handler +2; try and finally add nothing)
// expect f mccabe=4 cognitive=4
class TryCatch {
    int f(int x) {
        try {
            x = g(x);
        } catch (IllegalStateException e) {
            if (x > 0) {
                return 1;
            }
        } catch (RuntimeException e) {
            return 2;
        } finally {
            x = 0;
        }
        return x;
    }
}

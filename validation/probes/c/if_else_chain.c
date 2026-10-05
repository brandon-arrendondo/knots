// probe: if / else if / else
// source: McCabe 1976 (if and else-if are decisions; else is not); Cognitive 1.7 B1 (if, else if, else each +1)
// expect f mccabe=3 cognitive=3
int f(int x)
{
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
}

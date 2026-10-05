// probe: plain break, continue and early return add nothing
// source: McCabe 1976 (unconditional transfers are not decisions); Cognitive 1.7 B1 (only break/continue to a label increment)
//         for +1, three nested ifs +2 each
// expect f mccabe=5 cognitive=7
int f(int n)
{
    for (int i = 0; i < n; i++) {
        if (i == 3)
            continue;
        if (i == 7)
            break;
        if (i == n - 1)
            return i;
    }
    return -1;
}

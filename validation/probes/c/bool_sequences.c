// probe: boolean operators in conditions
// source: Cognitive 1.7 section "Sequences of logical operators" (its two worked examples, scores 4 and 3);
//         McCabe extended to conditions (Myers 1977; as pmccabe and lizard count): each && and || is a decision
// expect mixed mccabe=7 cognitive=4
// expect negated mccabe=4 cognitive=3
int mixed(int a, int b, int c, int d, int e, int g)
{
    if (a && b && c || d || e && g)
        return 1;
    return 0;
}

int negated(int a, int b, int c)
{
    if (a && !(b && c))
        return 1;
    return 0;
}

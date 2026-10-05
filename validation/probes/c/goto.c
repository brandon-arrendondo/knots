// probe: goto is not a decision, but Cognitive Complexity charges it
// source: McCabe 1976 (unconditional transfer); Cognitive 1.7 B1 (goto LABEL +1); pmccabe gives 2
// expect f mccabe=2 cognitive=2
int f(int x)
{
    if (x)
        goto out;
    x++;
out:
    return x;
}

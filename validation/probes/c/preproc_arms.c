// probe: decisions inside #ifdef arms
// source: McCabe 1976 defines v(G) on one program's control-flow graph; each preprocessor configuration is a
//         different program, and in either configuration this function has one if. Same for Cognitive Complexity.
// expect f mccabe=2 cognitive=1
int f(int x)
{
#ifdef FEATURE
    if (x > 1)
        x = 1;
#else
    if (x < 0)
        x = 0;
#endif
    return x;
}

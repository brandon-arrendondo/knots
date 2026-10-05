// probe: comment markers the parser reads as comments, and ones it reads as string contents
// source: Park 1992, CMU/SEI-92-TR-20: a physical line counts when it holds code; blank lines and lines holding only
//         comments don't, and a line with code and a comment is a code line. Whether `/*` starts a comment is
//         fixed by the language (ISO C 6.4.9: not inside a string literal). Of f's 9 lines, the line of two comments and the
//         first line of the block comment don't count: 7.
// expect f sloc=7
int f(int x)
{
    const char *open = "/*";
    /* else */ /* FALLTHROUGH */
    x += open[0];
    /* a block comment
       spanning lines */ x++;
    return x;
}

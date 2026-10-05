// probe: switch, the whitepaper's getWords example translated to C
// source: Cognitive 1.7: "Under Cyclomatic Complexity ... each case in the switch causes an increment";
//         getWords is given Cyclomatic Complexity 4 and Cognitive Complexity 1
// expect getWords mccabe=4 cognitive=1 mccabe.modified=2
const char *getWords(int number)
{
    switch (number) {
    case 1:
        return "one";
    case 2:
        return "a couple";
    case 3:
        return "a few";
    default:
        return "lots";
    }
}

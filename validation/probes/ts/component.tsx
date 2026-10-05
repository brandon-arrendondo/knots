// probe: decisions inside JSX
// source: McCabe 1976 (each ?: and && is a two-way decision; Myers 1977 for a decision inside an expression);
//         Cognitive 1.7 B1 (ternary +1 at nesting 0; one && sequence +1).
// expect Badge mccabe=3 cognitive=2
function Badge(props: { count: number; urgent: boolean }) {
    return (
        <span>
            {props.count > 0 && <b>{props.count}</b>}
            {props.urgent ? <i>!</i> : null}
        </span>
    );
}

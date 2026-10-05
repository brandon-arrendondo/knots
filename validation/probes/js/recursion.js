// probe: recursion, direct and indirect, through functions and methods
// source: Cognitive 1.7 section "Recursion": +1 for each method in a recursion cycle, direct or indirect, once
//         per method. ECMA-262 fixes which calls reach which function: a bare name resolves through the
//         enclosing scopes ("ResolveBinding"), and `this` inside an arrow function is the enclosing method's
//         `this`, while inside a `function` expression it is that function's own ("OrdinaryCallBindThis").
// expect fact mccabe=2 cognitive=2
// expect isEven mccabe=2 cognitive=2
// expect isOdd mccabe=2 cognitive=2
// expect outer mccabe=1 cognitive=0
// expect size mccabe=2 cognitive=2
// expect countAll mccabe=1 cognitive=1
// expect visitAll mccabe=1 cognitive=0
// expect #walk mccabe=2 cognitive=2
// expect depth mccabe=2 cognitive=2
function fact(n) {
    if (n <= 1) {
        return 1;
    }
    return n * fact(n - 1);
}

function isEven(n) {
    if (n === 0) {
        return true;
    }
    return isOdd(n - 1);
}

function isOdd(n) {
    if (n === 0) {
        return false;
    }
    return isEven(n - 1);
}

// outer's `step(n)` is its own nested step, which doesn't recurse; the top-level
// step calls outer, but nothing calls the top-level step from outer.
function outer(n) {
    function step(m) {
        return m + 1;
    }
    return step(n);
}

function step(n) {
    return outer(n);
}

class Tree {
    size(node) {
        let n = 1;
        for (const c of node.children) {
            n += this.size(c);
        }
        return n;
    }

    countAll(nodes) {
        return nodes.map((n) => this.countAll(n.children)).length;
    }

    // `this` inside the function expression is not the Tree.
    visitAll(items) {
        items.forEach(function (x) {
            this.visitAll(x);
        });
    }

    #walk(n) {
        if (n) {
            this.#walk(n.next);
        }
    }
}

class Node {
    depth() {
        let d = 0;
        for (const c of this.children) {
            d = Math.max(d, c.depth());
        }
        return d + 1;
    }
}

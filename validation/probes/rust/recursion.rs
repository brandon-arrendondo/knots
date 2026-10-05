// probe: recursion, direct and indirect, through free functions, associated functions and methods
// source: Cognitive 1.7 section "Recursion": +1 for each method in a recursion cycle, direct or indirect, once
//         per method
// expect fact mccabe=2 cognitive=2
// expect is_even mccabe=2 cognitive=2
// expect is_odd mccabe=2 cognitive=2
// expect leaf mccabe=1 cognitive=0
// expect count mccabe=2 cognitive=2
// expect size mccabe=3 cognitive=3
// expect depth mccabe=2 cognitive=2
fn fact(n: u64) -> u64 {
    if n <= 1 {
        return 1;
    }
    n * fact(n - 1)
}

fn is_even(n: u32) -> bool {
    if n == 0 {
        return true;
    }
    is_odd(n - 1)
}

fn is_odd(n: u32) -> bool {
    if n == 0 {
        return false;
    }
    is_even(n - 1)
}

fn leaf(n: u32) -> bool {
    is_even(n)
}

struct Tree {
    children: Vec<Tree>,
}

impl Tree {
    fn count(t: &Tree) -> usize {
        if t.children.is_empty() {
            return 1;
        }
        1 + t.children.iter().map(Self::count).sum::<usize>()
    }

    fn size(t: &Tree) -> usize {
        if t.children.is_empty() {
            return 1;
        }
        let mut n = 1;
        for c in &t.children {
            n += Self::size(c);
        }
        n
    }

    fn depth(&self) -> usize {
        if self.children.is_empty() {
            return 0;
        }
        1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
    }
}

// Expected Ce 1: a.rs. A `src/bin` file is its own crate root.
use fixture_crate::a::Alpha;

fn main() {
    let _ = Alpha;
}

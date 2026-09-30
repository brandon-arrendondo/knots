// Expected Ce 1: b.rs. An alias is resolved by its path.
use fixture_crate::b::Beta as B;

pub fn run() {
    let _ = B;
}

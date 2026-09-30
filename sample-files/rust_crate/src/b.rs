// Expected Ce 2: lib.rs (an item defined at the crate root) and
// net/tcp.rs (through `super`).
use super::net::tcp::connect;
use crate::Root;

pub struct Beta;

pub fn helper(_b: Beta, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
    connect(Root);
    Ok(())
}

// Expected Ce 3: b.rs (a glob through `super::super`), a.rs and
// net/mod.rs (a nested group).
use super::super::b::*;
use crate::{a::Alpha, net};

pub struct Socket;

pub fn connect(_r: crate::Root) {
    let _ = (Alpha, Beta);
    let _ = net::open;
}

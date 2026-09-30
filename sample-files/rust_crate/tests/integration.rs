// Expected Ce 2: tests/common/mod.rs and net/mod.rs. An external crate
// is not an edge.
mod common;

use common::setup;
use fixture_crate::net;
use serde::Serialize;

#[test]
fn opens() {
    setup();
    let _ = net::open();
}

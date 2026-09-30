// Expected Ce 3: cli.rs (an edition-2018 relative path to a child
// module), net/tcp.rs and lib.rs (the library reached by crate name).
mod cli;

use cli::run;
use fixture_crate::net::tcp;
use fixture_crate::Root;

fn main() {
    tcp::connect(Root);
    run();
}

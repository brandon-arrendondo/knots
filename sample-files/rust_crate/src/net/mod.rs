// Expected Ce 2: net/tcp.rs (through `self`) and a.rs.
pub mod tcp;

use self::tcp::Socket;
use crate::a;

pub fn open() -> Socket {
    let _ = a::Alpha;
    Socket
}

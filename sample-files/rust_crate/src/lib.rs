// Expected Ce 1: a.rs. `mod` declarations define the tree and are not edges.
pub mod a;
pub mod b;
pub mod net;

pub use a::Alpha;

pub struct Root;

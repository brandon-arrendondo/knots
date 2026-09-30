// Expected Ce 1: b.rs (a group names it once); std is outside the corpus,
// and `super::*` in the inline test module is this file.
use crate::b::{helper, Beta};
use std::fmt;

pub struct Alpha;

pub fn show(f: &mut fmt::Formatter) -> fmt::Result {
    helper(Beta, f)
}

#[cfg(test)]
mod tests {
    use super::*;
}

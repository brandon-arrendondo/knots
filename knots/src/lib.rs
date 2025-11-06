// knots library - shared complexity calculation functions

pub mod complexity;

// Re-export complexity functions for use by workspace members
pub use complexity::{calculate_mccabe_complexity, calculate_cognitive_complexity};

// Re-export tree-sitter for convenience
pub use tree_sitter;

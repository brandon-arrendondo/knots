// knots library - shared complexity calculation functions

pub mod complexity;

// Re-export complexity functions for use by workspace members
pub use complexity::{
    calculate_aicp, calculate_aird, calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_state_coupling,
};

// Re-export tree-sitter for convenience
pub use tree_sitter;
pub use tree_sitter_c;
pub use tree_sitter_cpp;
pub use tree_sitter_javascript;
pub use tree_sitter_python;
pub use tree_sitter_rust;

/// All source file extensions recognized by knots, grouped by language.
/// This is the single source of truth — add new language extensions here.
/// `.h` is intentionally excluded (treated as C by language_for_file but not
/// scanned during recursive discovery, since headers often contain vendor/inline code).
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // C
    "c",
    // C++
    "cpp", "cc", "cxx", "hpp", "hxx",
    // Rust
    "rs",
    // Python
    "py",
    // JavaScript
    "js", "mjs", "cjs",
];

/// Returns the appropriate tree-sitter language for a file based on extension.
/// `.h` defaults to C; C++ headers should use `.hpp`/`.hxx`.
pub fn language_for_file(path: &std::path::Path) -> tree_sitter::Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => {
            tree_sitter_cpp::language()
        }
        Some("rs") => tree_sitter_rust::language(),
        Some("py") => tree_sitter_python::language(),
        Some("js") | Some("mjs") | Some("cjs") => tree_sitter_javascript::language(),
        _ => tree_sitter_c::language(),
    }
}

/// Returns true if the file extension is supported by knots.
pub fn is_source_extension(ext: &std::ffi::OsStr) -> bool {
    ext.to_str()
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

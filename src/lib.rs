// knots library - shared complexity calculation functions

pub mod complexity;

// Re-export complexity functions for use by workspace members
pub use complexity::{
    calculate_aicp, calculate_aird, calculate_cognitive_complexity, calculate_mccabe_complexity,
};

// Re-export tree-sitter for convenience
pub use tree_sitter;
pub use tree_sitter_c;
pub use tree_sitter_cpp;
pub use tree_sitter_javascript;
pub use tree_sitter_python;
pub use tree_sitter_rust;

/// C++ source file extensions (not headers)
const CPP_SOURCE_EXTENSIONS: &[&str] = &["cpp", "cc", "cxx"];
/// C++ header extensions
const CPP_HEADER_EXTENSIONS: &[&str] = &["hpp", "hxx"];

/// Returns the appropriate tree-sitter language for a file based on extension.
/// `.h` defaults to C; C++ headers should use `.hpp`/`.hxx`.
/// @brief Select tree-sitter language grammar by file extension
/// @version 4
pub fn language_for_file(path: &std::path::Path) -> tree_sitter::Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext)
            if CPP_SOURCE_EXTENSIONS.contains(&ext) || CPP_HEADER_EXTENSIONS.contains(&ext) =>
        {
            tree_sitter_cpp::language()
        }
        Some("rs") => tree_sitter_rust::language(),
        Some("py") => tree_sitter_python::language(),
        Some("js") | Some("mjs") | Some("cjs") => tree_sitter_javascript::language(),
        _ => tree_sitter_c::language(),
    }
}

/// Returns true if the extension is a C/C++/Rust/Python source file (not a header).
/// @brief Check if file extension is a supported source extension
/// @version 4
pub fn is_source_extension(ext: &std::ffi::OsStr) -> bool {
    match ext.to_str() {
        Some("c") | Some("rs") | Some("py") | Some("js") | Some("mjs") | Some("cjs") => true,
        Some(e) => CPP_SOURCE_EXTENSIONS.contains(&e),
        None => false,
    }
}

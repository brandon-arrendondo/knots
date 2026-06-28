// knots library - shared complexity calculation functions

pub mod complexity;

// Re-export complexity functions for use by workspace members
pub use complexity::{
    calculate_aicp, calculate_aird, calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_sloc_ada, calculate_state_coupling,
};

// Re-export tree-sitter for convenience
pub use tree_sitter;
pub use tree_sitter_ada;
pub use tree_sitter_c;
pub use tree_sitter_cpp;
pub use tree_sitter_c_sharp;
pub use tree_sitter_go;
pub use tree_sitter_kotlin_ng;
pub use tree_sitter_swift;
pub use tree_sitter_java;
pub use tree_sitter_javascript;
pub use tree_sitter_python;
pub use tree_sitter_rust;
pub use tree_sitter_typescript;
pub use tree_sitter_php;

/// A language knots can analyze, with its display name and file extensions.
pub struct LanguageInfo {
    /// Human-facing name, e.g. "C++", "Ada".
    pub name: &'static str,
    /// Extensions scanned during recursive discovery (no leading dot).
    pub extensions: &'static [&'static str],
    /// Extensions parsed only when a file is passed explicitly, never during
    /// recursive discovery (e.g. headers, which often hold vendor/inline code).
    pub explicit_only: &'static [&'static str],
}

/// The single source of truth for language support. Add a new language here;
/// `SUPPORTED_EXTENSIONS`, `--supported-languages`, and (via `invoke
/// sync-languages`) every doc that lists languages all derive from this.
/// Keep one entry per line — `tasks.py` parses this table.
pub const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo { name: "C",          extensions: &["c"],                            explicit_only: &["h"] },
    LanguageInfo { name: "C++",        extensions: &["cpp", "cc", "cxx", "hpp", "hxx"], explicit_only: &[] },
    LanguageInfo { name: "Rust",       extensions: &["rs"],                           explicit_only: &[] },
    LanguageInfo { name: "Python",     extensions: &["py"],                           explicit_only: &[] },
    LanguageInfo { name: "JavaScript", extensions: &["js", "mjs", "cjs", "jsx"],      explicit_only: &[] },
    LanguageInfo { name: "TypeScript", extensions: &["ts", "tsx"],                    explicit_only: &[] },
    LanguageInfo { name: "Ada",        extensions: &["adb", "ada"],                   explicit_only: &["ads"] },
    LanguageInfo { name: "Go",         extensions: &["go"],                           explicit_only: &[] },
    LanguageInfo { name: "Java",       extensions: &["java"],                         explicit_only: &[] },
    LanguageInfo { name: "C#",         extensions: &["cs"],                           explicit_only: &[] },
    LanguageInfo { name: "Kotlin",     extensions: &["kt", "kts"],                    explicit_only: &[] },
    LanguageInfo { name: "Swift",      extensions: &["swift"],                        explicit_only: &[] },
    LanguageInfo { name: "PHP",        extensions: &["php"],                          explicit_only: &[] },
];

/// All source file extensions recognized during recursive discovery, grouped
/// by language. Mirrors the `extensions` of [`LANGUAGES`] (a test enforces it).
/// `.h`/`.ads` are intentionally excluded here — they are `explicit_only`
/// (parsed when passed directly, but skipped by `--recursive`).
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
    "js", "mjs", "cjs", "jsx",
    // TypeScript
    "ts", "tsx",
    // Ada
    "adb", "ada",
    // Go
    "go",
    // Java
    "java",
    // C#
    "cs",
    // Kotlin
    "kt", "kts",
    // Swift
    "swift",
    // PHP
    "php",
];

/// Renders the human-readable `knots --supported-languages` report.
pub fn supported_languages_report() -> String {
    let width = LANGUAGES.iter().map(|l| l.name.len()).max().unwrap_or(0);
    let mut out = String::from("Supported languages:\n");
    for lang in LANGUAGES {
        let exts = lang
            .extensions
            .iter()
            .map(|e| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("  {:<width$}  {exts}", lang.name, width = width));
        if !lang.explicit_only.is_empty() {
            let extra = lang
                .explicit_only
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(" ");
            out.push_str(&format!("  (also {extra} when passed explicitly)"));
        }
        out.push('\n');
    }
    out
}

/// Returns the appropriate tree-sitter language for a file based on extension.
/// `.h` defaults to C; C++ headers should use `.hpp`/`.hxx`.
pub fn language_for_file(path: &std::path::Path) -> tree_sitter::Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("adb") | Some("ada") | Some("ads") => tree_sitter_ada::LANGUAGE.into(),
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => {
            tree_sitter_cpp::LANGUAGE.into()
        }
        Some("rs") => tree_sitter_rust::LANGUAGE.into(),
        Some("py") => tree_sitter_python::LANGUAGE.into(),
        Some("js") | Some("mjs") | Some("cjs") | Some("jsx") => tree_sitter_javascript::LANGUAGE.into(),
        Some("ts") => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Some("tsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Some("go") => tree_sitter_go::LANGUAGE.into(),
        Some("java") => tree_sitter_java::LANGUAGE.into(),
        Some("cs") => tree_sitter_c_sharp::LANGUAGE.into(),
        Some("kt") | Some("kts") => tree_sitter_kotlin_ng::LANGUAGE.into(),
        Some("swift") => tree_sitter_swift::LANGUAGE.into(),
        Some("php") => tree_sitter_php::LANGUAGE_PHP.into(),
        _ => tree_sitter_c::LANGUAGE.into(),
    }
}

/// Returns true if the file extension is supported by knots.
pub fn is_source_extension(ext: &std::ffi::OsStr) -> bool {
    ext.to_str()
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

#[cfg(test)]
mod language_registry_tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// `SUPPORTED_EXTENSIONS` must stay exactly the recursive extensions of
    /// `LANGUAGES` — guards against the two lists drifting apart.
    #[test]
    fn supported_extensions_match_languages_table() {
        let from_table: BTreeSet<&str> =
            LANGUAGES.iter().flat_map(|l| l.extensions.iter().copied()).collect();
        let from_const: BTreeSet<&str> = SUPPORTED_EXTENSIONS.iter().copied().collect();
        assert_eq!(
            from_table, from_const,
            "LANGUAGES.extensions and SUPPORTED_EXTENSIONS disagree — update one to match the other"
        );
    }

    /// Every extension in the table (recursive and explicit-only) must route to
    /// a grammar — i.e. it must not silently fall through to the default C arm,
    /// unless it genuinely belongs to C.
    #[test]
    fn every_table_extension_maps_to_its_grammar() {
        let c_lang: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();
        for lang in LANGUAGES {
            for ext in lang.extensions.iter().chain(lang.explicit_only) {
                let mapped = language_for_file(Path::new(&format!("f.{ext}")));
                if lang.name != "C" {
                    assert_ne!(
                        mapped, c_lang,
                        "extension .{ext} ({}) falls through to the default C grammar in language_for_file",
                        lang.name
                    );
                }
            }
        }
    }
}

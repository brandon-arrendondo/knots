// knots library - shared complexity calculation functions

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use globset::Glob;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Tree, TreeCursor};
pub mod complexity;

// Re-export complexity functions for use by workspace members and for internal use
pub use complexity::{
    calculate_abc_complexity, calculate_aicp, calculate_aird,
    calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_nesting_depth, calculate_return_count, calculate_sloc,
    calculate_sloc_ada, calculate_sloc_fixed_form_fortran, calculate_sloc_fortran, calculate_sloc_python,
    calculate_state_coupling, calculate_test_scoring,
    TestScoringMetric,
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
pub use tree_sitter_fortran;
pub use tree_sitter_fixed_form_fortran;
pub use tree_sitter_scala;
pub use tree_sitter_lua;

/// Which SLOC variant a language uses (drives comment-stripping strategy).
#[derive(Clone, Copy, PartialEq)]
pub enum SlocMode {
    /// `//` and `/* */` — default for C, C++, Rust, JS, TS, Go, Java, …
    Default,
    /// Additionally strips `#`-prefixed comment lines.
    Python,
    /// Strips `--` comment lines.
    Ada,
    /// Strips `!` comment lines (free-form .f90/.f95/…).
    Fortran,
    /// Strips fixed-form comment lines: `*`, `C`, or `c` at column 1, plus `!` anywhere.
    FortranFixed,
    /// Strips `--` comment lines (same prefix as Ada, different grammar).
    Lua,
}

/// A language knots can analyze, with its display name and file extensions.
pub struct LanguageInfo {
    /// Human-facing name, e.g. "C++", "Ada".
    pub name: &'static str,
    /// Extensions scanned during recursive discovery (no leading dot).
    pub extensions: &'static [&'static str],
    /// Extensions parsed only when a file is passed explicitly, never during
    /// recursive discovery (e.g. headers, which often hold vendor/inline code).
    pub explicit_only: &'static [&'static str],
    /// Comment style used when computing SLOC for this language.
    pub sloc_mode: SlocMode,
}

/// The single source of truth for language support. Add a new language here;
/// `SUPPORTED_EXTENSIONS`, `--supported-languages`, and (via `invoke
/// sync-languages`) every doc that lists languages all derive from this.
/// Keep one entry per line — `tasks.py` parses this table.
pub const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo { name: "C",          extensions: &["c"],                            explicit_only: &["h"],             sloc_mode: SlocMode::Default },
    LanguageInfo { name: "C++",        extensions: &["cpp", "cc", "cxx", "hpp", "hxx"], explicit_only: &[],             sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Rust",       extensions: &["rs"],                           explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Python",     extensions: &["py"],                           explicit_only: &[],                sloc_mode: SlocMode::Python },
    LanguageInfo { name: "JavaScript", extensions: &["js", "mjs", "cjs", "jsx"],      explicit_only: &[],               sloc_mode: SlocMode::Default },
    LanguageInfo { name: "TypeScript", extensions: &["ts", "tsx"],                    explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Ada",        extensions: &["adb", "ada"],                   explicit_only: &["ads"],           sloc_mode: SlocMode::Ada },
    LanguageInfo { name: "Go",         extensions: &["go"],                           explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Java",       extensions: &["java"],                         explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "C#",         extensions: &["cs"],                           explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Kotlin",     extensions: &["kt", "kts"],                    explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Swift",      extensions: &["swift"],                        explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "PHP",        extensions: &["php"],                          explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Fortran",    extensions: &["f90", "f95", "f03", "f08", "F90", "F95", "F03", "F08"], explicit_only: &["f", "for", "f77", "F", "FOR", "F77"], sloc_mode: SlocMode::Fortran },
    LanguageInfo { name: "Scala",      extensions: &["scala", "sc"],                  explicit_only: &[],                sloc_mode: SlocMode::Default },
    LanguageInfo { name: "Lua",        extensions: &["lua"],                          explicit_only: &[],                sloc_mode: SlocMode::Lua },
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
    // Fortran (modern free-form; fixed-form .f/.for/.f77 are explicit-only)
    "f90", "f95", "f03", "f08", "F90", "F95", "F03", "F08",
    // Scala
    "scala", "sc",
    // Lua
    "lua",
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
        Some("f90") | Some("f95") | Some("f03") | Some("f08")
        | Some("F90") | Some("F95") | Some("F03") | Some("F08") => tree_sitter_fortran::LANGUAGE.into(),
        Some("f") | Some("for") | Some("f77")
        | Some("F") | Some("FOR") | Some("F77") => tree_sitter_fixed_form_fortran::LANGUAGE.into(),
        Some("scala") | Some("sc") => tree_sitter_scala::LANGUAGE.into(),
        Some("lua") => tree_sitter_lua::LANGUAGE.into(),
        _ => tree_sitter_c::LANGUAGE.into(),
    }
}

/// Returns true if the file extension is supported by knots for recursive discovery.
pub fn is_source_extension(ext: &std::ffi::OsStr) -> bool {
    ext.to_str()
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

fn language_info_for_ext(ext: &str) -> Option<&'static LanguageInfo> {
    LANGUAGES
        .iter()
        .find(|l| l.extensions.contains(&ext) || l.explicit_only.contains(&ext))
}

/// Returns true if knots can parse this extension when the file is passed explicitly.
/// Includes both recursive-discovery extensions and explicit-only ones (e.g. `.f`, `.h`).
pub fn is_parseable_extension(ext: &std::ffi::OsStr) -> bool {
    let Some(e) = ext.to_str() else { return false };
    language_info_for_ext(e).is_some()
}

/// Returns the SLOC comment-stripping mode for the given file path,
/// derived from the `LANGUAGES` table (the single source of truth).
pub fn sloc_mode_for_file(path: &str) -> SlocMode {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        // Fixed-form Fortran uses column-1 *, C, c comments — different from free-form !
        "f" | "for" | "f77" | "F" | "FOR" | "F77" => SlocMode::FortranFixed,
        _ => language_info_for_ext(ext).map_or(SlocMode::Default, |l| l.sloc_mode),
    }
}

/// Filter rules for including/excluding files and functions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterRules {
    /// File path patterns (glob-style, supports negation with !)
    #[serde(default)]
    pub file_patterns: Vec<String>,

    /// Function name patterns (regex)
    #[serde(default)]
    pub function_patterns: Vec<String>,

    /// Minimum complexity threshold (inclusive)
    #[serde(default)]
    pub min_complexity: Option<u32>,

    /// Maximum complexity threshold (inclusive)
    #[serde(default)]
    pub max_complexity: Option<u32>,
}

impl FilterRules {
    /// Load filter rules from a JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read filter file: {}", path.display()))?;
        let rules: FilterRules = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse filter JSON: {}", path.display()))?;
        Ok(rules)
    }

    /// Check if a file path matches the patterns
    pub fn matches_file(&self, file_path: &str) -> bool {
        if self.file_patterns.is_empty() {
            return true;
        }

        let mut included = false;
        let mut excluded = false;

        for pattern in &self.file_patterns {
            if let Some(neg_pattern) = pattern.strip_prefix('!') {
                if glob_match(neg_pattern, file_path) {
                    excluded = true;
                }
            } else {
                if glob_match(pattern, file_path) {
                    included = true;
                }
            }
        }

        if !self.file_patterns.iter().any(|p| !p.starts_with('!')) {
            !excluded
        } else {
            included && !excluded
        }
    }

    /// Check if a function name matches the patterns
    pub fn matches_function(&self, function_name: &str) -> bool {
        if self.function_patterns.is_empty() {
            return true;
        }

        for pattern in &self.function_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(function_name) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if complexity is within bounds
    pub fn matches_complexity(&self, complexity: u32) -> bool {
        if let Some(min) = self.min_complexity {
            if complexity < min {
                return false;
            }
        }
        if let Some(max) = self.max_complexity {
            if complexity > max {
                return false;
            }
        }
        true
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    Glob::new(pattern)
        .map(|g| g.compile_matcher().is_match(path))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    pub name: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub mccabe: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub sloc: u32,
    pub abc_magnitude: f64,
    pub return_count: u32,
    pub test_scoring: TestScoringMetric,
    pub aird: u32,
    pub aicp: u32,
    pub external_calls: u32,
    pub state_coupling: u32,
}

impl FunctionMetrics {
    pub fn max_complexity(&self) -> u32 {
        std::cmp::max(self.mccabe, self.cognitive)
    }
}

pub fn visit_functions<F>(cursor: &mut TreeCursor, source_code: &str, callback: &mut F)
where
    F: FnMut(Node, &str),
{
    let node = cursor.node();

    if matches!(
        node.kind(),
        "function_definition"
            | "function_item"
            | "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
            | "subprogram_body"
            | "expression_function_declaration"
            | "task_body"
            | "method_declaration"
            | "func_literal"
            | "constructor_declaration"
            | "local_function_statement"
            | "init_declaration"
            // Fortran: function subprogram, subroutine subprogram, module procedure, main program
            | "function"
            | "subroutine"
            | "module_procedure"
            | "program"
    ) {
        callback(node, source_code);
    }

    if cursor.goto_first_child() {
        loop {
            visit_functions(cursor, source_code, callback);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Extract the name of a Lua anonymous function_definition from its assignment context.
fn get_lua_assignment_name(node: Node, source_code: &str) -> Option<String> {
    let parent = node.parent()?;
    match parent.kind() {
        "field" => {
            let mut cur = parent.walk();
            let found = parent.named_children(&mut cur).find(|c| c.kind() == "identifier");
            found
                .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
                .map(|s| s.to_string())
        }
        "expression_list" => {
            let idx = {
                let mut cur = parent.walk();
                let pos = parent
                    .named_children(&mut cur)
                    .position(|c| c.id() == node.id())
                    .unwrap_or(0);
                pos
            };
            let assign = parent.parent()?;
            if assign.kind() != "assignment_statement" {
                return None;
            }
            let mut cur = assign.walk();
            let found = assign.children(&mut cur).find(|c| c.kind() == "variable_list");
            let var_list = found?;
            let mut cur2 = var_list.walk();
            let var = var_list.named_children(&mut cur2).nth(idx)?;
            if var.kind() != "identifier" {
                return None;
            }
            var.utf8_text(source_code.as_bytes()).ok().map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Extract the name of an arrow_function or anonymous function_expression from
/// the surrounding assignment context. Returns None for truly anonymous usage
/// (callbacks, IIFEs, return values, etc.).
fn get_name_from_assignment_context(node: Node, source_code: &str) -> Option<String> {
    let parent = node.parent()?;
    match parent.kind() {
        // const foo = () => {}  or  const foo = function() {}
        "variable_declarator" => parent
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
            .map(|s| s.to_string()),
        // { foo: () => {} }
        "pair" => parent
            .child_by_field_name("key")
            .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
            .map(|s| s.to_string()),
        // class { foo = () => {} }
        "public_field_definition" => parent
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn name_field(node: Node, source_code: &str) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
        .map(|s| s.to_string())
}

fn name_in_child(node: Node, child_kind: &str, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();
    let child = node.children(&mut cursor).find(|c| c.kind() == child_kind)?;
    name_field(child, source_code)
}

fn get_c_name(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(child, source_code);
        }
        if child.kind() == "pointer_declarator" {
            if let Some(name) = get_function_name_from_declarator(child, source_code) {
                return Some(name);
            }
        }
    }
    None
}

pub fn get_function_name(node: Node, source_code: &str) -> Option<String> {
    match node.kind() {
        "function_item"
        | "method_definition"
        | "generator_function_declaration"
        | "generator_function"
        | "method_declaration"
        | "constructor_declaration"
        | "local_function_statement" => name_field(node, source_code),

        "function_declaration" => name_field(node, source_code).or_else(|| {
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).find(|c| c.kind() == "simple_identifier");
            found.and_then(|c| c.utf8_text(source_code.as_bytes()).ok()).map(|s| s.to_string())
        }),

        "function_definition" => name_field(node, source_code)
            .or_else(|| get_c_name(node, source_code))
            .or_else(|| get_lua_assignment_name(node, source_code)),

        "function_expression" => {
            name_field(node, source_code).or_else(|| get_name_from_assignment_context(node, source_code))
        }

        "arrow_function" => get_name_from_assignment_context(node, source_code),

        "init_declaration" => Some("init".to_string()),

        "func_literal" => None,

        "subprogram_body" | "expression_function_declaration" => {
            let mut cursor = node.walk();
            let spec = node
                .children(&mut cursor)
                .find(|c| matches!(c.kind(), "function_specification" | "procedure_specification"))?;
            name_field(spec, source_code)
        }

        "task_body" => {
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).find(|c| c.kind() == "identifier");
            found.and_then(|c| c.utf8_text(source_code.as_bytes()).ok()).map(|s| s.to_string())
        }

        "function" => name_in_child(node, "function_statement", source_code),
        "subroutine" => name_in_child(node, "subroutine_statement", source_code),
        "module_procedure" => name_in_child(node, "module_procedure_statement", source_code),

        "program" => {
            let mut cursor = node.walk();
            let stmt = node.children(&mut cursor).find(|c| c.kind() == "program_statement")?;
            let mut inner = stmt.walk();
            let first_named = stmt.named_children(&mut inner).next();
            first_named
                .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
                .map(|s| s.to_string())
                .or_else(|| Some("program".to_string()))
        }

        _ => get_c_name(node, source_code),
    }
}

fn get_function_name_from_declarator(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(child, source_code);
        } else if child.kind() == "pointer_declarator" {
            if let Some(name) = get_function_name_from_declarator(child, source_code) {
                return Some(name);
            }
        }
    }

    None
}

fn get_declarator_name(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier"
            | "qualified_identifier"
            | "destructor_name"
            | "operator_name"
            | "field_identifier" => {
                return Some(child.utf8_text(source_code.as_bytes()).ok()?.to_string());
            }
            "pointer_declarator" | "function_declarator" => {
                if let Some(name) = get_declarator_name(child, source_code) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }

    None
}

/// Collects all function and macro names defined in this translation unit.
/// Used to classify call sites as local vs. external.
pub fn collect_local_names(root: Node, source_code: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_local_names_recursive(root, source_code, &mut names);
    names
}

fn collect_local_names_recursive(node: Node, source_code: &str, names: &mut HashSet<String>) {
    match node.kind() {
        "function_definition"
        | "function_item"
        | "function_declaration"
        | "function_expression"
        | "arrow_function"
        | "method_definition"
        | "generator_function_declaration"
        | "generator_function"
        | "subprogram_body"
        | "expression_function_declaration"
        | "task_body"
        | "method_declaration"
        | "func_literal"
        | "constructor_declaration"
        | "local_function_statement"
        | "init_declaration"
        | "function"
        | "subroutine"
        | "module_procedure"
        | "program" => {
            if let Some(name) = get_function_name(node, source_code) {
                names.insert(name);
            }
        }
        "preproc_def" | "preproc_function_def" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source_code.as_bytes()) {
                    names.insert(name.to_string());
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_local_names_recursive(child, source_code, names);
    }
}

fn handle_call_node(
    node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
    external: &mut HashSet<String>,
) {
    let Some(func_node) = node.child_by_field_name("function") else {
        return;
    };
    match func_node.kind() {
        "identifier" => {
            if let Ok(name) = func_node.utf8_text(source_code.as_bytes()) {
                if !local_names.contains(name) {
                    external.insert(name.to_string());
                }
            }
        }
        "scoped_identifier" | "attribute" | "member_expression" | "selector_expression"
        | "member_access_expression" => {
            if let Ok(name) = func_node.utf8_text(source_code.as_bytes()) {
                external.insert(name.to_string());
            }
        }
        "field_expression" => {
            if let Some(field) = func_node.child_by_field_name("field") {
                if let Ok(method_name) = field.utf8_text(source_code.as_bytes()) {
                    if !local_names.contains(method_name) {
                        external.insert(method_name.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_external_calls_recursive(
    node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
    external: &mut HashSet<String>,
) {
    if node.kind() == "call_expression" || node.kind() == "call"
        || node.kind() == "invocation_expression"
    {
        handle_call_node(node, source_code, local_names, external);
    }
    if node.kind() == "procedure_call_statement" || node.kind() == "function_call" {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source_code.as_bytes()) {
                if !local_names.contains(name) {
                    external.insert(name.to_string());
                }
            }
        }
    }
    if node.kind() == "method_invocation" {
        let has_object = node.child_by_field_name("object").is_some();
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source_code.as_bytes()) {
                if has_object || !local_names.contains(name) {
                    external.insert(name.to_string());
                }
            }
        }
    }
    if node.kind() == "call_expression" && node.child_by_field_name("function").is_none() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            if matches!(
                child.kind(),
                "value_arguments" | "type_arguments" | "annotated_lambda" | "call_suffix"
            ) {
                continue;
            }
            let text = child.utf8_text(source_code.as_bytes()).unwrap_or("").to_string();
            if child.kind() == "navigation_expression" {
                external.insert(text);
            } else if !local_names.contains(&text) {
                external.insert(text);
            }
            break;
        }
    }
    if node.kind() == "object_creation_expression" {
        if let Some(type_node) = node.child_by_field_name("type") {
            if let Ok(name) = type_node.utf8_text(source_code.as_bytes()) {
                external.insert(name.to_string());
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_external_calls_recursive(child, source_code, local_names, external);
    }
}

/// Collects the names of unique external call targets in a function body.
pub fn collect_external_call_names(
    func_node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
) -> HashSet<String> {
    let mut external: HashSet<String> = HashSet::new();
    collect_external_calls_recursive(func_node, source_code, local_names, &mut external);
    external
}

/// Counts unique external call targets in a function body — identifiers called
/// via call_expression that are not defined in the same translation unit.
fn calculate_external_calls(
    func_node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
) -> u32 {
    collect_external_call_names(func_node, source_code, local_names).len() as u32
}

/// Returns `true` if `kind` is a function-like node visited by `visit_functions`.
pub fn is_function_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "function_item"
            | "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
            | "subprogram_body"
            | "expression_function_declaration"
            | "task_body"
            | "method_declaration"
            | "func_literal"
            | "constructor_declaration"
            | "local_function_statement"
            | "init_declaration"
            | "function"
            | "subroutine"
            | "module_procedure"
            | "program"
    )
}

fn is_macro_function_definition(node: Node) -> bool {
    node.kind() == "function_definition"
        && node
            .child_by_field_name("declarator")
            .map(|d| d.kind() == "parenthesized_declarator")
            .unwrap_or(false)
}

/// Sums the raw SLOC of every function node that is directly nested inside `outer`.
/// Stops recursing as soon as a nested function boundary is crossed, so each level
/// only subtracts one layer of nesting (the recursive call in `collect_function_metrics`
/// handles the rest).
pub fn nested_fn_sloc(outer: Node, source_code: &str, sloc_mode: SlocMode) -> u32 {
    let mut total = 0u32;
    let mut cursor = outer.walk();
    if cursor.goto_first_child() {
        loop {
            accumulate_nested_sloc(cursor.node(), source_code, sloc_mode, &mut total);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    total
}

fn accumulate_nested_sloc(node: Node, source_code: &str, sloc_mode: SlocMode, total: &mut u32) {
    if is_function_kind(node.kind()) && !is_macro_function_definition(node) {
        *total += match sloc_mode {
            SlocMode::Python       => calculate_sloc_python(node, source_code.as_bytes()),
            SlocMode::Ada          => calculate_sloc_ada(node, source_code.as_bytes()),
            SlocMode::Fortran      => calculate_sloc_fortran(node, source_code.as_bytes()),
            SlocMode::FortranFixed => calculate_sloc_fixed_form_fortran(node, source_code.as_bytes()),
            SlocMode::Lua          => complexity::calculate_sloc_lua(node, source_code.as_bytes()),
            SlocMode::Default      => calculate_sloc(node, source_code.as_bytes()),
        };
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        accumulate_nested_sloc(child, source_code, sloc_mode, total);
    }
}

fn should_process_function(
    function_name: &str,
    complexity: u32,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> bool {
    if let Some(rules) = include_rules {
        if !rules.matches_function(function_name) {
            return false;
        }
        if !rules.matches_complexity(complexity) {
            return false;
        }
    }

    if let Some(rules) = exclude_rules {
        let matches_func =
            !rules.function_patterns.is_empty() && rules.matches_function(function_name);
        let matches_complexity = (rules.min_complexity.is_some() || rules.max_complexity.is_some())
            && rules.matches_complexity(complexity);

        let should_exclude = if rules.function_patterns.is_empty()
            && rules.min_complexity.is_none()
            && rules.max_complexity.is_none()
        {
            false
        } else if rules.function_patterns.is_empty() {
            matches_complexity
        } else if rules.min_complexity.is_none() && rules.max_complexity.is_none() {
            matches_func
        } else {
            matches_func && matches_complexity
        };

        if should_exclude {
            return false;
        }
    }

    true
}

pub fn collect_function_metrics(
    tree: &Tree,
    source_code: &str,
    file_path: &str,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
    count_anonymous_closures: bool,
) -> Vec<FunctionMetrics> {
    let root_node = tree.root_node();
    let local_names = collect_local_names(root_node, source_code);
    let mut cursor = root_node.walk();
    let mut metrics = Vec::new();

    let sloc_mode = sloc_mode_for_file(file_path);
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        let name_opt = get_function_name(node, src).or_else(|| {
            let is_anonymous_node = matches!(
                node.kind(),
                "func_literal" | "arrow_function" | "function_expression" | "generator_function"
            ) || (sloc_mode == SlocMode::Lua && node.kind() == "function_definition");
            if count_anonymous_closures && is_anonymous_node {
                let pos = node.start_position();
                Some(format!("<anonymous>@{}:{}", pos.row + 1, pos.column + 1))
            } else {
                None
            }
        });
        if let Some(name) = name_opt {
            let mccabe = calculate_mccabe_complexity(node, src.as_bytes());
            let cognitive = calculate_cognitive_complexity(node, src.as_bytes());
            let nesting = calculate_nesting_depth(node);
            let sloc = {
                let raw = match sloc_mode {
                    SlocMode::Python       => calculate_sloc_python(node, src.as_bytes()),
                    SlocMode::Ada          => calculate_sloc_ada(node, src.as_bytes()),
                    SlocMode::Fortran      => calculate_sloc_fortran(node, src.as_bytes()),
                    SlocMode::FortranFixed => calculate_sloc_fixed_form_fortran(node, src.as_bytes()),
                    SlocMode::Lua          => complexity::calculate_sloc_lua(node, src.as_bytes()),
                    SlocMode::Default      => calculate_sloc(node, src.as_bytes()),
                };
                raw.saturating_sub(nested_fn_sloc(node, src, sloc_mode))
            };
            let abc = calculate_abc_complexity(node, src.as_bytes());
            let abc_magnitude = abc.magnitude();
            let return_count = calculate_return_count(node);
            let test_scoring = calculate_test_scoring(node, src.as_bytes());
            let external_calls = calculate_external_calls(node, src, &local_names);
            let state_coupling = calculate_state_coupling(node, src.as_bytes());
            let aird = calculate_aird(
                cognitive,
                sloc,
                nesting,
                test_scoring.total_score,
                test_scoring.documentation_score,
                state_coupling,
            );
            let aicp = calculate_aicp(external_calls, sloc, test_scoring.documentation_score);

            let max_complexity = std::cmp::max(mccabe, cognitive);

            if should_process_function(&name, max_complexity, include_rules, exclude_rules) {
                let start_line = (node.start_position().row as u32) + 1;
                let end_line = (node.end_position().row as u32) + 1;
                metrics.push(FunctionMetrics {
                    name,
                    file_path: file_path.to_string(),
                    start_line,
                    end_line,
                    mccabe,
                    cognitive,
                    nesting,
                    sloc,
                    abc_magnitude,
                    return_count,
                    test_scoring,
                    aird,
                    aicp,
                    external_calls,
                    state_coupling,
                });
            }
        }
    });

    metrics
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

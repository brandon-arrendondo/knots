// knots library - shared complexity calculation functions

use anyhow::{Context, Result};
use globset::Glob;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Tree, TreeCursor};
pub mod complexity;
pub mod coupling;

// Re-export complexity functions for use by workspace members and for internal use
pub use complexity::{
    apply_aird_ce_multiplier, calculate_abc_complexity, calculate_aicp, calculate_aird,
    calculate_cognitive_complexity, calculate_mccabe_complexity, calculate_nesting_depth,
    calculate_return_count, calculate_sloc, calculate_sloc_ada, calculate_sloc_fortran,
    calculate_sloc_python, calculate_state_coupling, calculate_test_scoring,
    calculate_unreachable_blocks, TestScoringMetric,
};

// File-level Ce/Ca/Instability coupling metrics, built on the substrate's
// syntactic import extraction.
pub use coupling::{apply_file_ce_to_aird, build_import_graph, FileCoupling, ImportGraph};

// Re-export tree-sitter core (a direct knots dep) plus every grammar. Grammars
// now live behind the substrate, so knots carries no direct grammar deps; these
// re-exports keep existing `knots::tree_sitter_*` paths working.
pub use lang_parsing_substrate::{
    tree_sitter_ada, tree_sitter_c, tree_sitter_c_sharp, tree_sitter_cpp, tree_sitter_fortran,
    tree_sitter_go, tree_sitter_java, tree_sitter_javascript, tree_sitter_kotlin_ng,
    tree_sitter_lua, tree_sitter_php, tree_sitter_python, tree_sitter_rust, tree_sitter_scala,
    tree_sitter_swift, tree_sitter_typescript,
};
pub use tree_sitter;

// Language registry, detection, and SLOC-mode lookup also live in the substrate
// (they were extracted from knots). Re-export so existing `knots::` paths resolve.
pub use lang_parsing_substrate::{
    is_parseable_extension, is_source_extension, language_for_file, language_info_for_file,
    languages, sloc_mode_for_file, supported_languages_report, LanguageInfo, SlocMode,
};

// Inline suppression / ignore-region scanning (tools:off, tools:suppress)
// also lives in the substrate — re-exported for consumers that want to
// inspect suppressions directly rather than through `FunctionMetrics::suppressed`.
pub use lang_parsing_substrate::{ignored_regions, suppressions, IgnoredRegion, Suppression};

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
    /// Efferent coupling (Ce) of this function's file, from the corpus-wide
    /// import graph — `0` outside `--recursive` mode, where no corpus exists
    /// to resolve imports against. Already folded into `aird` via
    /// [`complexity::apply_aird_ce_multiplier`]; kept here so violation
    /// output can show it as a distinct driver.
    pub file_ce: u32,
    /// Count of dead-code basic blocks (statements written directly after a
    /// `return` in the same block) from [`complexity::calculate_unreachable_blocks`].
    /// Always `0` outside `c`/`cpp`/`rust`, since the substrate's CFG builder
    /// doesn't model other languages yet.
    pub unreachable_blocks: u32,
    /// Metric keys (e.g. `"cognitive"`, `"mccabe"`) suppressed for this
    /// function via a `tools:suppress knots:<metric>` comment, or all keys
    /// when an unqualified `tools:off` region covers it. Populated by
    /// [`collect_function_metrics`] from `lang_parsing_substrate`'s inline
    /// suppression scan; threshold checks consult this before flagging a
    /// violation.
    pub suppressed: std::collections::HashSet<String>,
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
    // Iterative pre-order walk driven entirely by the cursor's own
    // navigation (no explicit stack needed) — a real-world file with a
    // multi-thousand-deep nested control structure overflowed the call
    // stack under the naive recursive version this replaces (the same
    // failure mode `lang_parsing_substrate::query`'s traversal helpers
    // guard against). `depth` tracks how many `goto_parent` calls are
    // needed to get back to the node the caller handed us, so this never
    // wanders into that node's own siblings — matching the old recursive
    // function, which only ever recursed into children, never siblings,
    // of its `node` argument.
    let mut depth: u32 = 0;
    loop {
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
            depth += 1;
            continue;
        }

        loop {
            if depth == 0 {
                return;
            }
            if cursor.goto_next_sibling() {
                break;
            }
            cursor.goto_parent();
            depth -= 1;
        }
    }
}

/// Extract the name of a Lua anonymous function_definition from its assignment context.
fn get_lua_assignment_name(node: Node, source_code: &str) -> Option<String> {
    let parent = node.parent()?;
    match parent.kind() {
        "field" => {
            let mut cur = parent.walk();
            let found = parent
                .named_children(&mut cur)
                .find(|c| c.kind() == "identifier");
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
            let found = assign
                .children(&mut cur)
                .find(|c| c.kind() == "variable_list");
            let var_list = found?;
            let mut cur2 = var_list.walk();
            let var = var_list.named_children(&mut cur2).nth(idx)?;
            if var.kind() != "identifier" {
                return None;
            }
            var.utf8_text(source_code.as_bytes())
                .ok()
                .map(|s| s.to_string())
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
    let child = node
        .children(&mut cursor)
        .find(|c| c.kind() == child_kind)?;
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
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "simple_identifier");
            found
                .and_then(|c| c.utf8_text(source_code.as_bytes()).ok())
                .map(|s| s.to_string())
        }),

        "function_definition" => name_field(node, source_code)
            .or_else(|| get_c_name(node, source_code))
            .or_else(|| get_lua_assignment_name(node, source_code)),

        "function_expression" => name_field(node, source_code)
            .or_else(|| get_name_from_assignment_context(node, source_code)),

        "arrow_function" => get_name_from_assignment_context(node, source_code),

        "init_declaration" => Some("init".to_string()),

        "func_literal" => None,

        "subprogram_body" | "expression_function_declaration" => {
            let mut cursor = node.walk();
            let spec = node.children(&mut cursor).find(|c| {
                matches!(
                    c.kind(),
                    "function_specification" | "procedure_specification"
                )
            })?;
            name_field(spec, source_code)
        }

        "task_body" => {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "identifier");
            found
                .and_then(|c| c.utf8_text(source_code.as_bytes()).ok())
                .map(|s| s.to_string())
        }

        "function" => name_in_child(node, "function_statement", source_code),
        "subroutine" => name_in_child(node, "subroutine_statement", source_code),
        "module_procedure" => name_in_child(node, "module_procedure_statement", source_code),

        "program" => {
            let mut cursor = node.walk();
            let stmt = node
                .children(&mut cursor)
                .find(|c| c.kind() == "program_statement")?;
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

// Iterative, not recursive: C's declarator grammar wraps at most one
// meaningful inner declarator per pointer_declarator/function_declarator
// node, so this only ever walks a single chain — but that chain's depth is
// attacker-controlled (`int ****...****f(void)`), so it must not grow the
// call stack the way the naive recursive version did.
fn get_function_name_from_declarator(root: Node, source_code: &str) -> Option<String> {
    let mut current = root;
    loop {
        let mut cursor = current.walk();
        let mut next_pointer = None;
        for child in current.children(&mut cursor) {
            if child.kind() == "function_declarator" {
                return get_declarator_name(child, source_code);
            }
            if child.kind() == "pointer_declarator" {
                next_pointer = Some(child);
            }
        }
        current = next_pointer?;
    }
}

// Iterative, not recursive: see get_function_name_from_declarator's comment.
fn get_declarator_name(root: Node, source_code: &str) -> Option<String> {
    let mut current = root;
    loop {
        let mut cursor = current.walk();
        let mut next = None;
        for child in current.children(&mut cursor) {
            match child.kind() {
                "identifier"
                | "qualified_identifier"
                | "destructor_name"
                | "operator_name"
                | "field_identifier" => {
                    return Some(child.utf8_text(source_code.as_bytes()).ok()?.to_string());
                }
                "pointer_declarator" | "function_declarator" => {
                    next = Some(child);
                }
                _ => {}
            }
        }
        current = next?;
    }
}

/// Collects all function and macro names defined in this translation unit.
/// Used to classify call sites as local vs. external.
pub fn collect_local_names(root: Node, source_code: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_local_names_recursive(root, source_code, &mut names);
    names
}

// Iterative (explicit stack), not recursive: a real-world file with a
// multi-thousand-deep nested control structure overflowed the call stack
// under a naive recursive walk of this exact shape.
fn collect_local_names_recursive(root: Node, source_code: &str, names: &mut HashSet<String>) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
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
        stack.extend(node.children(&mut cursor));
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
        "scoped_identifier"
        | "attribute"
        | "member_expression"
        | "selector_expression"
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

// Iterative (explicit stack): see collect_local_names_recursive's comment.
fn collect_external_calls_recursive(
    root: Node,
    source_code: &str,
    local_names: &HashSet<String>,
    external: &mut HashSet<String>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        collect_external_calls_one(node, source_code, local_names, external);
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
}

fn collect_external_calls_one(
    node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
    external: &mut HashSet<String>,
) {
    if node.kind() == "call_expression"
        || node.kind() == "call"
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
            let text = child
                .utf8_text(source_code.as_bytes())
                .unwrap_or("")
                .to_string();
            if child.kind() == "navigation_expression" || !local_names.contains(&text) {
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

// Iterative (explicit stack): see collect_local_names_recursive's comment.
fn accumulate_nested_sloc(root: Node, source_code: &str, sloc_mode: SlocMode, total: &mut u32) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if is_function_kind(node.kind()) && !is_macro_function_definition(node) {
            *total += match sloc_mode {
                SlocMode::Python => calculate_sloc_python(node, source_code.as_bytes()),
                SlocMode::Ada => calculate_sloc_ada(node, source_code.as_bytes()),
                SlocMode::Fortran => calculate_sloc_fortran(node, source_code.as_bytes()),
                SlocMode::Lua => complexity::calculate_sloc_lua(node, source_code.as_bytes()),
                SlocMode::Default => calculate_sloc(node, source_code.as_bytes()),
            };
            continue;
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
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

/// Canonical metric keys used both in `tools:suppress knots:<key>` comments
/// and as `FunctionMetrics::suppressed` entries. Mirrors the CLI's
/// `--<key>-threshold` flag names.
pub const METRIC_KEYS: &[&str] = &[
    "mccabe",
    "cognitive",
    "nesting",
    "sloc",
    "abc",
    "returns",
    "aird",
    "aicp",
    "external_calls",
];

/// Resolves which metric keys are suppressed for the function spanning
/// `start_line..=end_line`: every key if an unqualified (or `knots`-scoped)
/// `tools:off` region overlaps the function, otherwise just the specific
/// keys named by any `tools:suppress knots:<key>` comment whose target line
/// falls inside the function.
fn suppressed_metrics_for(
    start_line: u32,
    end_line: u32,
    regions: &[IgnoredRegion],
    inline_suppressions: &[Suppression],
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();

    for region in regions {
        if !region.applies_to("knots") {
            continue;
        }
        let region_start = region.line_range.start as u32;
        let region_end = region.line_range.end as u32;
        if region_start <= end_line && region_end >= start_line {
            out.extend(METRIC_KEYS.iter().map(|k| k.to_string()));
            return out;
        }
    }

    for s in inline_suppressions {
        if !s.tool.eq_ignore_ascii_case("knots") {
            continue;
        }
        if let Some(target) = s.target_line {
            let target = target as u32;
            if target >= start_line && target <= end_line {
                out.insert(s.rule.clone());
            }
        }
    }

    out
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

    let sloc_mode = sloc_mode_for_file(Path::new(file_path)).unwrap_or(SlocMode::Default);
    let ignore_regions = ignored_regions(source_code, sloc_mode);
    let inline_suppressions = suppressions(source_code, sloc_mode);
    let cfg_language_key = language_info_for_file(Path::new(file_path)).map(|info| info.key);
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        let name_opt = get_function_name(node, src).or_else(|| {
            let is_anonymous_node = matches!(
                node.kind(),
                "func_literal" | "arrow_function" | "function_expression" | "generator_function"
            ) || (sloc_mode == SlocMode::Lua
                && node.kind() == "function_definition");
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
                    SlocMode::Python => calculate_sloc_python(node, src.as_bytes()),
                    SlocMode::Ada => calculate_sloc_ada(node, src.as_bytes()),
                    SlocMode::Fortran => calculate_sloc_fortran(node, src.as_bytes()),
                    SlocMode::Lua => complexity::calculate_sloc_lua(node, src.as_bytes()),
                    SlocMode::Default => calculate_sloc(node, src.as_bytes()),
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
            let unreachable_blocks = cfg_language_key
                .map(|key| calculate_unreachable_blocks(node, src.as_bytes(), key))
                .unwrap_or(0);

            let max_complexity = std::cmp::max(mccabe, cognitive);

            if should_process_function(&name, max_complexity, include_rules, exclude_rules) {
                let start_line = (node.start_position().row as u32) + 1;
                let end_line = (node.end_position().row as u32) + 1;
                let suppressed = suppressed_metrics_for(
                    start_line,
                    end_line,
                    &ignore_regions,
                    &inline_suppressions,
                );
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
                    file_ce: 0,
                    unreachable_blocks,
                    suppressed,
                });
            }
        }
    });

    metrics
}

// The language-registry tests moved to lang-parsing-substrate along with the
// registry itself (`every_registered_extension_maps_to_its_grammar`,
// `fixed_form_fortran_sloc_mode`, etc.).

#[cfg(test)]
mod suppression_tests {
    use super::*;

    fn rust_metrics(source: &str) -> Vec<FunctionMetrics> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        collect_function_metrics(&tree, source, "f.rs", &None, &None, false)
    }

    #[test]
    fn unqualified_block_suppresses_every_metric() {
        let src = "// tools:off\nfn big() {\n    if true {}\n}\n// tools:on\n";
        let metrics = rust_metrics(src);
        assert_eq!(metrics.len(), 1);
        for key in METRIC_KEYS {
            assert!(
                metrics[0].suppressed.contains(*key),
                "{key} should be suppressed"
            );
        }
    }

    #[test]
    fn single_line_suppress_covers_only_named_metric() {
        let src = "// tools:suppress knots:cognitive JUSTIFICATION:\"legacy\"\nfn big() {\n    if true {}\n}\n";
        let metrics = rust_metrics(src);
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].suppressed.contains("cognitive"));
        assert!(!metrics[0].suppressed.contains("mccabe"));
    }

    #[test]
    fn block_scoped_to_a_different_tool_does_not_suppress_knots() {
        let src = "// tools:off funky\nfn big() {\n    if true {}\n}\n// tools:on\n";
        let metrics = rust_metrics(src);
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].suppressed.is_empty());
    }

    #[test]
    fn function_outside_any_marker_has_no_suppressions() {
        let src = "fn big() {\n    if true {}\n}\n";
        let metrics = rust_metrics(src);
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].suppressed.is_empty());
    }
}

use std::collections::HashSet;
use tree_sitter::Node;

/// Calculates McCabe cyclomatic complexity for a function
/// Formula: M = E - N + 2P where E = edges, N = nodes, P = connected components
/// Simplified: Count decision points + 1
pub fn calculate_mccabe_complexity(node: Node, source_code: &[u8]) -> u32 {
    let mut complexity = 1; // Base complexity

    visit_node_mccabe(node, source_code, &mut complexity);

    complexity
}

// Increments complexity if node's `operator` field matches one of `valid_ops`.
fn mccabe_logical_op(node: Node, source_code: &[u8], valid_ops: &[&str], complexity: &mut u32) {
    if let Some(op) = node.child_by_field_name("operator") {
        if let Ok(op_text) = op.utf8_text(source_code) {
            if valid_ops.contains(&op_text) {
                *complexity += 1;
            }
        }
    }
}

fn visit_node_mccabe(node: Node, source_code: &[u8], complexity: &mut u32) {
    // Skip unnamed tokens (punctuation, keyword literals). Without this guard, Ada's named
    // `guard` rule would fire on Swift's unnamed `guard` keyword token, and similar cross-
    // grammar collisions would produce false positives.
    if !node.is_named() {
        return;
    }
    match node.kind() {
        // Rust ? operator / Swift try? — both use "try_expression".
        // No try_operator child → Rust ? (always a branch) or Scala/Kotlin try-block (skip).
        // try_operator child text "try?" → Swift short-circuit; "try"/"try!" → no branch.
        "try_expression" => {
            let mut cur = node.walk();
            let try_op = node.named_children(&mut cur).find(|c| c.kind() == "try_operator");
            match try_op {
                None => {
                    let mut cur2 = node.walk();
                    let is_try_block = node.named_children(&mut cur2)
                        .any(|c| matches!(c.kind(), "catch_clause" | "finally_clause" | "catch_block"));
                    if !is_try_block {
                        *complexity += 1;
                    }
                }
                Some(op) => {
                    if op.utf8_text(source_code).ok() == Some("try?") {
                        *complexity += 1;
                    }
                }
            }
        }

        // Ada: logical operators as unnamed keyword children (and / or / xor).
        // Each occurrence is a separate branch point (unlike cognitive, which chain-counts).
        "expression" => {
            let mut cursor = node.walk();
            let count = node
                .children(&mut cursor)
                .filter(|c| !c.is_named() && matches!(c.kind(), "and" | "or" | "xor"))
                .count() as u32;
            *complexity += count;
        }

        // Ada: selective_accept `else` clause (unnamed keyword) adds one alternative path.
        "selective_accept" => {
            let mut cursor = node.walk();
            if node.children(&mut cursor).any(|c| !c.is_named() && c.kind() == "else") {
                *complexity += 1;
            }
        }

        // Ada: exit when Condition — conditional loop exit is a branch; bare exit is not.
        "exit_statement" => {
            let mut cursor = node.walk();
            if node.children(&mut cursor).any(|c| !c.is_named() && c.kind() == "when") {
                *complexity += 1;
            }
        }

        // Logical operators in operator-field expressions.
        // binary_expression: C/C++/Rust/PHP (&&, ||, ??, ?:, and, or)
        // boolean_operator: Python (and, or)
        // infix_expression: Scala (&&, ||)
        // logical_expression: Fortran (.and., .or., .AND., .OR.)
        "binary_expression" | "boolean_operator" | "infix_expression" | "logical_expression" => {
            let valid_ops: &[&str] = match node.kind() {
                "binary_expression" => &["&&", "||", "??", "?:", "and", "or"],
                "boolean_operator" => &["and", "or"],
                "infix_expression" => &["&&", "||"],
                _ => &[".and.", ".or.", ".AND.", ".OR."],
            };
            mccabe_logical_op(node, source_code, valid_ops, complexity);
        }

        // Every other decision point: flat +1.
        // Covers all control-flow structures across all supported languages.
        //
        // C/C++: if/while/do/for/switch/goto/throw
        // Rust: if_expression, loop variants, match_expression, conditional_expression
        // Python: elif, except, match_statement, ternary (conditional_expression)
        // JS/TS: for_in_statement, optional_chain, ternary_expression
        // Ada: loop_statement, elsif, case_statement_alternative, exception_handler,
        //      select_alternative, guard, timed/conditional/asynchronous select
        // Go: expression_switch_statement, type_switch_statement, select_statement
        // Java: enhanced_for_statement, switch_expression
        // C#: foreach_statement, conditional_access_expression
        // Kotlin: do_while_statement, when_expression, catch_block
        // Swift: guard_statement, repeat_while_statement
        // PHP: else_if_clause, throw_expression, nullsafe_member_access_expression
        // Lua: elseif_statement, repeat_statement
        // Fortran: elseif_clause, select_case/rank/type, where_statement,
        //          elsewhere_clause, arithmetic_if_statement
        "if_statement" | "while_statement" | "do_statement" | "for_statement" | "for_range_loop"
        | "throw_statement" | "raise_statement" | "raise_expression" | "switch_statement"
        | "if_expression" | "while_expression" | "for_expression" | "loop_expression"
        | "do_while_expression" | "match_expression"
        | "conditional_expression" | "ternary_expression"
        | "goto_statement"
        | "elif_clause" | "except_clause" | "match_statement"
        | "for_in_statement" | "optional_chain"
        | "loop_statement" | "elsif_statement_item" | "case_statement_alternative"
        | "exception_handler" | "select_alternative" | "guard"
        | "timed_entry_call" | "conditional_entry_call" | "asynchronous_select"
        | "expression_switch_statement" | "type_switch_statement" | "select_statement"
        | "enhanced_for_statement" | "switch_expression"
        | "foreach_statement" | "conditional_access_expression"
        | "do_while_statement" | "when_expression" | "catch_block"
        | "guard_statement" | "repeat_while_statement"
        | "else_if_clause" | "throw_expression" | "nullsafe_member_access_expression"
        | "elseif_statement" | "repeat_statement"
        | "elseif_clause"
        | "select_case_statement" | "select_rank_statement" | "select_type_statement"
        | "where_statement" | "elsewhere_clause" | "arithmetic_if_statement" => *complexity += 1,

        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_mccabe(child, source_code, complexity);
    }
}

/// Calculates cognitive complexity for a function
/// Based on the Cognitive Complexity specification by SonarSource
pub fn calculate_cognitive_complexity(node: Node, source_code: &[u8]) -> u32 {
    let mut complexity = 0;
    visit_node_cognitive(node, source_code, 0, &mut complexity, None);
    complexity
}

// Handles binary/boolean/logical operator chain-counting for cognitive complexity.
// Returns true if the node was a logical op of a recognized kind (caller should return early).
fn handle_logical_op(
    node: Node,
    source_code: &[u8],
    nesting_level: u32,
    complexity: &mut u32,
    parent_binary_op: Option<&str>,
    valid_ops: &[&str],
) -> bool {
    let Some(op) = node.child_by_field_name("operator") else { return false; };
    let Ok(op_text) = op.utf8_text(source_code) else { return false; };
    if !valid_ops.contains(&op_text) { return false; }
    if parent_binary_op != Some(op_text) {
        *complexity += 1;
    }
    visit_children_cognitive(node, source_code, nesting_level, complexity, Some(op_text));
    true
}

fn visit_node_cognitive(
    node: Node,
    source_code: &[u8],
    nesting_level: u32,
    complexity: &mut u32,
    parent_binary_op: Option<&str>,
) {
    match node.kind() {
        // if/else — special: Ada bare `else` keyword adds a flat +1.
        "if_statement" | "if_expression" => {
            *complexity += 1 + nesting_level;
            let mut cur = node.walk();
            if node.children(&mut cur).any(|c| !c.is_named() && c.kind() == "else") {
                *complexity += 1;
            }
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // else clause — flat +1; if it contains an else-if, recurse into that child directly.
        "else_clause" => {
            *complexity += 1;
            let mut cursor = node.walk();
            let target = node
                .children(&mut cursor)
                .find(|c| matches!(c.kind(), "if_statement" | "if_expression"))
                .unwrap_or(node);
            visit_children_cognitive(target, source_code, nesting_level, complexity, None);
            return;
        }

        // try has NO cost and NO nesting increment; only catch/except gets the penalty.
        "try_statement" => {
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        // Closures/lambdas: nesting +1, no base cost.
        // Covers C++/Rust, Python, JS/TS arrow functions, C# delegates/local fns,
        // Kotlin lambdas, Go closures.
        "lambda_expression" | "closure_expression" | "lambda" | "arrow_function"
        | "anonymous_method_expression" | "local_function_statement"
        | "lambda_literal" | "anonymous_function" | "annotated_lambda"
        | "func_literal" => {
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Nesting structures: +1 + nesting_level, children at nesting+1.
        // Covers loops, switch/match/select, catch/except across all supported languages.
        "while_statement" | "do_statement" | "for_statement" | "for_range_loop"
        | "for_in_statement"
        | "while_expression" | "for_expression" | "loop_expression" | "do_while_expression"
        | "switch_statement" | "match_expression" | "match_statement"
        | "catch_clause" | "except_clause"
        | "loop_statement" | "case_statement" | "exception_handler"
        | "selective_accept" | "timed_entry_call" | "conditional_entry_call" | "asynchronous_select"
        | "expression_switch_statement" | "type_switch_statement" | "select_statement"
        | "enhanced_for_statement" | "switch_expression"
        | "foreach_statement"
        | "do_while_statement" | "when_expression" | "catch_block"
        | "guard_statement" | "repeat_while_statement" | "repeat_statement"
        | "select_case_statement" | "select_rank_statement" | "select_type_statement"
        | "where_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Flat branches: +1, children at same nesting.
        // elif/elsif/elseif/elsewhere across Python, Ada, PHP, Lua, Fortran.
        "elif_clause" | "elsif_statement_item" | "else_if_clause"
        | "elseif_statement" | "elseif_clause" | "elsewhere_clause" => {
            *complexity += 1;
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        // Flat jumps: +1, no recursion needed.
        // goto/throw/raise across C/C++, Rust, Python, PHP (throw_expression); Ada guard; Fortran arithmetic-if.
        "goto_statement" | "throw_statement" | "raise_statement" | "raise_expression"
        | "guard" | "throw_expression" | "arithmetic_if_statement" => {
            *complexity += 1;
        }

        // Logical operator chains: +1 per distinct operator kind, not per operator in a sequence.
        // binary_expression: C/C++/Rust/PHP (&&, ||, ??, and, or)
        // boolean_operator: Python (and, or)
        // infix_expression: Scala (&&, ||)
        // logical_expression: Fortran (.and., .or., .AND., .OR.)
        "binary_expression" | "boolean_operator" | "infix_expression" | "logical_expression" => {
            let valid_ops: &[&str] = match node.kind() {
                "binary_expression" => &["&&", "||", "??", "and", "or"],
                "boolean_operator" => &["and", "or"],
                "infix_expression" => &["&&", "||"],
                _ => &[".and.", ".or.", ".AND.", ".OR."],
            };
            if handle_logical_op(node, source_code, nesting_level, complexity, parent_binary_op, valid_ops) {
                return;
            }
        }

        // Ada: logical operators (and / or / xor) as unnamed keyword children of expression.
        // Count each new operator sequence, not each occurrence.
        "expression" => {
            let mut last_op: Option<&str> = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if !child.is_named() && matches!(child.kind(), "and" | "or" | "xor") {
                    let op = child.kind();
                    if last_op != Some(op) {
                        *complexity += 1;
                        last_op = Some(op);
                    }
                }
            }
        }

        // Ada: exit when Condition — flat +1 only when the `when` keyword is present.
        "exit_statement" => {
            let mut cur = node.walk();
            if node.children(&mut cur).any(|c| !c.is_named() && c.kind() == "when") {
                *complexity += 1;
            }
        }

        _ => {}
    }

    visit_children_cognitive(node, source_code, nesting_level, complexity, parent_binary_op);
}

fn visit_children_cognitive(
    node: Node,
    source_code: &[u8],
    nesting_level: u32,
    complexity: &mut u32,
    parent_binary_op: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_cognitive(child, source_code, nesting_level, complexity, parent_binary_op);
    }
}

/// Calculates maximum nesting depth of control structures
pub fn calculate_nesting_depth(node: Node) -> u32 {
    let mut max_depth = 0;
    visit_node_nesting(node, 0, &mut max_depth);
    max_depth
}

fn visit_node_nesting(node: Node, current_depth: u32, max_depth: &mut u32) {
    // Skip anonymous/terminal nodes (e.g. keyword tokens). In Python, the `lambda` keyword
    // token and the `lambda` expression node share the same kind string; without this guard
    // nesting depth would be double-counted for lambda expressions.
    if !node.is_named() {
        return;
    }

    let new_depth = match node.kind() {
        // C/C++ control structures
        "if_statement" | "while_statement" | "do_statement" | "for_statement"
        | "for_range_loop" | "switch_statement" | "catch_clause" | "lambda_expression"
        // Rust control structures
        | "if_expression" | "while_expression" | "for_expression" | "loop_expression"
        | "match_expression" | "closure_expression"
        // Python control structures
        | "except_clause" | "match_statement" | "lambda"
        // JavaScript control structures
        | "for_in_statement" | "arrow_function"
        // Ada control structures
        | "loop_statement" | "case_statement" | "exception_handler"
        // Go control structures
        | "expression_switch_statement" | "type_switch_statement" | "select_statement"
        | "func_literal"
        // Java control structures
        | "enhanced_for_statement" | "switch_expression"
        // C# control structures
        | "foreach_statement" | "anonymous_method_expression" | "local_function_statement"
        // Kotlin control structures
        | "do_while_statement" | "when_expression" | "catch_block"
        | "lambda_literal" | "anonymous_function" | "annotated_lambda"
        // Swift control structures
        | "guard_statement" | "repeat_while_statement"
        // Lua control structures
        | "repeat_statement"
        // Fortran control structures
        | "do_loop" | "select_case_statement" | "select_rank_statement"
        | "select_type_statement" | "where_statement"
        // Scala: do_while_expression (while_expression and for_expression are already covered above)
        | "do_while_expression" => {
            let depth = current_depth + 1;
            if depth > *max_depth {
                *max_depth = depth;
            }
            depth
        }
        _ => current_depth,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_nesting(child, new_depth, max_depth);
    }
}

/// Calculates Source Lines of Code (SLOC) - non-comment, non-blank lines (C/C++/Rust).
pub fn calculate_sloc(node: Node, source_code: &[u8]) -> u32 {
    calculate_sloc_inner(node, source_code, false)
}

/// Calculates SLOC for Python source — additionally skips lines beginning with `#`.
pub fn calculate_sloc_python(node: Node, source_code: &[u8]) -> u32 {
    calculate_sloc_inner(node, source_code, true)
}

/// Calculates SLOC for Ada source — skips lines beginning with `--` (Ada's only comment style).
pub fn calculate_sloc_ada(node: Node, source_code: &[u8]) -> u32 {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= end_byte || end_byte > source_code.len() {
        return 0;
    }

    let function_text = &source_code[start_byte..end_byte];
    let mut sloc = 0;

    for line in function_text.split(|&b| b == b'\n') {
        let trimmed = trim_bytes(line);
        if trimmed.is_empty() || trimmed.starts_with(b"--") {
            continue;
        }
        sloc += 1;
    }

    sloc
}

/// Calculates SLOC for Lua source — skips lines beginning with `--` (same style as Ada).
/// Long-form block comments (`--[[ ... ]]`) have their opening line skipped; interior
/// lines are counted (a known minor overcount, acceptable in practice).
pub fn calculate_sloc_lua(node: Node, source_code: &[u8]) -> u32 {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= end_byte || end_byte > source_code.len() {
        return 0;
    }

    let function_text = &source_code[start_byte..end_byte];
    let mut sloc = 0;

    for line in function_text.split(|&b| b == b'\n') {
        let trimmed = trim_bytes(line);
        if trimmed.is_empty() || trimmed.starts_with(b"--") {
            continue;
        }
        sloc += 1;
    }

    sloc
}

/// Calculates SLOC for Fortran source — skips lines beginning with `!` (Fortran's only comment style).
pub fn calculate_sloc_fortran(node: Node, source_code: &[u8]) -> u32 {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= end_byte || end_byte > source_code.len() {
        return 0;
    }

    let function_text = &source_code[start_byte..end_byte];
    let mut sloc = 0;

    for line in function_text.split(|&b| b == b'\n') {
        let trimmed = trim_bytes(line);
        if trimmed.is_empty() || trimmed.starts_with(b"!") {
            continue;
        }
        sloc += 1;
    }

    sloc
}

fn calculate_sloc_inner(node: Node, source_code: &[u8], skip_hash_comments: bool) -> u32 {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= end_byte || end_byte > source_code.len() {
        return 0;
    }

    let function_text = &source_code[start_byte..end_byte];
    let mut sloc = 0;
    let mut in_multiline_comment = false;

    for line in function_text.split(|&b| b == b'\n') {
        let trimmed = trim_bytes(line);

        if trimmed.is_empty() {
            continue;
        }

        // Handle multi-line comments (C/C++/Rust /* ... */)
        if in_multiline_comment {
            if let Some(pos) = find_bytes(trimmed, b"*/") {
                in_multiline_comment = false;
                let after_comment = &trimmed[pos + 2..];
                if !trim_bytes(after_comment).is_empty() {
                    sloc += 1;
                }
            }
            continue;
        }

        // Python # comments (# in C/C++/Rust is a preprocessor directive, not a comment)
        if skip_hash_comments && trimmed.starts_with(b"#") {
            continue;
        }

        // Check for start of multi-line comment
        if let Some(pos) = find_bytes(trimmed, b"/*") {
            // Check if it ends on the same line
            if let Some(end_pos) = find_bytes(&trimmed[pos..], b"*/") {
                let before = &trimmed[..pos];
                let after = &trimmed[pos + end_pos + 2..];
                if !trim_bytes(before).is_empty() || !trim_bytes(after).is_empty() {
                    sloc += 1;
                }
            } else {
                in_multiline_comment = true;
                if !trim_bytes(&trimmed[..pos]).is_empty() {
                    sloc += 1;
                }
            }
            continue;
        }

        // Single-line // comments (C/C++/Rust)
        if trimmed.starts_with(b"//") {
            continue;
        }

        sloc += 1;
    }

    sloc
}

fn trim_bytes(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    (0..=(haystack.len() - needle.len())).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// Represents ABC complexity components
#[derive(Debug, Clone, Copy)]
pub struct AbcComplexity {
    pub assignments: u32,
    pub branches: u32,
    pub conditions: u32,
}

impl AbcComplexity {
    pub fn magnitude(&self) -> f64 {
        let a = self.assignments as f64;
        let b = self.branches as f64;
        let c = self.conditions as f64;
        (a * a + b * b + c * c).sqrt()
    }
}

/// Calculates ABC complexity metric
/// A = Assignments (assignment statements and increments/decrements)
/// B = Branches (function/method calls)
/// C = Conditions (conditional logic)
pub fn calculate_abc_complexity(node: Node, source_code: &[u8]) -> AbcComplexity {
    let mut assignments = 0;
    let mut branches = 0;
    let mut conditions = 0;

    visit_node_abc(
        node,
        source_code,
        &mut assignments,
        &mut branches,
        &mut conditions,
    );

    AbcComplexity {
        assignments,
        branches,
        conditions,
    }
}

fn abc_try_expression(node: Node, source_code: &[u8], conditions: &mut u32) {
    let mut cur = node.walk();
    let try_op = node.named_children(&mut cur).find(|c| c.kind() == "try_operator");
    match try_op {
        None => {
            let mut cur2 = node.walk();
            let is_try_block = node
                .named_children(&mut cur2)
                .any(|c| matches!(c.kind(), "catch_clause" | "finally_clause" | "catch_block"));
            if !is_try_block {
                *conditions += 1;
            }
        }
        Some(op) => {
            if op.utf8_text(source_code).ok() == Some("try?") {
                *conditions += 1;
            }
        }
    }
}

fn abc_operator_is_condition(node: Node, source_code: &[u8], ops: &[&str]) -> bool {
    node.child_by_field_name("operator")
        .and_then(|op| op.utf8_text(source_code).ok())
        .map(|text| ops.iter().any(|&o| o == text))
        .unwrap_or(false)
}

fn abc_expression_ada(node: Node, conditions: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() && matches!(child.kind(), "and" | "or" | "xor") {
            *conditions += 1;
        }
    }
}

fn abc_exit_statement_ada(node: Node, conditions: &mut u32) {
    let mut cursor = node.walk();
    if node
        .children(&mut cursor)
        .any(|c| !c.is_named() && c.kind() == "when")
    {
        *conditions += 1;
    }
}

fn visit_node_abc(
    node: Node,
    source_code: &[u8],
    assignments: &mut u32,
    branches: &mut u32,
    conditions: &mut u32,
) {
    match node.kind() {
        // Assignments — C/C++
        "assignment_expression" => *assignments += 1,
        "update_expression" => *assignments += 1, // C/C++ ++ and --
        // Assignments — Rust
        "compound_assignment_expr" => *assignments += 1, // Rust +=, -=, *=, etc.
        // Assignments — Python
        "assignment" => *assignments += 1,           // x = value
        "augmented_assignment" => *assignments += 1, // x += value
        "named_expression" => *assignments += 1,     // x := value (walrus)
        // Assignments — JavaScript (augmented_assignment_expression: +=, -=, etc.)
        "augmented_assignment_expression" => *assignments += 1,

        // Assignments — Ada (:= operator)
        "assignment_statement" => *assignments += 1,

        // Branches — function calls (C/C++/Rust use call_expression, Python uses call, C# uses invocation_expression)
        "call_expression" | "call" | "invocation_expression" => *branches += 1,
        // Ada: procedure and function invocations; Lua: function call
        "procedure_call_statement" | "function_call" => *branches += 1,
        // Fortran: CALL statement
        "subroutine_call" => *branches += 1,

        // Branches: throw, new, delete create control flow paths (C/C++)
        "throw_statement" | "new_expression" | "delete_expression"
        | "raise_statement" | "raise_expression" => *branches += 1,

        // Conditions (C/C++)
        "if_statement"
        | "while_statement"
        | "do_statement"
        | "for_statement"
        | "for_range_loop"
        | "switch_statement"
        | "conditional_expression" => *conditions += 1,

        // Conditions (JavaScript)
        "for_in_statement" | "ternary_expression" => *conditions += 1,

        // Conditions (Rust + Scala do-while)
        "if_expression" | "while_expression" | "for_expression" | "loop_expression"
        | "match_expression" | "do_while_expression" => *conditions += 1,
        // Rust ? / Swift try? — same grammar-level discrimination as visit_node_mccabe.
        // Scala/Kotlin try-catch also uses try_expression; skip the try itself (catch adds conditions).
        "try_expression" => abc_try_expression(node, source_code, conditions),

        // Conditions (Python)
        "elif_clause" | "match_statement" => *conditions += 1,

        // Conditions (Ada)
        "elsif_statement_item" | "loop_statement" | "case_statement" | "exception_handler" => {
            *conditions += 1
        }
        // Ada tasking conditions
        "select_alternative" | "guard"
        | "timed_entry_call" | "conditional_entry_call" | "asynchronous_select" => {
            *conditions += 1
        }

        // Assignments (Go): short variable declaration `:=`
        "short_var_declaration" => *assignments += 1,

        // Conditions (Go)
        "expression_switch_statement" | "type_switch_statement" | "select_statement" => {
            *conditions += 1
        }

        // Branches (Java): method invocations and object creation
        // object_creation_expression also covers PHP `new Foo()`
        "method_invocation" | "object_creation_expression" => *branches += 1,

        // Branches (PHP): function/method call expressions
        "function_call_expression" | "member_call_expression" | "nullsafe_member_call_expression" => {
            *branches += 1
        }
        // PHP 8: throw expression used as a value (rhs of ??, assignment, etc.)
        "throw_expression" => *branches += 1,

        // Conditions (Java)
        "enhanced_for_statement" | "switch_expression" => *conditions += 1,

        // Conditions (C#)
        "foreach_statement" | "conditional_access_expression" => *conditions += 1,

        // Conditions (Kotlin)
        "do_while_statement" | "when_expression" => *conditions += 1,

        // Scala: assignments (val/var declarations)
        "val_definition" | "var_definition" => *assignments += 1,
        // Scala: logical operators in infix_expression
        "infix_expression" => {
            if abc_operator_is_condition(node, source_code, &["&&", "||"]) {
                *conditions += 1;
            }
        }

        // Conditions (Swift)
        "guard_statement" | "repeat_while_statement" => *conditions += 1,

        // JS/TS optional chaining: a?.b
        "optional_chain" => *conditions += 1,

        // PHP: elseif clause and null-safe access
        "else_if_clause" | "nullsafe_member_access_expression" => *conditions += 1,

        // Fortran conditions
        "elseif_clause" | "select_case_statement" | "select_rank_statement"
        | "select_type_statement" | "where_statement" | "elsewhere_clause"
        | "arithmetic_if_statement" => *conditions += 1,
        // Fortran: logical operators .AND. / .OR. are condition branches
        "logical_expression" => {
            if abc_operator_is_condition(node, source_code, &[".and.", ".or.", ".AND.", ".OR."]) {
                *conditions += 1;
            }
        }

        // Logical operators (C/C++/Rust: &&/||; JS/TS/C#: ??; Kotlin Elvis: ?:; PHP: and/or)
        "binary_expression" => {
            if abc_operator_is_condition(node, source_code, &["&&", "||", "??", "?:", "and", "or"]) {
                *conditions += 1;
            }
        }

        // Logical operators (Python: and/or)
        "boolean_operator" => {
            if abc_operator_is_condition(node, source_code, &["and", "or"]) {
                *conditions += 1;
            }
        }

        // Logical operators (Ada: and then / or else / xor / and / or).
        // Count each and/or/xor child as a condition branch point.
        "expression" => abc_expression_ada(node, conditions),

        // Ada: exit when Condition — the `when` makes it a conditional branch.
        "exit_statement" => abc_exit_statement_ada(node, conditions),

        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_abc(child, source_code, assignments, branches, conditions);
    }
}

/// Calculates the number of return statements in a function
pub fn calculate_return_count(node: Node) -> u32 {
    let mut count = 0;
    visit_node_returns(node, &mut count);
    count
}

fn visit_node_returns(node: Node, count: &mut u32) {
    if matches!(
        node.kind(),
        "return_statement" | "return_expression"
        // Ada uses split names for simple and extended returns
        | "simple_return_statement" | "extended_return_statement"
    ) {
        *count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_returns(child, count);
    }
}

/// Represents test scoring metric components
/// Based on automated test generation difficulty assessment
#[derive(Debug, Clone, Copy)]
pub struct TestScoringMetric {
    pub signature_score: u32,
    pub dependency_score: u32,
    pub observable_score: u32,
    pub implementation_score: u32,
    pub documentation_score: i32,
    pub total_score: i32,
}

impl TestScoringMetric {
    pub fn classification(&self) -> &'static str {
        match self.total_score {
            i32::MIN..=10 => "Trivial",
            11..=20 => "Simple",
            21..=30 => "Moderate",
            31..=40 => "Complex",
            41..=50 => "Difficult",
            _ => "Very Hard",
        }
    }

    #[allow(dead_code)]
    pub fn automation_level(&self) -> &'static str {
        match self.total_score {
            i32::MIN..=10 => "Fully automatable",
            11..=20 => "Automated with minimal metadata",
            21..=30 => "Needs good documentation",
            31..=40 => "Requires detailed specifications",
            41..=50 => "May need manual test design",
            _ => "Extensive manual effort needed",
        }
    }
}

/// Calculates test scoring metric for assessing test generation difficulty
/// Score components: signature, dependency, observable behavior, implementation, documentation
pub fn calculate_test_scoring(node: Node, source_code: &[u8]) -> TestScoringMetric {
    let signature = calculate_signature_complexity(node, source_code);
    let dependency = calculate_dependency_score(node, source_code);
    let observable = calculate_observable_behavior_score(node, source_code);

    // Use existing cyclomatic complexity for implementation score
    let mccabe = calculate_mccabe_complexity(node, source_code);
    let implementation = map_cyclomatic_to_implementation_score(mccabe);

    let documentation = calculate_documentation_score(node, source_code);

    let total = signature as i32 + dependency as i32 + observable as i32 + implementation as i32
        - documentation;

    TestScoringMetric {
        signature_score: signature,
        dependency_score: dependency,
        observable_score: observable,
        implementation_score: implementation,
        documentation_score: documentation,
        total_score: total,
    }
}

/// Maps cyclomatic complexity to implementation score (0-10 scale)
fn map_cyclomatic_to_implementation_score(cyclomatic: u32) -> u32 {
    match cyclomatic {
        1..=5 => (cyclomatic - 1) / 2,        // 1-5 -> 0-2
        6..=10 => 3 + (cyclomatic - 6) / 2,   // 6-10 -> 3-5
        11..=20 => 6 + (cyclomatic - 11) / 5, // 11-20 -> 6-8
        _ => 9,                               // 20+ -> 9-10
    }
}

/// Calculates signature complexity based on function parameters and return type
fn calculate_signature_complexity(node: Node, source_code: &[u8]) -> u32 {
    let mut input_score = 0;
    let mut output_score = 0;

    // Find function declarator
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(declarator) = child.child_by_field_name("declarator") {
                // Analyze parameters
                input_score = analyze_parameters(declarator, source_code);

                // Analyze return type
                if let Some(type_node) = child.child_by_field_name("type") {
                    output_score = analyze_return_type(type_node, source_code);
                }
            }
        } else if child.kind() == "declaration" {
            // For function declarations
            if let Some(declarator) = child.child_by_field_name("declarator") {
                input_score = analyze_parameters(declarator, source_code);
            }
            if let Some(type_node) = child.child_by_field_name("type") {
                output_score = analyze_return_type(type_node, source_code);
            }
        }
    }

    // Combined score capped at 10
    (input_score + output_score).min(10)
}

fn analyze_parameters(declarator: Node, source_code: &[u8]) -> u32 {
    let mut param_count = 0;
    let mut has_pointer = false;
    let mut has_function_pointer = false;
    let mut has_void_ptr = false;
    let mut has_variadic = false;

    // Find parameter list
    let mut cursor = declarator.walk();
    for child in declarator.children(&mut cursor) {
        if child.kind() == "parameter_list" {
            let mut param_cursor = child.walk();
            for param in child.children(&mut param_cursor) {
                if param.kind() == "parameter_declaration" {
                    param_count += 1;

                    // Check for pointers, function pointers, void*
                    let param_text = param.utf8_text(source_code).unwrap_or("");
                    if param_text.contains("void*") || param_text.contains("void *") {
                        has_void_ptr = true;
                    } else if param_text.contains("(*") || param_text.contains("* )") {
                        has_function_pointer = true;
                    } else if param_text.contains('*') {
                        has_pointer = true;
                    }
                } else if param.kind() == "variadic_parameter" {
                    has_variadic = true;
                }
            }
        }
    }

    // Score based on complexity
    if has_function_pointer || has_void_ptr || has_variadic {
        10
    } else if has_pointer && param_count > 1 {
        8
    } else if has_pointer {
        6
    } else if param_count > 1 {
        4
    } else if param_count == 1 {
        2
    } else {
        0
    }
}

fn analyze_return_type(type_node: Node, source_code: &[u8]) -> u32 {
    let type_text = type_node.utf8_text(source_code).unwrap_or("");

    if type_text.contains("void") && !type_text.contains('*') {
        0
    } else if type_text.contains("struct") {
        10
    } else if type_text.contains('*') {
        6
    } else if type_text.contains("enum") {
        4
    } else {
        2
    }
}

/// Calculates dependency and side effect score
fn calculate_dependency_score(node: Node, source_code: &[u8]) -> u32 {
    let mut score = 0;
    let mut has_io = false;
    let mut has_allocation = false;
    let mut has_system_calls = false;
    let mut modifies_globals = false;

    visit_node_dependencies(
        node,
        source_code,
        &mut has_io,
        &mut has_allocation,
        &mut has_system_calls,
        &mut modifies_globals,
    );

    // Check for global state access (simplified heuristic)
    if modifies_globals {
        score += 6;
    }

    // I/O operations
    if has_io {
        score += 2;
    }

    // Memory allocation
    if has_allocation {
        score += 3;
    }

    // System calls
    if has_system_calls {
        score += 2;
    }

    score.min(10)
}

fn dep_classify_call(func_name: &str, has_io: &mut bool, has_allocation: &mut bool, has_system_calls: &mut bool) {
    if matches!(
        func_name,
        "fopen" | "fclose" | "fread" | "fwrite" | "fprintf" | "fscanf" | "fgets" | "fputs"
        | "fseek" | "ftell" | "rewind" | "printf" | "scanf" | "puts" | "getc" | "putc"
        | "open" | "print" | "input"
    ) {
        *has_io = true;
    }
    if matches!(func_name, "malloc" | "calloc" | "realloc" | "free" | "aligned_alloc") {
        *has_allocation = true;
    }
    if matches!(
        func_name,
        "time" | "clock" | "rand" | "srand" | "getpid" | "fork" | "exec" | "system"
        | "signal" | "kill" | "wait" | "pipe" | "exit" | "abort"
    ) {
        *has_system_calls = true;
    }
}

fn dep_check_global_assignment(node: Node, source_code: &[u8], modifies_globals: &mut bool) {
    let left = match node.child_by_field_name("left") {
        Some(n) if n.kind() == "identifier" => n,
        _ => return,
    };
    // Heuristic: uppercase-initial identifier is likely a global
    if let Ok(name) = left.utf8_text(source_code) {
        if name.starts_with(|c: char| c.is_uppercase()) {
            *modifies_globals = true;
        }
    }
}

fn visit_node_dependencies(
    node: Node,
    source_code: &[u8],
    has_io: &mut bool,
    has_allocation: &mut bool,
    has_system_calls: &mut bool,
    modifies_globals: &mut bool,
) {
    if matches!(node.kind(), "call_expression" | "call") {
        if let Some(name) = node
            .child_by_field_name("function")
            .and_then(|f| f.utf8_text(source_code).ok())
        {
            dep_classify_call(name, has_io, has_allocation, has_system_calls);
        }
    }

    if matches!(node.kind(), "new_expression" | "delete_expression") {
        *has_allocation = true;
    }

    if node.kind() == "assignment_expression" {
        dep_check_global_assignment(node, source_code, modifies_globals);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_dependencies(child, source_code, has_io, has_allocation, has_system_calls, modifies_globals);
    }
}

/// Calculates observable behavior score (how easy to verify correctness)
fn calculate_observable_behavior_score(node: Node, source_code: &[u8]) -> u32 {
    let mut score = 0;
    let mut has_io = false;
    let mut has_random = false;
    let mut has_time = false;

    // Check return type
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(type_node) = child.child_by_field_name("type") {
                let type_text = type_node.utf8_text(source_code).unwrap_or("");
                if type_text.contains("void") && !type_text.contains('*') {
                    score += 4;
                }
            }
        }
    }

    // Check for I/O, randomness, time dependencies
    visit_node_observability(
        node,
        source_code,
        &mut has_io,
        &mut has_random,
        &mut has_time,
    );

    if has_io {
        score += 2;
    }
    if has_random {
        score += 3;
    }
    if has_time {
        score += 2;
    }

    score.min(10)
}

fn visit_node_observability(
    node: Node,
    source_code: &[u8],
    has_io: &mut bool,
    has_random: &mut bool,
    has_time: &mut bool,
) {
    if node.kind() == "call_expression" || node.kind() == "call" {
        if let Some(function) = node.child_by_field_name("function") {
            if let Ok(func_name) = function.utf8_text(source_code) {
                if matches!(
                    func_name,
                    "fopen"
                        | "fclose"
                        | "fread"
                        | "fwrite"
                        | "fprintf"
                        | "printf"
                        | "scanf"
                        | "puts"
                        | "open"   // Python
                        | "print"  // Python
                        | "input" // Python
                ) {
                    *has_io = true;
                }
                if matches!(func_name, "rand" | "srand" | "random") {
                    *has_random = true;
                }
                if matches!(func_name, "time" | "clock" | "gettimeofday") {
                    *has_time = true;
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_observability(child, source_code, has_io, has_random, has_time);
    }
}

/// Calculates documentation quality score (higher is better, reduces total difficulty)
const DOC_TAGS: &[(&str, i32)] = &[
    ("@intent",       5),
    ("@param",        2),
    ("@return",       2),
    ("@requires",     2),
    ("@ensures",      2),
    ("@side_effects", 2),
    ("@example",      3),
    ("@edge_cases",   2),
    ("@complexity",   2),
];

fn calculate_documentation_score(node: Node, source_code: &[u8]) -> i32 {
    // Look for comment immediately before the function.
    // C/C++ use "comment"; Rust uses "line_comment"/"block_comment".
    let Some(prev) = node.prev_sibling() else {
        return 0;
    };
    if !matches!(prev.kind(), "comment" | "line_comment" | "block_comment") {
        return 0;
    }
    let Ok(text) = prev.utf8_text(source_code) else {
        return 0;
    };

    if text.contains("/**") || text.contains("///") {
        let tag_score: i32 = DOC_TAGS
            .iter()
            .filter(|(tag, _)| text.contains(tag))
            .map(|(_, pts)| pts)
            .sum();
        (4 + tag_score).min(10)
    } else if text.starts_with("//") || text.starts_with("/*") {
        2
    } else {
        0
    }
}

/// Calculates AI Reasoning Difficulty (AIRD): a normalized 0-100 estimate of how
/// much reasoning effort an AI model requires to safely modify a function.
///
/// Higher = more AI reasoning required. Weights calibrated against corpus percentile
/// analysis and empirical validation (see AIM_BENCHMARK_RESULTS.md).
///
/// Inputs:
///   cognitive    - primary driver (reasoning steps needed)
///   sloc         - context window consumption
///   nesting      - structural confusion penalty
///   test_score   - iteration cost (low testability = more turns to validate)
///   doc_score    - documentation reduces ambiguity (negative contributor)
pub fn calculate_aird(
    cognitive: u32,
    sloc: u32,
    nesting: u32,
    test_score: i32,
    doc_score: i32,
    state_coupling: u32,
) -> u32 {
    let cognitive_norm = (cognitive as f64 / 75.0).min(1.0);
    let sloc_norm = (sloc as f64 / 200.0).min(1.0);
    let nesting_norm = (nesting as f64 / 8.0).min(1.0);
    let test_norm = (test_score.max(0) as f64 / 20.0).min(1.0);
    let doc_norm = (doc_score.max(0) as f64 / 10.0).min(1.0);
    // Normalize at 12: Clippy's too_many_arguments fires at 7, so 11-param functions land ~0.9.
    // Weight 10 dampens mechanical splits without inverting genuine wins.
    let coupling_norm = (state_coupling as f64 / 12.0).min(1.0);

    let raw = (cognitive_norm * 55.0)
        + (sloc_norm * 15.0)
        + (nesting_norm * 15.0)
        + (test_norm * 15.0)
        - (doc_norm * 15.0)
        + (coupling_norm * 10.0);

    raw.round().clamp(0.0, 100.0) as u32
}

/// Same formula as `calculate_aird` but without the per-input `.min(1.0)` caps.
/// Returns a raw f64 that can exceed 100 when inputs are above the normalization
/// denominators. Used for progress tracking: the capped AIRD stays pinned at 100
/// for very large functions, but the raw score decreases as you refactor — giving
/// a meaningful signal while the CI-gate score is still at its ceiling.
pub fn calculate_aird_raw(
    cognitive: u32,
    sloc: u32,
    nesting: u32,
    test_score: i32,
    doc_score: i32,
    state_coupling: u32,
) -> f64 {
    let cognitive_norm = cognitive as f64 / 75.0;
    let sloc_norm = sloc as f64 / 200.0;
    let nesting_norm = (nesting as f64 / 8.0).min(1.0); // nesting rarely exceeds cap; keep clamped
    let test_norm = (test_score.max(0) as f64 / 20.0).min(1.0);
    let doc_norm = (doc_score.max(0) as f64 / 10.0).min(1.0);
    let coupling_norm = (state_coupling as f64 / 12.0).min(1.0);

    ((cognitive_norm * 55.0)
        + (sloc_norm * 15.0)
        + (nesting_norm * 15.0)
        + (test_norm * 15.0)
        - (doc_norm * 15.0)
        + (coupling_norm * 10.0))
        .max(0.0)
}

/// Calculates AI Context Pressure (AICP): a normalized 0-100 estimate of how much
/// context an AI model must load to understand a function, independent of reasoning depth.
///
/// Higher = more tokens consumed gathering external context before the model can act.
/// Complements AIRD: a function can be cheap to load but hard to reason about, or
/// expensive to load but trivial once context is assembled.
///
/// Inputs:
///   external_calls - unique call targets not defined in this translation unit (p99=20)
///   sloc           - raw function size (context volume)
///   doc_score      - documentation reduces load (negative contributor)
pub fn calculate_aicp(external_calls: u32, sloc: u32, doc_score: i32) -> u32 {
    let ext_norm = (external_calls as f64 / 20.0).min(1.0);
    let sloc_norm = (sloc as f64 / 200.0).min(1.0);
    let doc_norm = (doc_score.max(0) as f64 / 10.0).min(1.0);

    let raw = (ext_norm * 60.0) + (sloc_norm * 40.0) - (doc_norm * 15.0);

    raw.round().clamp(0.0, 100.0) as u32
}

/// Counts the explicit (non-self) parameters declared in a function signature.
///
/// Language coverage:
/// - Rust `function_item`: counts `parameter` children of the `parameters` node;
///   `self_parameter` nodes are excluded.
/// - Python `function_definition`: counts params that are not `self` or `cls`.
/// - C/C++ `function_definition`/`function_declaration`: counts `parameter_declaration`
///   children of the `parameter_list` found inside the declarator subtree.
/// - JavaScript: counts children of the `parameters` (formal_parameters) node.
// Counts children of `params_node` whose kind is in `kinds`.
fn count_param_children(params_node: Node, kinds: &[&str]) -> u32 {
    let mut cursor = params_node.walk();
    params_node
        .children(&mut cursor)
        .filter(|c| kinds.contains(&c.kind()))
        .count() as u32
}

// Gets `node`'s `parameters` named field and counts children matching `kinds`; 0 if absent.
fn count_params_in_field(node: Node, kinds: &[&str]) -> u32 {
    node.child_by_field_name("parameters")
        .map(|p| count_param_children(p, kinds))
        .unwrap_or(0)
}

// Python/PHP `function_definition` parameters: counts all param kinds, excluding self/cls identifiers.
fn count_python_params(params: Node, source_code: &[u8]) -> u32 {
    let mut count = 0u32;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let text = child.utf8_text(source_code).unwrap_or("");
                if text != "self" && text != "cls" {
                    count += 1;
                }
            }
            "typed_parameter"
            | "default_parameter"
            | "typed_default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern"
            | "simple_parameter"
            | "variadic_parameter"
            | "property_promotion_parameter"
            | "parameter" => count += 1,
            _ => {}
        }
    }
    count
}

// Fortran: finds the header child of `header_kind` and counts its `parameters` identifiers.
// module_procedure and program nodes have no dummy argument list and return 0.
fn count_fortran_params(node: Node, header_kind: &str) -> u32 {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == header_kind {
            return child
                .child_by_field_name("parameters")
                .map(|p| count_param_children(p, &["identifier"]))
                .unwrap_or(0);
        }
    }
    0
}

// Ada `parameter_specification`: counts identifier children before the `:` type separator.
// `X, Y : Integer` → 2 params.
fn count_ada_param_spec(spec: Node, source_code: &[u8]) -> u32 {
    let mut count = 0u32;
    let mut cursor = spec.walk();
    for child in spec.children(&mut cursor) {
        if !child.is_named() && child.kind() == ":" {
            break;
        }
        if child.is_named() && child.kind() == "identifier" {
            count += 1;
        }
    }
    // source_code unused here but kept for signature consistency with callers
    let _ = source_code;
    count
}

// Ada `subprogram_body` / `expression_function_declaration`: walks to the specification,
// finds formal_part, and sums counts across all parameter_specification children.
fn count_ada_params(node: Node, source_code: &[u8]) -> u32 {
    let mut c1 = node.walk();
    let Some(spec) = node.children(&mut c1)
        .find(|c| matches!(c.kind(), "function_specification" | "procedure_specification"))
    else {
        return 0;
    };
    let mut c2 = spec.walk();
    let Some(formal_part) = spec.children(&mut c2).find(|c| c.kind() == "formal_part") else {
        return 0;
    };
    let mut c3 = formal_part.walk();
    formal_part
        .children(&mut c3)
        .filter(|c| c.kind() == "parameter_specification")
        .map(|ps| count_ada_param_spec(ps, source_code))
        .sum()
}

fn count_explicit_params(node: Node, source_code: &[u8]) -> u32 {
    match node.kind() {
        // Rust: named 'parameters' field; 'self_parameter' is excluded (not a real arg).
        "function_item" => count_params_in_field(node, &["parameter"]),
        // Python/PHP/Scala/C: Python and PHP have a 'parameters' field; C uses a declarator chain.
        "function_definition" => {
            if let Some(params) = node.child_by_field_name("parameters") {
                count_python_params(params, source_code)
            } else {
                count_c_params_in_subtree(node, source_code)
            }
        }
        // JS/TS/Go/Kotlin/Swift/C++: four layout variants tried in order.
        "function_declaration" => {
            // JS/TS/Go: direct 'parameters' field
            if let Some(params) = node.child_by_field_name("parameters") {
                return count_param_children(params, &[
                    "identifier", "required_parameter", "optional_parameter",
                    "rest_pattern", "assignment_pattern",
                    "parameter_declaration", "variadic_parameter_declaration",
                ]);
            }
            // Kotlin: wrapped in a 'function_value_parameters' child (no field name)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_value_parameters" {
                    return count_param_children(child, &["parameter"]);
                }
            }
            // Swift: 'parameter' nodes are direct children (no wrapper)
            let swift_count = {
                let mut cursor = node.walk();
                node.children(&mut cursor).filter(|c| c.kind() == "parameter").count() as u32
            };
            if swift_count > 0 {
                return swift_count;
            }
            // C/C++: no 'parameters' field; drill into the declarator chain
            count_c_params_in_subtree(node, source_code)
        }
        // Swift: init_declaration — same direct-children layout as function_declaration.
        "init_declaration" => {
            let mut cursor = node.walk();
            node.children(&mut cursor).filter(|c| c.kind() == "parameter").count() as u32
        }
        // Go method/closure, Java method/constructor, C# method/constructor/local fn, PHP method.
        "method_declaration" | "func_literal" | "constructor_declaration"
        | "local_function_statement" => count_params_in_field(node, &[
            "parameter_declaration", "variadic_parameter_declaration",
            "formal_parameter", "spread_parameter",
            "parameter",
            "simple_parameter", "variadic_parameter", "property_promotion_parameter",
        ]),
        // JS/TS method/function/arrow/generator. PHP arrow `fn($x) => expr` uses arrow_function.
        "method_definition"
        | "function_expression"
        | "arrow_function"
        | "generator_function_declaration"
        | "generator_function" => {
            // Single unparenthesised arrow-function parameter uses the singular 'parameter' field.
            if node.kind() == "arrow_function" && node.child_by_field_name("parameter").is_some() {
                return 1;
            }
            count_params_in_field(node, &[
                "identifier", "required_parameter", "optional_parameter",
                "rest_pattern", "assignment_pattern",
                "simple_parameter", "variadic_parameter", "property_promotion_parameter",
            ])
        }
        // Fortran: each parameter is an identifier in the header statement's 'parameters' field.
        "function" => count_fortran_params(node, "function_statement"),
        "subroutine" => count_fortran_params(node, "subroutine_statement"),
        // Ada: walk specification → formal_part → parameter_specification.
        "subprogram_body" | "expression_function_declaration" => count_ada_params(node, source_code),
        _ => 0,
    }
}

fn count_c_params_in_subtree(node: Node, source_code: &[u8]) -> u32 {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_list" {
            let mut inner = child.walk();
            return child
                .children(&mut inner)
                .filter(|c| c.kind() == "parameter_declaration")
                .count() as u32;
        }
        if child.kind().contains("declarator") {
            let n = count_c_params_in_subtree(child, source_code);
            if n > 0 {
                return n;
            }
        }
    }
    0
}

// Returns the name from `node.child_by_field_name(name_field)` when
// `node.child_by_field_name(object_field)` equals `self_kw`.
fn try_extract_named_field<'a>(
    node: Node,
    source_code: &'a [u8],
    object_field: &str,
    self_kw: &str,
    name_field: &str,
) -> Option<&'a str> {
    let obj = node.child_by_field_name(object_field)?;
    if obj.utf8_text(source_code).ok()? != self_kw {
        return None;
    }
    node.child_by_field_name(name_field)?.utf8_text(source_code).ok()
}

// Handles `navigation_expression` for both Kotlin (`this.field`) and Swift (`self.field`).
// Kotlin AST: navigation_expression { this_expression, <identifier> }
// Swift AST:  navigation_expression { self_expression, navigation_suffix { <identifier> } }
fn extract_navigation_self_field<'a>(node: Node, source_code: &'a [u8]) -> Option<&'a str> {
    let first = node.named_child(0)?;
    match first.kind() {
        "this_expression" => node.named_child(1)?.utf8_text(source_code).ok(),
        "self_expression" => {
            let suffix = node.named_child(1)?;
            if suffix.kind() == "navigation_suffix" {
                suffix.named_child(0)?.utf8_text(source_code).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn collect_self_fields_recursive(node: Node, source_code: &[u8], fields: &mut HashSet<String>) {
    let extracted: Option<&str> = match node.kind() {
        "field_expression" => try_extract_named_field(node, source_code, "value", "self", "field"),
        "attribute" => try_extract_named_field(node, source_code, "object", "self", "attribute"),
        "member_expression" => try_extract_named_field(node, source_code, "object", "this", "property"),
        "field_access" => try_extract_named_field(node, source_code, "object", "this", "field"),
        "member_access_expression" => {
            // C#: expression == "this"; PHP: object == "$this" (different field names, no ambiguity)
            try_extract_named_field(node, source_code, "expression", "this", "name")
                .or_else(|| try_extract_named_field(node, source_code, "object", "$this", "name"))
        }
        "navigation_expression" => extract_navigation_self_field(node, source_code),
        _ => None,
    };
    if let Some(name) = extracted {
        fields.insert(name.to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_self_fields_recursive(child, source_code, fields);
    }
}

/// Estimates the state surface area a caller must hold to safely modify a function:
/// explicit parameter count plus the count of distinct `self.field` accesses in the body.
///
/// This catches the two coupling patterns that raw AIRD misses:
/// - Wide parameter lists created when a large function is mechanically split.
/// - God-struct methods where coupling hides behind `self` with low arity.
///
/// Normalization and weight in `calculate_aird` are calibrated to dampen, not invert:
/// a split that lowers cognitive complexity still improves AIRD, but less so when it
/// forces the caller to track many more state values.
pub fn calculate_state_coupling(node: Node, source_code: &[u8]) -> u32 {
    let explicit_params = count_explicit_params(node, source_code);
    let mut self_fields: HashSet<String> = HashSet::new();
    collect_self_fields_recursive(node, source_code, &mut self_fields);
    explicit_params + self_fields.len() as u32
}

#[cfg(test)]
mod aird_tests {
    use super::*;

    #[test]
    fn test_aird_trivial_function() {
        let aird = calculate_aird(1, 5, 1, 2, 8, 0);
        assert!(
            aird < 15,
            "trivial function AIRD should be < 15, got {}",
            aird
        );
    }

    #[test]
    fn test_aird_complex_function() {
        let aird = calculate_aird(80, 200, 7, 15, 0, 0);
        assert!(
            aird > 70,
            "complex function AIRD should be > 70, got {}",
            aird
        );
    }

    #[test]
    fn test_aird_doc_reduces_score() {
        let without_docs = calculate_aird(20, 40, 3, 15, 0, 0);
        let with_docs = calculate_aird(20, 40, 3, 15, 10, 0);
        assert!(
            with_docs < without_docs,
            "documentation should reduce AIRD: {} vs {}",
            with_docs,
            without_docs
        );
    }

    #[test]
    fn test_aird_clamps_to_100() {
        let aird = calculate_aird(1000, 1000, 1000, 1000, 0, 1000);
        assert_eq!(aird, 100);
    }

    #[test]
    fn test_aird_clamps_to_0() {
        let aird = calculate_aird(0, 0, 0, 0, 10, 0);
        assert_eq!(aird, 0);
    }

    #[test]
    fn test_aird_raw_exceeds_capped_for_large_inputs() {
        // cognitive=150 (2× cap), sloc=400 (2× cap); nesting/test/doc/coupling at neutral zero.
        // Capped: (1.0*55) + (1.0*15) = 70. Raw: (2.0*55) + (2.0*15) = 140.
        let capped = calculate_aird(150, 400, 0, 0, 0, 0);
        let raw = calculate_aird_raw(150, 400, 0, 0, 0, 0);
        assert_eq!(capped, 70, "capped AIRD should be 70 for these inputs");
        assert!(raw > capped as f64, "raw AIRD ({}) must exceed capped ({}) when inputs are above caps", raw, capped);
    }

    #[test]
    fn test_aird_raw_tracks_progress_above_cap() {
        // Simulates two snapshots of the same function mid-refactor.
        // Both are still above the normalization caps so capped AIRD is identical,
        // but raw AIRD must decrease to prove progress is measurable.
        // cognitive 300→150 (both above 75), sloc 400→250 (both above 200).
        let raw_before  = calculate_aird_raw(300, 400, 0, 0, 0, 0);
        let raw_after   = calculate_aird_raw(150, 250, 0, 0, 0, 0);
        let gate_before = calculate_aird(300, 400, 0, 0, 0, 0);
        let gate_after  = calculate_aird(150, 250, 0, 0, 0, 0);
        assert_eq!(gate_before, gate_after, "capped AIRD should be identical when both inputs remain above caps");
        assert!(raw_after < raw_before, "raw AIRD must decrease as inputs are reduced: {} -> {}", raw_before, raw_after);
    }

    #[test]
    fn test_aird_raw_equals_capped_below_normalization_caps() {
        // When inputs are within the normalization denominators the raw and capped scores
        // must agree (within rounding) — raw only diverges above the caps.
        let capped = calculate_aird(30, 80, 2, 8, 3, 0) as f64;
        let raw    = calculate_aird_raw(30, 80, 2, 8, 3, 0);
        assert!(
            (capped - raw).abs() < 1.0,
            "raw and capped AIRD should agree below normalization caps: {} vs {}",
            capped, raw
        );
    }
}

#[cfg(test)]
mod aicp_tests {
    use super::*;

    #[test]
    fn test_aicp_trivial_function() {
        let aicp = calculate_aicp(2, 30, 0);
        assert!(
            aicp < 20,
            "trivial function AICP should be < 20, got {}",
            aicp
        );
    }

    #[test]
    fn test_aicp_high_pressure_function() {
        let aicp = calculate_aicp(25, 300, 0);
        assert!(
            aicp > 70,
            "high-pressure function AICP should be > 70, got {}",
            aicp
        );
    }

    #[test]
    fn test_aicp_doc_reduces_score() {
        let without_docs = calculate_aicp(10, 100, 0);
        let with_docs = calculate_aicp(10, 100, 10);
        assert!(
            with_docs < without_docs,
            "documentation should reduce AICP: {} vs {}",
            with_docs,
            without_docs
        );
    }

    #[test]
    fn test_aicp_clamps_to_100() {
        let aicp = calculate_aicp(1000, 1000, 0);
        assert_eq!(aicp, 100);
    }

    #[test]
    fn test_aicp_clamps_to_0() {
        let aicp = calculate_aicp(0, 0, 10);
        assert_eq!(aicp, 0);
    }
}

#[cfg(test)]
mod state_coupling_tests {
    use super::*;

    fn parse_rust(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    fn rust_coupling(code: &str) -> u32 {
        let tree = parse_rust(code);
        let root = tree.root_node();
        let mut result = 0u32;
        fn find_fn(node: tree_sitter::Node<'_>, src: &[u8], out: &mut u32) {
            if node.kind() == "function_item" {
                *out = calculate_state_coupling(node, src);
                return;
            }
            let mut c = node.walk();
            for child in node.children(&mut c) {
                find_fn(child, src, out);
            }
        }
        find_fn(root, code.as_bytes(), &mut result);
        result
    }

    #[test]
    fn test_no_params_no_self() {
        let coupling = rust_coupling("fn f() { let x = 1; }");
        assert_eq!(coupling, 0);
    }

    #[test]
    fn test_counts_explicit_params_excludes_self() {
        // &self is a self_parameter, not counted; a and b are regular parameters
        let coupling =
            rust_coupling("struct S; impl S { fn f(&self, a: i32, b: i32) -> i32 { a + b } }");
        assert_eq!(coupling, 2);
    }

    #[test]
    fn test_self_field_accesses_counted() {
        let coupling = rust_coupling(
            "struct S { x: i32, y: i32 } impl S { fn f(&self) { let _ = self.x + self.y; } }",
        );
        assert_eq!(coupling, 2);
    }

    #[test]
    fn test_duplicate_self_field_counted_once() {
        let coupling = rust_coupling(
            "struct S { x: i32 } impl S { fn f(&self) -> i32 { self.x + self.x + self.x } }",
        );
        // 'x' accessed multiple times but only one distinct field
        assert_eq!(coupling, 1);
    }

    #[test]
    fn test_params_plus_self_fields() {
        let coupling = rust_coupling(
            "struct S { a: i32, b: i32 } impl S { fn f(&self, x: i32, y: i32) { \
             let _ = self.a + self.b + x + y; } }",
        );
        // 2 explicit params + 2 distinct self fields
        assert_eq!(coupling, 4);
    }

    #[test]
    fn test_high_arity_dampens_aird() {
        // Simulates an 11-param mechanical split: AIRD should be higher with coupling than without.
        // cognitive=30 sloc=60 nesting=2 test=5 doc=0 — low complexity but wide signature
        let aird_no_coupling = calculate_aird(30, 60, 2, 5, 0, 0);
        let aird_with_coupling = calculate_aird(30, 60, 2, 5, 0, 11);
        assert!(
            aird_with_coupling > aird_no_coupling,
            "high arity should raise AIRD: {} vs {}",
            aird_with_coupling,
            aird_no_coupling
        );
    }

    #[test]
    fn test_genuine_win_still_positive() {
        // str31_violation: 4 params, dropped from cognitive ~80 to ~20 — should remain improved.
        let aird_before = calculate_aird(80, 150, 5, 12, 0, 2);
        let aird_after = calculate_aird(20, 40, 2, 5, 0, 4);
        assert!(
            aird_after < aird_before,
            "genuine simplification should still lower AIRD: {} -> {}",
            aird_before,
            aird_after
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Tree;

    fn parse_c_function(code: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into()).unwrap();
        parser.parse(code, None).unwrap()
    }

    fn parse_cpp_function(code: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();
        parser.parse(code, None).unwrap()
    }

    // ---- C++ parser/discovery tests ----

    #[test]
    fn test_cpp_namespace_function() {
        let code = r#"
        namespace myns {
            void func() {
                if (x) { }
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // Same as equivalent C function: base 1 + 1 if = 2
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_class_method() {
        let code = r#"
        class Foo {
            void method() {
                if (x) { }
            }
        };
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_template_function() {
        let code = r#"
        template <typename T>
        T add(T a, T b) {
            if (a > b) { return a; }
            return b;
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // base 1 + 1 if = 2
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_extern_c_function() {
        let code = r#"
        extern "C" {
            void c_func() {
                if (x) { }
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    // ---- C++ range-for tests ----

    #[test]
    fn test_cpp_range_for_mccabe() {
        let code = r#"
        void func() {
            for (auto& x : items) {
                x++;
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // base 1 + 1 for_range_loop = 2
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_range_for_cognitive() {
        let code = r#"
        void func() {
            for (auto& x : items) {
                if (x > 0) { }
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // for_range_loop: +1 (nesting 0)
        // if inside loop: +1 (base) +1 (nesting=1) = +2
        // Total: 3
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_cpp_range_for_nesting() {
        let code = r#"
        void func() {
            for (auto& x : items) {
                for (auto& y : other) {
                    x++;
                }
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        assert_eq!(calculate_nesting_depth(node), 2);
    }

    // ---- C++ lambda tests ----

    #[test]
    fn test_cpp_lambda_cognitive_nesting() {
        let code = r#"
        void func() {
            auto f = [](int x) {
                if (x > 0) { }
            };
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // lambda: nesting increment (no +1 itself)
        // if inside lambda: +1 (base) +1 (nesting=1) = +2
        // Total: 2
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_lambda_nesting_depth() {
        let code = r#"
        void func() {
            auto f = [](int x) {
                if (x > 0) { }
            };
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // lambda: depth 1, if inside: depth 2
        assert_eq!(calculate_nesting_depth(node), 2);
    }

    // ---- C++ try/catch tests ----

    #[test]
    fn test_cpp_try_catch_cognitive() {
        let code = r#"
        void func() {
            try {
                if (x) { }
            } catch (const std::exception& e) {
                int y = 0;
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // Per SonarSource: try has NO cost, NO nesting increment
        // if inside try: +1 (base) +0 (nesting=0) = +1
        // catch: +1 (base) +0 (nesting=0) = +1
        // Total: 2
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cpp_try_nesting_depth() {
        let code = r#"
        void func() {
            try {
                if (x) { }
            } catch (...) { }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // try has no nesting contribution; if: depth 1, catch: depth 1, max=1
        assert_eq!(calculate_nesting_depth(node), 1);
    }

    #[test]
    fn test_cpp_try_catch_nested_cognitive() {
        let code = r#"
        void func() {
            try {
            } catch (...) {
                if (y) { }
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // catch: +1 (base) +0 (nesting=0) = +1
        // if inside catch: +1 (base) +1 (nesting=1) = +2
        // Total: 3
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    // ---- C++ throw tests ----

    #[test]
    fn test_cpp_throw_mccabe() {
        let code = r#"
        void func() {
            if (x) {
                throw std::runtime_error("err");
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // base 1 + 1 if + 1 throw = 3
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_cpp_throw_cognitive() {
        let code = r#"
        void func() {
            throw std::runtime_error("err");
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        // throw: flat +1
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    // ---- C++ new/delete ABC tests ----

    #[test]
    fn test_cpp_new_delete_abc() {
        let code = r#"
        void func() {
            int* p = new int(42);
            delete p;
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        let abc = calculate_abc_complexity(node, code.as_bytes());
        // new: +1 branch, delete: +1 branch
        assert_eq!(abc.branches, 2);
    }

    #[test]
    fn test_cpp_range_for_abc() {
        let code = r#"
        void func() {
            for (auto& x : items) {
                x++;
            }
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        let abc = calculate_abc_complexity(node, code.as_bytes());
        // for_range_loop: +1 condition, x++: +1 assignment
        assert_eq!(abc.conditions, 1);
        assert_eq!(abc.assignments, 1);
    }

    #[test]
    fn test_cpp_throw_abc() {
        let code = r#"
        void func() {
            throw 42;
        }
        "#;
        let tree = parse_cpp_function(code);
        let node = tree.root_node();
        let abc = calculate_abc_complexity(node, code.as_bytes());
        // throw: +1 branch
        assert_eq!(abc.branches, 1);
    }

    // ---- Existing C tests ----

    #[test]
    fn test_simple_function_mccabe() {
        let code = r#"
        void simple() {
            int x = 1;
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // Simple function with no branches should have complexity 1
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_if_statement_mccabe() {
        let code = r#"
        void with_if() {
            if (1) {
                int x = 1;
            }
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // One if statement increases complexity to 2
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_simple_function_cognitive() {
        let code = r#"
        void simple() {
            int x = 1;
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // Simple function with no branches should have complexity 0
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 0);
    }

    #[test]
    fn test_nesting_depth_simple_if() {
        let code = r#"
        void func() {
            if (1) {
                int x = 1;
            }
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // One if statement = nesting depth 1
        assert_eq!(calculate_nesting_depth(node), 1);
    }

    #[test]
    fn test_nesting_depth_nested_if() {
        let code = r#"
        void func() {
            if (1) {
                if (2) {
                    int x = 1;
                }
            }
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // Two nested ifs = nesting depth 2
        assert_eq!(calculate_nesting_depth(node), 2);
    }

    #[test]
    fn test_nesting_depth_no_control_flow() {
        let code = r#"
        void func() {
            int x = 1;
            int y = 2;
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // No control structures = nesting depth 0
        assert_eq!(calculate_nesting_depth(node), 0);
    }

    #[test]
    fn test_cognitive_else_if_nested() {
        let code = r#"
        void func() {
            if (a) {
            } else if (b) {
                if (c) {
                }
            }
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // if(a): +1 (nesting 0)
        // else if(b): +1 (else-if flat)
        // if(c) inside else-if body: +1 (base) +1 (nesting=1) = +2
        // Total: 4
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 4);
    }

    #[test]
    fn test_nested_if_cognitive() {
        let code = r#"
        void nested() {
            if (1) {
                if (2) {
                    int x = 1;
                }
            }
        }
        "#;
        let tree = parse_c_function(code);
        let node = tree.root_node();
        // Outer if: +1, inner if: +1 (base) +1 (nesting) = 3
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    // ---- Rust parser/metric tests ----

    fn parse_rust(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        parser.parse(code, None).unwrap()
    }

    // McCabe

    #[test]
    fn test_rust_simple_mccabe() {
        let code = "fn simple() { let x = 1; }";
        let tree = parse_rust(code);
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    #[test]
    fn test_rust_if_expression_mccabe() {
        let code = "fn f(x: i32) { if x > 0 { let y = 1; } }";
        let tree = parse_rust(code);
        // base 1 + 1 if_expression = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_rust_while_expression_mccabe() {
        let code = "fn f() { let mut i = 0; while i < 10 { i += 1; } }";
        let tree = parse_rust(code);
        // base 1 + 1 while_expression = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_rust_for_expression_mccabe() {
        let code = "fn f(items: &[i32]) { for item in items { let _ = item; } }";
        let tree = parse_rust(code);
        // base 1 + 1 for_expression = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_rust_loop_expression_mccabe() {
        let code = "fn f() { loop { break; } }";
        let tree = parse_rust(code);
        // base 1 + 1 loop_expression = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_rust_match_expression_mccabe() {
        let code = "fn f(x: i32) -> i32 { match x { 0 => 0, _ => 1 } }";
        let tree = parse_rust(code);
        // base 1 + 1 match (arms don't count) = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    // Cognitive

    #[test]
    fn test_rust_if_expression_cognitive() {
        let code = "fn f(x: i32) { if x > 0 { let y = 1; } }";
        let tree = parse_rust(code);
        // if_expression at nesting 0: +1
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    #[test]
    fn test_rust_nested_if_cognitive() {
        let code = "fn f(x: i32, y: i32) { if x > 0 { if y > 0 { let z = 1; } } }";
        let tree = parse_rust(code);
        // outer if: +1 (nesting 0); inner if: +1+1 (nesting 1) = 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_rust_else_if_cognitive() {
        let code = "fn f(x: i32) { if x > 0 { } else if x < 0 { } }";
        let tree = parse_rust(code);
        // if: +1; else-if: +1 (flat, no nesting penalty) = 2
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_rust_match_cognitive() {
        let code = "fn f(x: i32) { match x { 0 => {}, _ => {} } }";
        let tree = parse_rust(code);
        // match at nesting 0: +1+0 = 1
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    #[test]
    fn test_rust_closure_nesting_cognitive() {
        let code = "fn f() { let _g = |x: i32| { if x > 0 { 1 } else { 0 } }; }";
        let tree = parse_rust(code);
        // closure: +0 base, enters nesting_level 1
        // if inside closure (nesting=1): +1+1=2; else: +1
        // Total: 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_rust_for_with_if_cognitive() {
        let code = "fn f(items: &[i32]) { for item in items { if *item > 0 { } } }";
        let tree = parse_rust(code);
        // for at nesting 0: +1; if inside for (nesting=1): +1+1=2 → total 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    // Nesting depth

    #[test]
    fn test_rust_nesting_simple_if() {
        let code = "fn f(x: i32) { if x > 0 { let y = 1; } }";
        let tree = parse_rust(code);
        assert_eq!(calculate_nesting_depth(tree.root_node()), 1);
    }

    #[test]
    fn test_rust_nesting_nested_loops() {
        let code = "fn f(a: &[i32], b: &[i32]) { for x in a { for _y in b { let _ = x; } } }";
        let tree = parse_rust(code);
        assert_eq!(calculate_nesting_depth(tree.root_node()), 2);
    }

    #[test]
    fn test_rust_nesting_match() {
        let code = "fn f(x: i32) { match x { 0 => {}, _ => {} } }";
        let tree = parse_rust(code);
        assert_eq!(calculate_nesting_depth(tree.root_node()), 1);
    }

    #[test]
    fn test_rust_nesting_closure_with_if() {
        let code = "fn f() { let _g = |x: i32| { if x > 0 { 1 } else { 0 } }; }";
        let tree = parse_rust(code);
        // closure = depth 1, if inside closure = depth 2
        assert_eq!(calculate_nesting_depth(tree.root_node()), 2);
    }

    // ABC

    #[test]
    fn test_rust_compound_assignment_abc() {
        let code = "fn f() { let mut x = 0; x += 1; x -= 2; }";
        let tree = parse_rust(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.assignments, 2, "two compound_assignment_expr nodes");
    }

    #[test]
    fn test_rust_if_condition_abc() {
        let code = "fn f(x: i32) { if x > 0 { } }";
        let tree = parse_rust(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    #[test]
    fn test_rust_for_condition_abc() {
        let code = "fn f(items: &[i32]) { for _ in items { } }";
        let tree = parse_rust(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    #[test]
    fn test_rust_match_condition_abc() {
        let code = "fn f(x: i32) { match x { 0 => {}, _ => {} } }";
        let tree = parse_rust(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    // Return count

    #[test]
    fn test_rust_return_expression() {
        let code = "fn f(x: i32) -> i32 { if x > 0 { return x; } return 0; }";
        let tree = parse_rust(code);
        assert_eq!(calculate_return_count(tree.root_node()), 2);
    }

    #[test]
    fn test_rust_implicit_return_not_counted() {
        let code = "fn f(x: i32) -> i32 { x + 1 }";
        let tree = parse_rust(code);
        // Implicit return (expression tail, no return keyword) counts 0
        assert_eq!(calculate_return_count(tree.root_node()), 0);
    }

    // ---- Python parser/metric tests ----

    fn parse_python(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    // McCabe

    #[test]
    fn test_python_simple_mccabe() {
        let code = "def f(x):\n    return x + 1\n";
        let tree = parse_python(code);
        // base 1, no branches
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    #[test]
    fn test_python_if_mccabe() {
        let code = "def f(x):\n    if x > 0:\n        return x\n    return 0\n";
        let tree = parse_python(code);
        // base 1 + 1 if = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_elif_mccabe() {
        let code = "def f(x, y):\n    if x > 0:\n        return x\n    elif y > 0:\n        return y\n    return 0\n";
        let tree = parse_python(code);
        // base 1 + 1 if + 1 elif = 3
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_python_for_mccabe() {
        let code = "def f(items):\n    for item in items:\n        pass\n";
        let tree = parse_python(code);
        // base 1 + 1 for = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_while_mccabe() {
        let code = "def f(x):\n    while x > 0:\n        x -= 1\n";
        let tree = parse_python(code);
        // base 1 + 1 while = 2
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_except_mccabe() {
        let code = "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n";
        let tree = parse_python(code);
        // base 1 + 1 except = 2 (try itself has no cost)
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_boolean_and_mccabe() {
        let code = "def f(a, b):\n    if a > 0 and b > 0:\n        return True\n    return False\n";
        let tree = parse_python(code);
        // base 1 + 1 if + 1 and = 3
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_python_boolean_or_mccabe() {
        let code = "def f(a, b, c):\n    if a < 0 or b < 0 or c < 0:\n        return True\n    return False\n";
        let tree = parse_python(code);
        // base 1 + 1 if + 1 or + 1 or = 4
        assert_eq!(
            calculate_mccabe_complexity(tree.root_node(), code.as_bytes()),
            4
        );
    }

    // Cognitive

    #[test]
    fn test_python_simple_cognitive() {
        let code = "def f(x):\n    return x + 1\n";
        let tree = parse_python(code);
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            0
        );
    }

    #[test]
    fn test_python_if_cognitive() {
        let code = "def f(x):\n    if x > 0:\n        return x\n    return 0\n";
        let tree = parse_python(code);
        // if at nesting=0: +1
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    #[test]
    fn test_python_elif_cognitive() {
        let code = "def f(x, y):\n    if x > 0:\n        return x\n    elif y > 0:\n        return y\n    else:\n        return 0\n";
        let tree = parse_python(code);
        // if: +1, elif: +1 flat, else: +1 flat → 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_python_nested_if_cognitive() {
        let code =
            "def f(x, y):\n    if x > 0:\n        if y > 0:\n            return 1\n    return 0\n";
        let tree = parse_python(code);
        // outer if: +1 (nesting=0); inner if: +1+1=2 → total 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_python_for_with_if_cognitive() {
        let code =
            "def f(items):\n    for item in items:\n        if item > 0:\n            pass\n";
        let tree = parse_python(code);
        // for at nesting=0: +1; if inside for (nesting=1): +1+1=2 → total 3
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            3
        );
    }

    #[test]
    fn test_python_and_cognitive() {
        let code = "def f(a, b):\n    if a > 0 and b > 0:\n        return True\n    return False\n";
        let tree = parse_python(code);
        // if: +1; and (new op, parent=None): +1 → total 2
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_and_chain_cognitive() {
        let code = "def f(a, b, c):\n    if a > 0 and b > 0 and c > 0:\n        return True\n    return False\n";
        let tree = parse_python(code);
        // if: +1; first and: +1; second and (same op, no penalty): +0 → total 2
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            2
        );
    }

    #[test]
    fn test_python_except_cognitive() {
        let code = "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n";
        let tree = parse_python(code);
        // try: no cost; except at nesting=0: +1+0=1 → total 1
        assert_eq!(
            calculate_cognitive_complexity(tree.root_node(), code.as_bytes()),
            1
        );
    }

    // Nesting depth

    #[test]
    fn test_python_nesting_simple_if() {
        let code = "def f(x):\n    if x > 0:\n        return x\n";
        let tree = parse_python(code);
        assert_eq!(calculate_nesting_depth(tree.root_node()), 1);
    }

    #[test]
    fn test_python_nesting_nested_for() {
        let code = "def f(a, b):\n    for x in a:\n        for y in b:\n            pass\n";
        let tree = parse_python(code);
        assert_eq!(calculate_nesting_depth(tree.root_node()), 2);
    }

    #[test]
    fn test_python_nesting_lambda() {
        let code = "def f():\n    fn = lambda x: x * 2\n    return fn\n";
        let tree = parse_python(code);
        // lambda: depth 1 (keyword token doesn't count due to is_named() guard)
        assert_eq!(calculate_nesting_depth(tree.root_node()), 1);
    }

    // ABC

    #[test]
    fn test_python_assignment_abc() {
        let code = "def f():\n    x = 1\n    y = 2\n";
        let tree = parse_python(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.assignments, 2, "two assignments");
    }

    #[test]
    fn test_python_augmented_assignment_abc() {
        let code = "def f():\n    x = 0\n    x += 1\n    x -= 2\n";
        let tree = parse_python(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        // 1 assignment + 2 augmented = 3
        assert_eq!(abc.assignments, 3);
    }

    #[test]
    fn test_python_call_abc() {
        let code = "def f():\n    foo()\n    bar()\n";
        let tree = parse_python(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.branches, 2, "two function calls");
    }

    #[test]
    fn test_python_if_condition_abc() {
        let code = "def f(x):\n    if x > 0:\n        pass\n";
        let tree = parse_python(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    #[test]
    fn test_python_boolean_operator_abc() {
        let code = "def f(a, b):\n    if a > 0 and b > 0:\n        pass\n";
        let tree = parse_python(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        // 1 if + 1 and = 2 conditions
        assert_eq!(abc.conditions, 2);
    }

    // Return count

    #[test]
    fn test_python_return_count() {
        let code = "def f(x):\n    if x > 0:\n        return x\n    return 0\n";
        let tree = parse_python(code);
        assert_eq!(calculate_return_count(tree.root_node()), 2);
    }

    // SLOC

    #[test]
    fn test_python_sloc_skips_hash_comments() {
        let code = "def f(x):\n    # this is a comment\n    return x + 1\n";
        let tree = parse_python(code);
        // def line + return line = 2; comment skipped
        let sloc = calculate_sloc_python(tree.root_node(), code.as_bytes());
        assert!(sloc <= 3, "comment line should be excluded, got {}", sloc);
    }

    #[test]
    fn test_python_sloc_c_sloc_counts_hash_preprocessor() {
        let code = "#include <stdio.h>\nint f() { return 0; }\n";
        let tree = parse_c_function(code);
        // In C, # is a preprocessor directive (IS code), not a comment
        let sloc = calculate_sloc(tree.root_node(), code.as_bytes());
        assert!(sloc >= 1, "C # line should be counted as SLOC");
    }

    // ---- JavaScript tests ----

    fn parse_js_function(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    fn js_func_node(tree: &tree_sitter::Tree) -> tree_sitter::Node<'_> {
        fn find_func(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if matches!(
                node.kind(),
                "function_declaration"
                    | "function_expression"
                    | "method_definition"
                    | "generator_function_declaration"
            ) {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(found) = find_func(child) {
                    return Some(found);
                }
            }
            None
        }
        find_func(tree.root_node()).expect("no function node found")
    }

    #[test]
    fn test_js_simple_mccabe() {
        let code = "function f() { return 1; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_js_if_mccabe() {
        let code = "function f(x) { if (x > 0) { return 1; } return 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_for_in_mccabe() {
        let code = "function f(obj) { for (const k in obj) { console.log(k); } }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_for_of_mccabe() {
        let code = "function f(arr) { for (const x of arr) { console.log(x); } }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_ternary_mccabe() {
        let code = "function f(x) { return x > 0 ? 1 : 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_nullish_mccabe() {
        let code = "function f(x) { return x ?? 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_logical_and_mccabe() {
        let code = "function f(a, b) { return a && b; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_simple_cognitive() {
        let code = "function f() { return 1; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 0);
    }

    #[test]
    fn test_js_if_cognitive() {
        let code = "function f(x) { if (x > 0) { return 1; } return 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_js_for_of_cognitive() {
        let code = "function f(arr) { for (const x of arr) { if (x > 0) { return x; } } }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        // for_of: +1+0=1, if inside for: +1+1=2 → total 3
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_js_nullish_cognitive() {
        let code = "function f(a, b) { return a ?? b; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_js_nesting_simple_if() {
        let code = "function f(x) { if (x > 0) { return 1; } }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_nesting_depth(node), 1);
    }

    #[test]
    fn test_js_nesting_for_in_with_if() {
        let code = "function f(obj) { for (const k in obj) { if (obj[k]) { return k; } } }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_nesting_depth(node), 2);
    }

    #[test]
    fn test_js_nesting_arrow_function() {
        let code = "function f(arr) { return arr.map(x => x * 2); }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        // arrow_function adds one nesting level
        assert_eq!(calculate_nesting_depth(node), 1);
    }

    #[test]
    fn test_js_assignment_abc() {
        // `let x = 1` is a variable_declarator (declaration), not an assignment_expression.
        // Only `x = expr` and `x += expr` forms count as A in ABC.
        let code = "function f(x) { x = 1; x += 2; return x; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert_eq!(abc.assignments, 2);
    }

    #[test]
    fn test_js_call_abc() {
        let code = "function f() { console.log('hi'); return 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert!(abc.branches >= 1);
    }

    #[test]
    fn test_js_ternary_abc() {
        let code = "function f(x) { return x > 0 ? 1 : 0; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert!(abc.conditions >= 1);
    }

    // ---- Ada complexity tests ----

    fn parse_ada(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_ada::LANGUAGE.into()).unwrap();
        parser.parse(code, None).unwrap()
    }

    fn ada_subprogram_node(tree: &tree_sitter::Tree) -> tree_sitter::Node<'_> {
        fn find(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if matches!(node.kind(), "subprogram_body" | "expression_function_declaration") {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(found) = find(child) {
                    return Some(found);
                }
            }
            None
        }
        find(tree.root_node()).expect("no subprogram node found")
    }

    #[test]
    fn test_ada_and_then_mccabe() {
        // `if A and then B` → base 1 (if) + 1 (and then) = 2
        let code = "procedure P is begin if A > 0 and then B > 0 then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3); // base 1 + if 1 + and 1
    }

    #[test]
    fn test_ada_or_else_mccabe() {
        // `if X or else Y` → base 1 + if 1 + or 1 = 3
        let code = "procedure P is begin if X = 1 or else Y = 2 then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_and_then_chain_mccabe() {
        // `A and then B and then C` → 2 `and` tokens → +2 logical, +1 if = 4 total
        let code = "procedure P is begin if A and then B and then C then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 4);
    }

    #[test]
    fn test_ada_and_then_cognitive() {
        // `and then` chain of 2 → one sequence → +1; if → +1; base is not counted
        let code = "procedure P is begin if A > 0 and then B > 0 then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        // if (nesting 0) = +1; and then (same-type chain) = +1 total for sequence
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_ada_and_then_or_else_cognitive() {
        // Two distinct operator sequences: `and` then `or` → +2 logical; if → +1
        // `(A and then B) or else C` — needs parens in Ada to mix operators
        let code = "procedure P is begin if (A and then B) or else C then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_param_count_single_name() {
        // `Z : Float` → 1 parameter; no self-field accesses → state_coupling = 1
        let code = "procedure P (Z : Float) is begin null; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_state_coupling(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_ada_param_count_multi_name() {
        // `X, Y : Integer` → 2 params in one spec; `Z : Float` → 1; total 3
        let code = "procedure P (X, Y : Integer; Z : Float) is begin null; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_state_coupling(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_param_count_three_names() {
        // `A, B, C : Boolean` → 3 params in one spec
        let code = "function F (A, B, C : Boolean) return Boolean is begin return A; end F;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_state_coupling(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_else_cognitive() {
        // if → +1+0; else → +1 flat; total = 2
        let code = "procedure P is begin if X then null; else null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_ada_if_no_else_cognitive() {
        // if with no else → +1+0 only
        let code = "procedure P is begin if X then null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_ada_if_elsif_else_cognitive() {
        // if → +1; elsif → +1 flat; else → +1 flat; total = 3
        let code = "procedure P is begin if A then null; elsif B then null; else null; end if; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_exit_when_mccabe() {
        // loop = +1; exit when = +1; base = 1 → total 3
        let code = "procedure P is begin loop exit when X > 0; end loop; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_exit_unconditional_mccabe() {
        // bare `exit` has no condition — no branch, loop = +1, base = 1 → total 2
        let code = "procedure P is begin loop exit; end loop; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_ada_raise_statement_mccabe() {
        // raise adds +1 like throw; base = 1 → total 2
        let code = "procedure P is begin raise Constraint_Error; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_ada_raise_statement_cognitive() {
        // raise adds +1 flat like goto/throw
        let code = "procedure P is begin raise Constraint_Error; end P;";
        let tree = parse_ada(code);
        let node = ada_subprogram_node(&tree);
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_ada_selective_accept_mccabe() {
        // 2 select_alternatives → +2; else → +1; base = 1 → total 4
        let code = "task body T is begin select accept A; or accept B; else null; end select; end T;";
        let tree = parse_ada(code);
        let node = tree.root_node();
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 4);
    }

    #[test]
    fn test_ada_guard_mccabe() {
        // guard (when condition) → +1; 1 select_alternative → +1; base = 1 → total 3
        let code = "task body T is begin select when Ready => accept A; end select; end T;";
        let tree = parse_ada(code);
        let node = tree.root_node();
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_ada_selective_accept_cognitive() {
        // selective_accept → +1+0; total = 1
        let code = "task body T is begin select accept A; or accept B; end select; end T;";
        let tree = parse_ada(code);
        let node = tree.root_node();
        assert_eq!(calculate_cognitive_complexity(node, code.as_bytes()), 1);
    }

    // ---- Rust ? operator tests ----

    #[test]
    fn test_rust_try_operator_mccabe() {
        // Each ? is a CFG branch: base 1 + 3 ? = 4
        let code = "fn f() -> Result<(), std::io::Error> { a()?; b()?; c()?; Ok(()) }";
        let tree = parse_rust(code);
        assert_eq!(calculate_mccabe_complexity(tree.root_node(), code.as_bytes()), 4);
    }

    #[test]
    fn test_rust_try_operator_single_mccabe() {
        let code = "fn f() -> Result<i32, String> { let x = g()?; Ok(x) }";
        let tree = parse_rust(code);
        // base 1 + 1 ? = 2
        assert_eq!(calculate_mccabe_complexity(tree.root_node(), code.as_bytes()), 2);
    }

    #[test]
    fn test_rust_try_operator_cognitive() {
        // ? does not add to cognitive complexity (linear error-propagation pipeline)
        let code = "fn f() -> Result<(), std::io::Error> { a()?; b()?; c()?; Ok(()) }";
        let tree = parse_rust(code);
        assert_eq!(calculate_cognitive_complexity(tree.root_node(), code.as_bytes()), 0);
    }

    #[test]
    fn test_rust_try_operator_abc() {
        // Each ? adds +1 condition in ABC
        let code = "fn f() -> Result<(), std::io::Error> { a()?; b()?; Ok(()) }";
        let tree = parse_rust(code);
        let abc = calculate_abc_complexity(tree.root_node(), code.as_bytes());
        assert_eq!(abc.conditions, 2);
    }

    // ---- JS/TS optional chaining tests ----

    fn parse_ts_function(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    #[test]
    fn test_js_optional_chain_mccabe() {
        // a?.b: null-check branch; base 1 + 1 optional_chain = 2
        let code = "function f(a) { return a?.b; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_js_optional_chain_multiple_mccabe() {
        // Two chained ?. operators; each optional_chain is a separate branch
        let code = "function f(a) { return a?.b?.c; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 3);
    }

    #[test]
    fn test_js_optional_chain_abc() {
        let code = "function f(a) { return a?.b; }";
        let tree = parse_js_function(code);
        let node = js_func_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    #[test]
    fn test_ts_optional_chain_mccabe() {
        let code = "function f(a: any) { return a?.foo; }";
        let tree = parse_ts_function(code);
        let node = js_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    // ---- C# conditional_access_expression (?.) tests ----

    fn parse_cs(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    fn cs_method_node(tree: &tree_sitter::Tree) -> tree_sitter::Node<'_> {
        fn find(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if node.kind() == "method_declaration" {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(f) = find(child) {
                    return Some(f);
                }
            }
            None
        }
        find(tree.root_node()).expect("no method_declaration found")
    }

    #[test]
    fn test_cs_null_conditional_mccabe() {
        // a?.Length: null-check branch; base 1 + 1 conditional_access_expression = 2
        let code = "class C { int? F(string a) { return a?.Length; } }";
        let tree = parse_cs(code);
        let node = cs_method_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_cs_null_conditional_abc() {
        let code = "class C { int? F(string a) { return a?.Length; } }";
        let tree = parse_cs(code);
        let node = cs_method_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    // ---- Kotlin Elvis (?:) tests ----

    fn parse_kotlin(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    fn kotlin_func_node(tree: &tree_sitter::Tree) -> tree_sitter::Node<'_> {
        fn find(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if node.kind() == "function_declaration" {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(f) = find(child) {
                    return Some(f);
                }
            }
            None
        }
        find(tree.root_node()).expect("no function_declaration found")
    }

    #[test]
    fn test_kotlin_elvis_mccabe() {
        // b ?: 0 — Elvis operator is a binary_expression with operator ?:; base 1 + 1 = 2
        let code = "fun f(b: String?): Int { return b?.length ?: 0 }";
        let tree = parse_kotlin(code);
        let node = kotlin_func_node(&tree);
        // ?: adds 1; ?. safe call is not counted (navigation expression, not binary)
        assert!(calculate_mccabe_complexity(node, code.as_bytes()) >= 2);
    }

    #[test]
    fn test_kotlin_elvis_abc() {
        let code = "fun f(b: String?): Int { return b ?: 0 }";
        let tree = parse_kotlin(code);
        let node = kotlin_func_node(&tree);
        let abc = calculate_abc_complexity(node, code.as_bytes());
        assert_eq!(abc.conditions, 1);
    }

    // ---- Swift try? tests ----

    fn parse_swift(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_swift::LANGUAGE.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    fn swift_func_node(tree: &tree_sitter::Tree) -> tree_sitter::Node<'_> {
        fn find(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if matches!(node.kind(), "function_declaration" | "init_declaration") {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(f) = find(child) {
                    return Some(f);
                }
            }
            None
        }
        find(tree.root_node()).expect("no function_declaration found")
    }

    #[test]
    fn test_swift_try_optional_mccabe() {
        // try? converts a throw to Optional — a real branch; base 1 + 1 = 2
        let code = "func f() -> Int? { let x = try? g(); return x }";
        let tree = parse_swift(code);
        let node = swift_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 2);
    }

    #[test]
    fn test_swift_try_plain_not_counted_mccabe() {
        // plain try propagates the error — not a branch in the CFG; base 1 + 0 = 1
        let code = "func f() throws -> Int { let x = try g(); return x }";
        let tree = parse_swift(code);
        let node = swift_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 1);
    }

    #[test]
    fn test_swift_try_force_not_counted_mccabe() {
        // try! crashes on failure — no branch; base 1 + 0 = 1
        let code = "func f() -> Int { let x = try! g(); return x }";
        let tree = parse_swift(code);
        let node = swift_func_node(&tree);
        assert_eq!(calculate_mccabe_complexity(node, code.as_bytes()), 1);
    }
}

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

fn visit_node_mccabe(node: Node, source_code: &[u8], complexity: &mut u32) {
    // Decision points that increase cyclomatic complexity
    match node.kind() {
        // C/C++ conditional statements
        "if_statement" => *complexity += 1,
        "while_statement" => *complexity += 1,
        "do_statement" => *complexity += 1,
        "for_statement" | "for_range_loop" => *complexity += 1,

        // Throw creates exceptional control flow path (like goto)
        "throw_statement" => *complexity += 1,

        // Switch statement: pmccabe compatibility - count as +1 regardless of cases
        // This matches pmccabe's simpler approach
        "switch_statement" => {
            *complexity += 1;
        }

        // Don't count individual case statements - handled by switch above
        // "case_statement" => *complexity += 1,

        // Rust conditional / loop expressions
        "if_expression" => *complexity += 1,
        "while_expression" | "for_expression" | "loop_expression" => *complexity += 1,
        // Rust match: same treatment as C switch (+1 for the whole expression)
        "match_expression" => *complexity += 1,

        // Python elif: each elif clause is an additional branch
        "elif_clause" => *complexity += 1,
        // Python except: like catch, creates an alternative path
        "except_clause" => *complexity += 1,
        // Python match_statement (3.10+): like switch
        "match_statement" => *complexity += 1,

        // JavaScript: for...in and for...of share for_in_statement
        "for_in_statement" => *complexity += 1,

        // Logical operators (each adds a path)
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" || op_text == "??" {
                        *complexity += 1;
                    }
                }
            }
        }

        // Ternary operator (C/C++) and Python ternary (x if cond else y)
        "conditional_expression" => *complexity += 1,
        // JavaScript ternary expression
        "ternary_expression" => *complexity += 1,

        // Python boolean operators: and/or each add a path (like && / ||)
        "boolean_operator" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "and" || op_text == "or" {
                        *complexity += 1;
                    }
                }
            }
        }

        // goto/continue/break can create additional paths
        "goto_statement" => *complexity += 1,

        // Ada: loop_statement covers plain loop, while loop, and for loop
        "loop_statement" => *complexity += 1,
        // Ada: each elsif branch is an additional path
        "elsif_statement_item" => *complexity += 1,
        // Ada: each case alternative (when branch) is an additional path
        "case_statement_alternative" => *complexity += 1,
        // Ada: each exception handler (when clause) is an additional path
        "exception_handler" => *complexity += 1,

        // Ada: logical operators (and then / or else / xor / and / or).
        // The `expression` node is flat: relations separated by unnamed keyword tokens.
        // Count each `and`, `or`, or `xor` child — each one is a branch point.
        "expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if !child.is_named() && matches!(child.kind(), "and" | "or" | "xor") {
                    *complexity += 1;
                }
            }
        }

        _ => {}
    }

    // Recursively visit children
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

fn visit_node_cognitive(
    node: Node,
    source_code: &[u8],
    nesting_level: u32,
    complexity: &mut u32,
    parent_binary_op: Option<&str>,
) {
    match node.kind() {
        // Control flow structures that increase complexity (C/C++ and Rust share some names)
        "if_statement" | "if_expression" => {
            *complexity += 1 + nesting_level;
            // Ada: else is an unnamed keyword child of if_statement (no named else_clause node).
            // C/C++ uses a named else_clause, so this check is safe for all grammars.
            let has_bare_else = {
                let mut cur = node.walk();
                let found = node.children(&mut cur).any(|c| !c.is_named() && c.kind() == "else");
                found
            };
            if has_bare_else {
                *complexity += 1;
            }
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Python elif: flat +1 (no nesting penalty), same semantics as C else-if
        "elif_clause" => {
            *complexity += 1;
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        // Else clause handling — used by both C/C++ (if_statement) and Rust (if_expression)
        "else_clause" => {
            let mut cursor = node.walk();

            for child in node.children(&mut cursor) {
                if child.kind() == "if_statement" || child.kind() == "if_expression" {
                    // else-if: +1 flat (no nesting penalty), body gets nesting_level+1
                    *complexity += 1;
                    visit_children_cognitive(child, source_code, nesting_level, complexity, None);
                    return;
                }
            }

            // Regular else: +1 without nesting increment
            *complexity += 1;
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        "while_statement" | "do_statement" | "for_statement" | "for_range_loop" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // JavaScript for...in and for...of share for_in_statement
        "for_in_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Rust loops
        "while_expression" | "for_expression" | "loop_expression" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Per SonarSource spec: try has NO cost and NO nesting increment.
        // Only catch gets +1 + nesting penalty from the outer scope.
        "try_statement" => {
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        // Lambda / closure body is a nested scope — children get increased nesting.
        // No +1 base cost: a closure is not itself a decision point.
        // "lambda" covers Python lambdas; "lambda_expression"/"closure_expression" cover C++/Rust.
        // "arrow_function" covers JavaScript arrow functions (() => ...).
        "lambda_expression" | "closure_expression" | "lambda" | "arrow_function" => {
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        "switch_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Rust match expression / Python match_statement: same treatment as switch
        "match_expression" | "match_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Case statements do NOT add complexity in cognitive complexity
        // (only the switch itself does); same for Rust match arms

        // Catch/except blocks — C++ uses catch_clause, Python uses except_clause
        "catch_clause" | "except_clause" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Jump statements: goto and throw (flat +1, no nesting penalty)
        "goto_statement" | "throw_statement" => {
            *complexity += 1;
        }

        // Binary logical operators (C/C++/Rust: && ||) — count once per contiguous sequence
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" || op_text == "??" {
                        if parent_binary_op != Some(op_text) {
                            *complexity += 1;
                        }
                        visit_children_cognitive_with_op(
                            node,
                            source_code,
                            nesting_level,
                            complexity,
                            Some(op_text),
                        );
                        return;
                    }
                }
            }
        }

        // Python boolean operators (and / or) — same chain-counting logic as binary_expression
        "boolean_operator" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "and" || op_text == "or" {
                        if parent_binary_op != Some(op_text) {
                            *complexity += 1;
                        }
                        visit_children_cognitive_with_op(
                            node,
                            source_code,
                            nesting_level,
                            complexity,
                            Some(op_text),
                        );
                        return;
                    }
                }
            }
        }

        // Ada: loop_statement covers plain loop, while loop, and for loop
        "loop_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Ada: elsif is flat +1, like Python elif
        "elsif_statement_item" => {
            *complexity += 1;
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        // Ada: case_statement is like switch
        "case_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Ada: each exception handler is like a catch clause
        "exception_handler" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Ada: logical operators (and then / or else / xor / and / or).
        // Count each new operator sequence (+1 per distinct contiguous sequence).
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

        _ => {}
    }

    // Visit children with current nesting level for non-control-flow nodes
    visit_children_cognitive(
        node,
        source_code,
        nesting_level,
        complexity,
        parent_binary_op,
    );
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
        visit_node_cognitive(
            child,
            source_code,
            nesting_level,
            complexity,
            parent_binary_op,
        );
    }
}

fn visit_children_cognitive_with_op(
    node: Node,
    source_code: &[u8],
    nesting_level: u32,
    complexity: &mut u32,
    parent_binary_op: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_cognitive(
            child,
            source_code,
            nesting_level,
            complexity,
            parent_binary_op,
        );
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
        | "loop_statement" | "case_statement" | "exception_handler" => {
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

        // Branches — function calls (C/C++/Rust use call_expression, Python uses call)
        "call_expression" | "call" => *branches += 1,
        // Ada: procedure and function invocations
        "procedure_call_statement" | "function_call" => *branches += 1,

        // Branches: throw, new, delete create control flow paths (C/C++)
        "throw_statement" | "new_expression" | "delete_expression" => *branches += 1,

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

        // Conditions (Rust)
        "if_expression" | "while_expression" | "for_expression" | "loop_expression"
        | "match_expression" => *conditions += 1,

        // Conditions (Python)
        "elif_clause" | "match_statement" => *conditions += 1,

        // Conditions (Ada)
        "elsif_statement_item" | "loop_statement" | "case_statement" | "exception_handler" => {
            *conditions += 1
        }

        // Logical operators (C/C++/Rust: &&/||; JavaScript also: ??)
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" || op_text == "??" {
                        *conditions += 1;
                    }
                }
            }
        }

        // Logical operators (Python: and/or)
        "boolean_operator" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "and" || op_text == "or" {
                        *conditions += 1;
                    }
                }
            }
        }

        // Logical operators (Ada: and then / or else / xor / and / or).
        // Count each and/or/xor child as a condition branch point.
        "expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if !child.is_named() && matches!(child.kind(), "and" | "or" | "xor") {
                    *conditions += 1;
                }
            }
        }

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

fn visit_node_dependencies(
    node: Node,
    source_code: &[u8],
    has_io: &mut bool,
    has_allocation: &mut bool,
    has_system_calls: &mut bool,
    modifies_globals: &mut bool,
) {
    if node.kind() == "call_expression" || node.kind() == "call" {
        if let Some(function) = node.child_by_field_name("function") {
            if let Ok(func_name) = function.utf8_text(source_code) {
                // File I/O — C/C++ functions
                if matches!(
                    func_name,
                    "fopen"
                        | "fclose"
                        | "fread"
                        | "fwrite"
                        | "fprintf"
                        | "fscanf"
                        | "fgets"
                        | "fputs"
                        | "fseek"
                        | "ftell"
                        | "rewind"
                        | "printf"
                        | "scanf"
                        | "puts"
                        | "getc"
                        | "putc"
                ) {
                    *has_io = true;
                }

                // File I/O — Python functions
                if matches!(func_name, "open" | "print" | "input") {
                    *has_io = true;
                }

                // Memory allocation (C/C++)
                if matches!(
                    func_name,
                    "malloc" | "calloc" | "realloc" | "free" | "aligned_alloc"
                ) {
                    *has_allocation = true;
                }

                // System calls (C/C++)
                if matches!(
                    func_name,
                    "time"
                        | "clock"
                        | "rand"
                        | "srand"
                        | "getpid"
                        | "fork"
                        | "exec"
                        | "system"
                        | "signal"
                        | "kill"
                        | "wait"
                        | "pipe"
                ) {
                    *has_system_calls = true;
                }

                // System calls (Python simple-name form)
                if matches!(func_name, "exit" | "abort") {
                    *has_system_calls = true;
                }
            }
        }
    }

    // C++ new/delete expressions count as allocation
    if node.kind() == "new_expression" || node.kind() == "delete_expression" {
        *has_allocation = true;
    }

    // Check for global variable modifications (simplified - looks for assignments to identifiers)
    if node.kind() == "assignment_expression" {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                // Heuristic: if identifier doesn't start with lowercase, might be global
                if let Ok(name) = left.utf8_text(source_code) {
                    if !name.is_empty() && name.chars().next().unwrap().is_uppercase() {
                        *modifies_globals = true;
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_dependencies(
            child,
            source_code,
            has_io,
            has_allocation,
            has_system_calls,
            modifies_globals,
        );
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
fn count_explicit_params(node: Node, source_code: &[u8]) -> u32 {
    match node.kind() {
        "function_item" => {
            // Rust: named 'parameters' field; children are 'parameter' or 'self_parameter'
            if let Some(params) = node.child_by_field_name("parameters") {
                let mut cursor = params.walk();
                return params
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "parameter")
                    .count() as u32;
            }
            0
        }
        "function_definition" => {
            // Python has a named 'parameters' field; C does not (uses declarator).
            if let Some(params) = node.child_by_field_name("parameters") {
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
                        | "dictionary_splat_pattern" => count += 1,
                        _ => {}
                    }
                }
                return count;
            }
            // C/C++ function_definition: drill into the declarator subtree
            count_c_params_in_subtree(node, source_code)
        }
        "function_declaration" => {
            // JavaScript/TypeScript: has a direct 'parameters' field (formal_parameters)
            if let Some(params) = node.child_by_field_name("parameters") {
                let mut cursor = params.walk();
                return params
                    .children(&mut cursor)
                    .filter(|c| {
                        matches!(
                            c.kind(),
                            "identifier"
                                | "required_parameter"
                                | "optional_parameter"
                                | "rest_pattern"
                                | "assignment_pattern"
                        )
                    })
                    .count() as u32;
            }
            // C/C++: no 'parameters' field, drill into declarator chain
            count_c_params_in_subtree(node, source_code)
        }
        // JavaScript/TypeScript: method_definition, function_expression, etc.
        "method_definition"
        | "function_expression"
        | "generator_function_declaration"
        | "generator_function" => {
            if let Some(params) = node.child_by_field_name("parameters") {
                let mut cursor = params.walk();
                return params
                    .children(&mut cursor)
                    .filter(|c| {
                        matches!(
                            c.kind(),
                            "identifier"
                                | "required_parameter"
                                | "optional_parameter"
                                | "rest_pattern"
                                | "assignment_pattern"
                        )
                    })
                    .count() as u32;
            }
            0
        }
        "subprogram_body" | "expression_function_declaration" => {
            // Ada: traverse to function_specification or procedure_specification,
            // then find formal_part and count actual parameter names.
            // Each parameter_specification may declare multiple names: `X, Y : T` → 2 params.
            // Count identifier children before the `:` separator in each spec.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "function_specification" | "procedure_specification") {
                    let mut spec_cursor = child.walk();
                    for spec_child in child.children(&mut spec_cursor) {
                        if spec_child.kind() == "formal_part" {
                            let mut total = 0u32;
                            let mut formal_cursor = spec_child.walk();
                            for param_spec in spec_child.children(&mut formal_cursor) {
                                if param_spec.kind() == "parameter_specification" {
                                    // Identifiers before `:` are parameter names; after `:` is the type.
                                    let mut ps_cursor = param_spec.walk();
                                    for ps_child in param_spec.children(&mut ps_cursor) {
                                        if !ps_child.is_named() && ps_child.kind() == ":" {
                                            break;
                                        }
                                        if ps_child.is_named() && ps_child.kind() == "identifier" {
                                            total += 1;
                                        }
                                    }
                                }
                            }
                            return total;
                        }
                    }
                    return 0;
                }
            }
            0
        }
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

fn collect_self_fields_recursive(node: Node, source_code: &[u8], fields: &mut HashSet<String>) {
    // Rust: field_expression where the 'value' child is the identifier "self"
    if node.kind() == "field_expression" {
        if let Some(value) = node.child_by_field_name("value") {
            if value.utf8_text(source_code).unwrap_or("") == "self" {
                if let Some(field) = node.child_by_field_name("field") {
                    if let Ok(name) = field.utf8_text(source_code) {
                        fields.insert(name.to_string());
                    }
                }
            }
        }
    }
    // Python: attribute node where the 'object' child is the identifier "self"
    if node.kind() == "attribute" {
        if let Some(obj) = node.child_by_field_name("object") {
            if obj.utf8_text(source_code).unwrap_or("") == "self" {
                if let Some(attr) = node.child_by_field_name("attribute") {
                    if let Ok(name) = attr.utf8_text(source_code) {
                        fields.insert(name.to_string());
                    }
                }
            }
        }
    }
    // JavaScript/TypeScript: member_expression where 'object' is 'this'
    if node.kind() == "member_expression" {
        if let Some(obj) = node.child_by_field_name("object") {
            if obj.utf8_text(source_code).unwrap_or("") == "this" {
                if let Some(prop) = node.child_by_field_name("property") {
                    if let Ok(name) = prop.utf8_text(source_code) {
                        fields.insert(name.to_string());
                    }
                }
            }
        }
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
}

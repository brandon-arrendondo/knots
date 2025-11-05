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
        // Conditional statements
        "if_statement" => *complexity += 1,
        "while_statement" => *complexity += 1,
        "do_statement" => *complexity += 1,
        "for_statement" => *complexity += 1,

        // Switch statement: pmccabe compatibility - count as +1 regardless of cases
        // This matches pmccabe's simpler approach 
        "switch_statement" => {
            *complexity += 1;
        }

        // Don't count individual case statements - handled by switch above
        // "case_statement" => *complexity += 1,

        // Logical operators (each adds a path)
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" {
                        *complexity += 1;
                    }
                }
            }
        }

        // Ternary operator
        "conditional_expression" => *complexity += 1,

        // goto/continue/break can create additional paths
        "goto_statement" => *complexity += 1,

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

fn visit_node_cognitive(node: Node, source_code: &[u8], nesting_level: u32, complexity: &mut u32, parent_binary_op: Option<&str>) {
    match node.kind() {
        // Control flow structures that increase complexity
        "if_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Else clause handling
        "else_clause" => {
            // Check if this is an "else if" by looking for if_statement as direct child
            let mut cursor = node.walk();

            for child in node.children(&mut cursor) {
                if child.kind() == "if_statement" {
                    // For else-if, only add +1 total (not +1 for else and +1+nesting for if)
                    // Process the if with current nesting level, not increased
                    *complexity += 1;
                    visit_children_cognitive(child, source_code, nesting_level, complexity, None);
                    return;
                }
            }

            // Regular else clause adds +1 without nesting increment
            *complexity += 1;
            visit_children_cognitive(node, source_code, nesting_level, complexity, None);
            return;
        }

        "while_statement" | "do_statement" | "for_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        "switch_statement" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Case statements do NOT add complexity in cognitive complexity
        // (only the switch itself does)

        // Catch blocks
        "catch_clause" => {
            *complexity += 1 + nesting_level;
            visit_children_cognitive(node, source_code, nesting_level + 1, complexity, None);
            return;
        }

        // Jump statements: only goto (not break/continue in switches)
        "goto_statement" => {
            *complexity += 1;
        }

        // Binary logical operators - only count if not same as parent operator
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" {
                        // Only add complexity if this operator is different from parent
                        // This ensures we only count once per sequence of same operators
                        if parent_binary_op != Some(op_text) {
                            *complexity += 1;
                        }
                        // Pass this operator as parent to children
                        visit_children_cognitive_with_op(node, source_code, nesting_level, complexity, Some(op_text));
                        return;
                    }
                }
            }
        }

        // Recursive calls (identified by looking for function calls)
        // This is a simplified heuristic - in practice, you'd need to track function names

        _ => {}
    }

    // Visit children with current nesting level for non-control-flow nodes
    visit_children_cognitive(node, source_code, nesting_level, complexity, parent_binary_op);
}

fn visit_children_cognitive(node: Node, source_code: &[u8], nesting_level: u32, complexity: &mut u32, parent_binary_op: Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_cognitive(child, source_code, nesting_level, complexity, parent_binary_op);
    }
}

fn visit_children_cognitive_with_op(node: Node, source_code: &[u8], nesting_level: u32, complexity: &mut u32, parent_binary_op: Option<&str>) {
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
    let new_depth = match node.kind() {
        "if_statement" | "while_statement" | "do_statement" | "for_statement"
        | "switch_statement" | "compound_statement" => {
            let depth = current_depth + 1;
            if depth > *max_depth {
                *max_depth = depth;
            }
            depth
        }
        _ => current_depth
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_nesting(child, new_depth, max_depth);
    }
}

/// Calculates Source Lines of Code (SLOC) - non-comment, non-blank lines
pub fn calculate_sloc(node: Node, source_code: &[u8]) -> u32 {
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

        // Handle multi-line comments
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

        // Check for single-line comment
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

    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }

    None
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

    visit_node_abc(node, source_code, &mut assignments, &mut branches, &mut conditions);

    AbcComplexity {
        assignments,
        branches,
        conditions,
    }
}

fn visit_node_abc(node: Node, source_code: &[u8], assignments: &mut u32, branches: &mut u32, conditions: &mut u32) {
    match node.kind() {
        // Assignments
        "assignment_expression" => {
            *assignments += 1;
        }
        "update_expression" => {
            // ++ and -- operators
            *assignments += 1;
        }

        // Branches (function calls)
        "call_expression" => {
            *branches += 1;
        }

        // Conditions
        "if_statement" | "while_statement" | "do_statement" | "for_statement"
        | "switch_statement" | "conditional_expression" => {
            *conditions += 1;
        }

        // Logical operators
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                if let Ok(op_text) = op.utf8_text(source_code) {
                    if op_text == "&&" || op_text == "||" {
                        *conditions += 1;
                    }
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
    if node.kind() == "return_statement" {
        *count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node_returns(child, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Tree;

    fn parse_c_function(code: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::language()).unwrap();
        parser.parse(code, None).unwrap()
    }

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
}

//! The real preprocessor configurations of one C or C++ function, so a
//! metric can be scored per configuration instead of summed across arms no
//! build compiles together (ADR-0002 §3).
//!
//! The substrate has already blanked arms the file proves dead
//! (`blank_dead_code`). What remains are live alternatives: each
//! `#if`/`#ifdef` chain is one choice among its arms (an absent `#else` is an
//! empty arm). Chains in the same function with the same conditions are the
//! same choice, so `#ifdef FAST ... #endif` and a later `#ifndef FAST ...
//! #endif` never both count. Conditions are compared as written (whitespace
//! and `defined X` spelling normalized); logically related conditions
//! written differently are treated as independent.
//!
//! A configuration is returned as the set of node ids to skip: the content
//! of every arm not chosen.

use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

/// Configurations beyond this many are not enumerated; the function is then
/// scored with every arm counted, as before.
const MAX_CONFIGURATIONS: usize = 64;

/// One chain's arms, keyed for linking: the conditions of its conditional
/// arms, and each arm's content (one more entry than conditions: `#else`).
struct Chain {
    conditions: Vec<String>,
    arms: Vec<Vec<usize>>,
}

/// The skip set of each real configuration of `func`. Always at least one
/// set; a single empty set when the function has no live alternatives or
/// has more than `MAX_CONFIGURATIONS`.
pub(crate) fn configurations(func: Node, source_code: &[u8]) -> Vec<HashSet<usize>> {
    let choices = choices(chains(func, source_code));
    let count = choices
        .iter()
        .try_fold(1usize, |n, options| n.checked_mul(options.len()))
        .filter(|n| *n <= MAX_CONFIGURATIONS);
    match count {
        Some(_) => product(&choices),
        None => vec![HashSet::new()],
    }
}

/// For each linked group with more than one non-empty option, the skip set
/// of each option.
fn choices(chains: Vec<Chain>) -> Vec<Vec<HashSet<usize>>> {
    linked(chains)
        .into_values()
        .map(|group| options(&group))
        .filter(|options| options.len() > 1)
        .collect()
}

/// Chains grouped by their conditions: one group is one choice.
fn linked(chains: Vec<Chain>) -> HashMap<Vec<String>, Vec<Chain>> {
    let mut groups: HashMap<Vec<String>, Vec<Chain>> = HashMap::new();
    for chain in chains {
        groups
            .entry(chain.conditions.clone())
            .or_default()
            .push(chain);
    }
    groups
}

/// Skip sets for choosing each arm of a linked group. An option whose arms
/// are empty in every chain adds nothing that another option lacks, so it
/// is dropped when any other option exists.
fn options(group: &[Chain]) -> Vec<HashSet<usize>> {
    let width = group[0].arms.len();
    let live: Vec<usize> = (0..width)
        .filter(|i| group.iter().any(|chain| !chain.arms[*i].is_empty()))
        .collect();
    live.iter().map(|chosen| skipped(group, *chosen)).collect()
}

fn skipped(group: &[Chain], chosen: usize) -> HashSet<usize> {
    let others = group.iter().flat_map(|chain| {
        chain
            .arms
            .iter()
            .enumerate()
            .filter(move |(i, _)| *i != chosen)
    });
    others.flat_map(|(_, ids)| ids.iter().copied()).collect()
}

/// Every combination of one option per group, as the union of skip sets.
fn product(choices: &[Vec<HashSet<usize>>]) -> Vec<HashSet<usize>> {
    let mut configurations = vec![HashSet::new()];
    for options in choices {
        configurations = configurations
            .iter()
            .flat_map(|base| {
                options
                    .iter()
                    .map(move |option| base.union(option).copied().collect())
            })
            .collect();
    }
    configurations
}

/// Every `#if`/`#ifdef` chain under `func` (an `#elif` belongs to its chain).
fn chains(func: Node, source_code: &[u8]) -> Vec<Chain> {
    let mut found = Vec::new();
    let mut stack = vec![func];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "preproc_if" | "preproc_ifdef") {
            found.push(chain(node, source_code));
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    found
}

fn chain(head: Node, source_code: &[u8]) -> Chain {
    let nodes = arm_nodes(head);
    let conditions = chain_conditions(&nodes, source_code);
    let arms = chain_arms(&nodes, conditions.len());
    normalized(Chain { conditions, arms })
}

fn chain_conditions(nodes: &[Node], source_code: &[u8]) -> Vec<String> {
    let conditional = nodes.iter().filter(|arm| arm.kind() != "preproc_else");
    conditional
        .map(|arm| arm_condition(*arm, source_code))
        .collect()
}

/// Each arm's content, with an empty else arm when the chain has no #else.
fn chain_arms(nodes: &[Node], conditions: usize) -> Vec<Vec<usize>> {
    let mut arms: Vec<Vec<usize>> = nodes.iter().map(|arm| arm_content(*arm)).collect();
    arms.resize(conditions + 1, Vec::new());
    arms
}

/// The chain's arms in order: the head, then each `alternative`.
fn arm_nodes(head: Node) -> Vec<Node> {
    let mut nodes = vec![head];
    while let Some(next) = nodes[nodes.len() - 1].child_by_field_name("alternative") {
        nodes.push(next);
    }
    nodes
}

/// `#ifndef X` is `#ifdef X` with its arms swapped, so the two link.
fn normalized(mut chain: Chain) -> Chain {
    let negated = chain.conditions.len() == 1 && chain.conditions[0].starts_with('!');
    if negated {
        chain.conditions[0].remove(0);
        chain.arms.swap(0, 1);
    }
    chain
}

/// The statements of one arm: its named children that aren't the condition,
/// the macro name or the next arm.
fn arm_content(arm: Node) -> Vec<usize> {
    let content = unfielded_children(arm).into_iter().filter(|c| c.is_named());
    content.map(|c| c.id()).collect()
}

fn unfielded_children(node: Node) -> Vec<Node> {
    let unfielded =
        (0..node.child_count()).filter(|i| node.field_name_for_child(*i as u32).is_none());
    unfielded.filter_map(|i| node.child(i)).collect()
}

fn arm_condition(arm: Node, source_code: &[u8]) -> String {
    match arm.child_by_field_name("condition") {
        Some(condition) => condition_text(condition, source_code),
        None => defined_test(arm, source_code),
    }
}

/// `#ifdef X` / `#elifdef X` as `defined(X)`; the `n` forms as `!defined(X)`.
fn defined_test(arm: Node, source_code: &[u8]) -> String {
    let name = arm
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source_code).ok())
        .unwrap_or("");
    let directive = arm.child(0).map_or("", |d| d.kind());
    let negated = directive.ends_with("ndef");
    format!("{}defined({name})", if negated { "!" } else { "" })
}

/// A condition's tokens without whitespace. `defined X` and `defined(X)`
/// are both a `preproc_defined` node and are written `defined(X)`, so the
/// two spellings link.
fn condition_text(condition: Node, source_code: &[u8]) -> String {
    let mut text = String::new();
    let mut stack = vec![condition];
    while let Some(node) = stack.pop() {
        text.push_str(&token_text(node, source_code));
        if node.kind() != "preproc_defined" {
            stack.extend(children_reversed(node));
        }
    }
    text
}

fn children_reversed(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    let mut children: Vec<Node> = node.children(&mut cursor).collect();
    children.reverse();
    children
}

fn token_text(node: Node, source_code: &[u8]) -> String {
    match node.kind() {
        "preproc_defined" => defined_text(node, source_code),
        _ if node.child_count() == 0 => node.utf8_text(source_code).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

fn defined_text(node: Node, source_code: &[u8]) -> String {
    let name = node
        .named_child(0)
        .and_then(|n| n.utf8_text(source_code).ok());
    format!("defined({})", name.unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(code: &str) -> usize {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&crate::tree_sitter_c::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        configurations(tree.root_node(), code.as_bytes()).len()
    }

    #[test]
    fn both_spellings_of_defined_and_their_negations_link() {
        // One macro, four spellings: two configurations, not sixteen.
        let code =
            "void f(int x) {\n#ifdef A\nx++;\n#endif\n#if defined A\nx++;\n#else\nx--;\n#endif\n\
                    #if !defined(A)\nx--;\n#endif\n#ifndef A\nx--;\n#endif\n}";
        assert_eq!(count(code), 2);
    }

    #[test]
    fn independent_macros_multiply() {
        let code = "void f(int x) {\n#ifdef A\nx++;\n#else\nx--;\n#endif\n#ifdef B\nx++;\n#else\nx--;\n#endif\n}";
        assert_eq!(count(code), 4);
    }

    #[test]
    fn an_ifdef_without_else_is_no_choice() {
        assert_eq!(count("void f(int x) {\n#ifdef A\nx++;\n#endif\n}"), 1);
    }
}

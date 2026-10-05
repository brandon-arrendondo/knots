//! Which functions in a file sit in a call cycle, for Cognitive Complexity's
//! recursion increment (whitepaper 1.7, Appendix B1: +1 for each method in a
//! recursion cycle, direct or indirect).
//!
//! Calls are resolved from syntax alone, so a call becomes an edge only when
//! the syntax fixes its target:
//!
//! - a bare call, `name(..)`, to a function of that name that isn't a Rust
//!   impl or trait member (a bare call can't reach one). In Rust the name is
//!   looked up from the innermost enclosing `fn` or `mod` outwards, so `mod
//!   small`'s `iadd` and `mod large`'s are different functions. Elsewhere it
//!   is looked up file-wide: in C a function nested in another is nearly
//!   always the parser recovering from a macro, not a real scope;
//! - a call through `self` or `this`, `self.name(..)`, to the caller's own
//!   type's `name`;
//! - in a Rust impl or trait, `Self::name` or `Type::name`, called or passed
//!   as a value (`map(Self::count)`), to that type's `name`.
//!
//! A method called on any other receiver (`self.inner.fmt(f)`,
//! `child.depth()`) has a type knots can't see. Matching it by name alone
//! scored a third of the Rust functions in twelve crates as recursive, nearly
//! all of them delegations such as `clone`, `hash` and `fmt`, so it adds no
//! edge, and recursion that goes only through such a call is not counted.

use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

use crate::{get_function_name, visit_functions};

/// A call target as the syntax names it.
enum Callee {
    /// `name(..)`: a function that isn't a type's member.
    Free(String),
    /// `self.name(..)`, `Self::name`: a member of the caller's own type.
    Member(String),
}

struct Site {
    id: usize,
    name: String,
    /// The type (or trait) whose impl a Rust function is in.
    owner: Option<String>,
    /// The scope the function is declared in (`None` for the file).
    scope: Option<usize>,
    /// Where a bare call from this function looks, innermost first: its own
    /// body, then each enclosing scope, then the file.
    lookup: Vec<Option<usize>>,
    refs: Vec<Callee>,
}

/// Node ids of the functions under `root` that are in a call cycle.
pub(crate) fn recursive_functions(root: Node, source_code: &str) -> HashSet<usize> {
    let sites = function_sites(root, source_code);
    let graph: HashMap<usize, Vec<usize>> = sites
        .iter()
        .map(|site| (site.id, callees(site, &sites)))
        .collect();
    sites
        .iter()
        .map(|site| site.id)
        .filter(|id| reaches_itself(&graph, *id))
        .collect()
}

fn function_sites(root: Node, source_code: &str) -> Vec<Site> {
    let mut sites = Vec::new();
    let mut cursor = root.walk();
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            sites.push(site(node, name, src));
        }
    });
    sites
}

fn site(node: Node, name: String, source_code: &str) -> Site {
    let owner = rust_owner(node, source_code);
    let refs = references(node, source_code, owner.as_deref());
    let lookup = lookup_scopes(node);
    Site {
        id: node.id(),
        name,
        owner,
        scope: lookup.get(1).copied().flatten(),
        lookup,
        refs,
    }
}

fn is_scope(node: Node) -> bool {
    matches!(node.kind(), "mod_item" | "function_item")
}

fn lookup_scopes(func: Node) -> Vec<Option<usize>> {
    let mut scopes = vec![Some(func.id())];
    let mut parent = func.parent();
    while let Some(node) = parent {
        if is_scope(node) {
            scopes.push(Some(node.id()));
        }
        parent = node.parent();
    }
    scopes.push(None);
    scopes
}

fn callees(caller: &Site, sites: &[Site]) -> Vec<usize> {
    caller
        .refs
        .iter()
        .flat_map(|callee| resolve(callee, caller, sites))
        .collect()
}

/// A member reference resolves only when the caller's type has exactly one
/// function of that name: a type implementing `From<A>` and `From<B>` has two
/// `from`s, and the syntax doesn't say which `Self::from(x)` reaches.
fn resolve(callee: &Callee, caller: &Site, sites: &[Site]) -> Vec<usize> {
    match (callee, &caller.owner) {
        (Callee::Member(name), Some(owner)) => {
            let found = named(sites, name, Some(owner));
            if found.len() == 1 {
                found
            } else {
                Vec::new()
            }
        }
        (Callee::Member(name) | Callee::Free(name), _) => in_nearest_scope(sites, name, caller),
    }
}

fn in_nearest_scope(sites: &[Site], name: &str, caller: &Site) -> Vec<usize> {
    let free: Vec<&Site> = sites
        .iter()
        .filter(|site| site.name == name && site.owner.is_none())
        .collect();
    let nearest = caller.lookup.iter().find(|scope| declares(&free, **scope));
    nearest.map_or_else(Vec::new, |scope| declared_in(&free, *scope))
}

fn declares(free: &[&Site], scope: Option<usize>) -> bool {
    free.iter().any(|site| site.scope == scope)
}

fn declared_in(free: &[&Site], scope: Option<usize>) -> Vec<usize> {
    free.iter()
        .filter(|site| site.scope == scope)
        .map(|site| site.id)
        .collect()
}

fn named(sites: &[Site], name: &str, owner: Option<&String>) -> Vec<usize> {
    sites
        .iter()
        .filter(|site| site.name == name && site.owner.as_ref() == owner)
        .map(|site| site.id)
        .collect()
}

fn reaches_itself(graph: &HashMap<usize, Vec<usize>>, start: usize) -> bool {
    let mut stack = vec![start];
    let mut seen: HashSet<usize> = HashSet::new();
    while let Some(id) = stack.pop() {
        for &callee in graph.get(&id).into_iter().flatten() {
            if callee == start {
                return true;
            }
            if seen.insert(callee) {
                stack.push(callee);
            }
        }
    }
    false
}

/// For a Rust function in an impl or trait block, the type it is for (the
/// trait's name for a trait's own methods), without path or generics.
fn rust_owner(func: Node, source_code: &str) -> Option<String> {
    if func.kind() != "function_item" {
        return None;
    }
    let block = func.parent()?.parent()?;
    match block.kind() {
        "impl_item" => type_name(block.child_by_field_name("type")?, source_code),
        "trait_item" => text(block.child_by_field_name("name")?, source_code),
        _ => None,
    }
}

fn type_name(ty: Node, source_code: &str) -> Option<String> {
    let base = match ty.kind() {
        "generic_type" => ty.child_by_field_name("type")?,
        _ => ty,
    };
    let last = match base.kind() {
        "scoped_type_identifier" => base.child_by_field_name("name")?,
        _ => base,
    };
    text(last, source_code)
}

fn references(func: Node, source_code: &str, owner: Option<&str>) -> Vec<Callee> {
    let mut refs = Vec::new();
    let mut stack = vec![func];
    while let Some(node) = stack.pop() {
        refs.extend(reference(node, source_code, owner));
        let mut cursor = node.walk();
        stack.extend(
            node.named_children(&mut cursor)
                .filter(|c| !is_named_function(*c)),
        );
    }
    refs
}

/// A Rust `fn` nested in another is a site of its own; its calls are not its
/// parent's. Closures are part of the function they're in.
fn is_named_function(node: Node) -> bool {
    node.kind() == "function_item"
}

fn reference(node: Node, source_code: &str, owner: Option<&str>) -> Option<Callee> {
    match node.kind() {
        "call_expression" | "call" | "invocation_expression" => call(node, source_code),
        "method_invocation" => java_called(node, source_code),
        "procedure_call_statement" | "function_call" => named_call(node, source_code),
        "scoped_identifier" => own_type_path(node, source_code, owner?),
        _ => None,
    }
}

fn call(node: Node, source_code: &str) -> Option<Callee> {
    match node.child_by_field_name("function") {
        Some(function) => called(function, source_code),
        None => kotlin_called(node, source_code),
    }
}

/// Ada and Fortran calls carry the callee in a `name` field.
fn named_call(node: Node, source_code: &str) -> Option<Callee> {
    text(node.child_by_field_name("name")?, source_code).map(Callee::Free)
}

/// The target of a call's `function` part: a bare name, or a field of
/// `self`/`this` (Rust, C, C++).
fn called(function: Node, source_code: &str) -> Option<Callee> {
    match function.kind() {
        "identifier" => text(function, source_code).map(Callee::Free),
        "field_expression" => self_field(function, source_code),
        _ => None,
    }
}

/// `self.name` (Rust) or `this->name` (C++); any other receiver is unresolved.
fn self_field(function: Node, source_code: &str) -> Option<Callee> {
    let receiver = function
        .child_by_field_name("value")
        .or_else(|| function.child_by_field_name("argument"))?;
    let through_self = matches!(text(receiver, source_code)?.as_str(), "self" | "this");
    through_self
        .then(|| text(function.child_by_field_name("field")?, source_code).map(Callee::Member))?
}

/// Kotlin's call has no `function` field; its callee is the first named child.
fn kotlin_called(node: Node, source_code: &str) -> Option<Callee> {
    let mut cursor = node.walk();
    let first = node.named_children(&mut cursor).next()?;
    (first.kind() == "simple_identifier").then(|| text(first, source_code).map(Callee::Free))?
}

/// Java: `name(..)` or `this.name(..)`; any other receiver is unresolved.
fn java_called(node: Node, source_code: &str) -> Option<Callee> {
    let unqualified = match node.child_by_field_name("object") {
        None => true,
        Some(object) => text(object, source_code)? == "this",
    };
    unqualified.then(|| text(node.child_by_field_name("name")?, source_code).map(Callee::Free))?
}

/// Rust `Self::name` or `Owner::name` inside `Owner`'s impl.
fn own_type_path(node: Node, source_code: &str, owner: &str) -> Option<Callee> {
    let path = text(node.child_by_field_name("path")?, source_code)?;
    let own = path == "Self" || path == owner;
    own.then(|| text(node.child_by_field_name("name")?, source_code).map(Callee::Member))?
}

fn text(node: Node, source_code: &str) -> Option<String> {
    node.utf8_text(source_code.as_bytes())
        .ok()
        .map(str::to_string)
}

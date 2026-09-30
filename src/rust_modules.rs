//! Rust `use` resolution for file-level coupling: places each `.rs` file in
//! its crate's module tree and resolves `use` paths to the corpus files that
//! define the modules they name.
//!
//! The generic resolver in [`crate::coupling`] matches an import's trailing
//! identifier against file stems. That cannot work for Rust: a `use` names an
//! item (`crate::coupling::FileCoupling`) or a group
//! (`crate::{a, b::C}`) far more often than a module, and `mod.rs` stems are
//! ambiguous by construction. So Rust gets its own two passes, still
//! source-only (no `cargo metadata`, no build):
//!
//! 1. [`rust_use_paths`] expands every `use` tree to flat paths, one per
//!    imported name, and rewrites `super`/`self` inside an inline
//!    `mod name { ... }` so the path is relative to the file's own module.
//! 2. [`RustModuleIndex`] maps each file to a crate root and a module path
//!    by the standard layout (`<name>.rs` or `<name>/mod.rs`, children of a
//!    non-`mod.rs` file in a directory named after it), and resolves each
//!    path to the deepest module *file* it passes through.
//!
//! The crate a file belongs to is found through the nearest `Cargo.toml`,
//! which is read only for the library's name and the declared library and
//! binary paths; `src/bin`, `tests`, `examples` and `benches` roots follow
//! Cargo's defaults. A file under no `Cargo.toml` falls back to the nearest
//! directory holding a corpus `lib.rs` or `main.rs`. A `mod` declaration is
//! not itself an edge — it defines the tree, it does not use it — and
//! `#[path]` attributes are not followed.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use tree_sitter::{Node, Tree};

/// Every `use` path in a Rust `tree`, expanded to one `::`-joined path per
/// imported name (`use a::{b, c::D}` gives `a::b` and `a::c::D`), with
/// aliases and globs dropped. A path inside an inline `mod` is rewritten
/// relative to the file's module: `super` that only climbs out of inline
/// modules becomes `self`, and a relative path becomes `::`-rooted, since
/// it can only name a child of the inline module (same file) or an
/// external crate.
pub fn rust_use_paths(tree: &Tree, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    collect_uses(tree.root_node(), source, 0, &mut out);
    out
}

fn collect_uses(node: Node, source: &[u8], inline_depth: usize, out: &mut Vec<String>) {
    if node.kind() == "use_declaration" {
        if let Some(arg) = node.child_by_field_name("argument") {
            let mut paths = Vec::new();
            expand_use_tree(arg, source, &[], &mut paths);
            out.extend(
                paths
                    .into_iter()
                    .map(|p| rebase_inline(p, inline_depth).join("::")),
            );
        }
        return;
    }
    let depth = inline_depth + usize::from(is_inline_mod(node));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_uses(child, source, depth, out);
    }
}

fn is_inline_mod(node: Node) -> bool {
    node.kind() == "mod_item" && node.child_by_field_name("body").is_some()
}

fn expand_use_tree(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<Vec<String>>) {
    match node.kind() {
        "use_as_clause" | "use_wildcard" => expand_first_path(node, source, prefix, out),
        "scoped_use_list" => expand_scoped_list(node, source, prefix, out),
        "use_list" => expand_list(node, source, prefix, out),
        "line_comment" | "block_comment" => {}
        _ => out.push(join_segments(prefix, node_text(node, source))),
    }
}

/// `path as alias` and `path::*`: only the path matters. A bare `*` in a
/// list (`a::{*}`) has no path child and imports from the prefix itself.
fn expand_first_path(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<Vec<String>>) {
    let path = node
        .child_by_field_name("path")
        .or_else(|| node.named_child(0));
    match path {
        Some(p) => expand_use_tree(p, source, prefix, out),
        None => out.push(prefix.to_vec()),
    }
}

fn expand_scoped_list(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<Vec<String>>) {
    let inner = match node.child_by_field_name("path") {
        Some(p) => join_segments(prefix, node_text(p, source)),
        None => prefix.to_vec(),
    };
    if let Some(list) = node.child_by_field_name("list") {
        expand_list(list, source, &inner, out);
    }
}

fn expand_list(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<Vec<String>>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        expand_use_tree(child, source, prefix, out);
    }
}

fn node_text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

/// `prefix` extended by the `::`-separated segments of `text`. A trailing
/// `self` (`a::{self}`) names the prefix itself and is dropped.
fn join_segments(prefix: &[String], text: &str) -> Vec<String> {
    let mut segs = prefix.to_vec();
    segs.extend(text.split("::").map(|s| s.trim().to_string()));
    if segs.len() > 1 && segs.last().map(String::as_str) == Some("self") {
        segs.pop();
    }
    segs
}

/// Rewrites a path written `inline_depth` inline modules deep so it reads
/// from the file's own module (see [`rust_use_paths`]).
fn rebase_inline(segs: Vec<String>, inline_depth: usize) -> Vec<String> {
    if inline_depth == 0 {
        return segs;
    }
    let supers = segs.iter().take_while(|s| *s == "super").count();
    let mut rest = segs[supers..].to_vec();
    let head: Vec<String> = match (supers, rest.first().map(String::as_str)) {
        (s, _) if s > inline_depth => vec!["super".to_string(); s - inline_depth],
        (s, _) if s > 0 => vec!["self".to_string()],
        (_, Some("crate")) | (_, Some("")) => Vec::new(),
        (_, Some("self")) => return vec!["self".to_string()],
        _ => vec![String::new()],
    };
    rest.splice(0..0, head);
    rest
}

/// Where one crate's module tree lives: its root file (`lib.rs`,
/// `main.rs`, a `src/bin` file, ...) and the directory its top-level
/// modules sit in (the root file's directory).
#[derive(Debug, Clone)]
struct CrateRoot {
    file: PathBuf,
    dir: PathBuf,
}

/// A file's place in a crate: which crate root owns it, the module path
/// from that root, and the library crate its package exposes (so a binary
/// can reach it by name).
#[derive(Debug, Clone)]
struct Placement {
    root: CrateRoot,
    module: Vec<String>,
}

/// One Cargo package as far as resolution needs it.
#[derive(Debug, Clone, Default)]
struct Package {
    lib_name: Option<String>,
    lib: Option<CrateRoot>,
    roots: Vec<CrateRoot>,
}

/// The corpus's Rust files placed in their crates' module trees, able to
/// resolve a file's expanded `use` paths (from [`rust_use_paths`]) to the
/// other corpus files they depend on.
pub struct RustModuleIndex {
    files: HashMap<PathBuf, String>,
    placements: HashMap<PathBuf, Placement>,
    crates_by_name: HashMap<String, CrateRoot>,
}

impl RustModuleIndex {
    /// Indexes every `.rs` path in `paths` (others are ignored), reading
    /// the nearest `Cargo.toml` of each for crate names and roots.
    pub fn build<'a>(paths: impl IntoIterator<Item = &'a str>) -> Self {
        let files: HashMap<PathBuf, String> = paths
            .into_iter()
            .filter(|p| p.ends_with(".rs"))
            .map(|p| (normalize(Path::new(p)), p.to_string()))
            .collect();
        let packages = discover_packages(files.keys());
        let placements = place_files(&files, &packages);
        let crates_by_name = packages
            .values()
            .filter_map(|pkg| Some((pkg.lib_name.clone()?, pkg.lib.clone()?)))
            .filter(|(_, lib)| files.contains_key(&lib.file))
            .collect();
        RustModuleIndex {
            files,
            placements,
            crates_by_name,
        }
    }

    /// The distinct corpus files `path`'s `use` paths resolve to, never
    /// `path` itself; `None` when `path` could not be placed in any crate,
    /// so the caller can fall back to a crate-less heuristic.
    pub fn resolve(&self, path: &str, uses: &[String]) -> Option<HashSet<String>> {
        let key = normalize(Path::new(path));
        let placement = self.placements.get(&key)?;
        let targets = uses
            .iter()
            .filter_map(|u| self.resolve_use(placement, u))
            .filter(|t| **t != key)
            .filter_map(|t| self.files.get(t).cloned())
            .collect();
        Some(targets)
    }

    fn resolve_use<'a>(&'a self, from: &'a Placement, path: &str) -> Option<&'a PathBuf> {
        let segs: Vec<&str> = path.split("::").collect();
        let (root, base, rest) = self.anchor(from, &segs)?;
        self.deepest_module_file(root, base, rest)
    }

    /// The crate root and module a path starts from, and the segments
    /// still to walk: `crate`, `self`, `super`, a corpus crate's name, or
    /// (edition 2018) a child module of the current one.
    fn anchor<'a, 's>(
        &'a self,
        from: &'a Placement,
        segs: &'s [&'s str],
    ) -> Option<(&'a CrateRoot, Vec<String>, &'s [&'s str])> {
        let first = *segs.first()?;
        match first {
            "crate" => Some((&from.root, Vec::new(), &segs[1..])),
            "self" => Some((&from.root, from.module.clone(), &segs[1..])),
            "super" => self.anchor_super(from, segs),
            "" => self.anchor_extern(segs.get(1..)?),
            _ if self.is_child_module(from, first) => Some((&from.root, from.module.clone(), segs)),
            _ => self.anchor_extern(segs),
        }
    }

    fn anchor_super<'a, 's>(
        &'a self,
        from: &'a Placement,
        segs: &'s [&'s str],
    ) -> Option<(&'a CrateRoot, Vec<String>, &'s [&'s str])> {
        let ups = segs.iter().take_while(|s| **s == "super").count();
        let keep = from.module.len().checked_sub(ups)?;
        Some((&from.root, from.module[..keep].to_vec(), &segs[ups..]))
    }

    fn anchor_extern<'s>(
        &self,
        segs: &'s [&'s str],
    ) -> Option<(&CrateRoot, Vec<String>, &'s [&'s str])> {
        let root = self.crates_by_name.get(*segs.first()?)?;
        Some((root, Vec::new(), &segs[1..]))
    }

    fn is_child_module(&self, from: &Placement, name: &str) -> bool {
        let mut module = from.module.clone();
        module.push(name.to_string());
        self.module_file(&from.root, &module).is_some()
    }

    /// Walks `rest` down from `base`, keeping the last module that has a
    /// file in the corpus. Stops at the first segment that is not a module
    /// file (an item, or an inline module), since nothing below it can be.
    fn deepest_module_file(
        &self,
        root: &CrateRoot,
        mut module: Vec<String>,
        rest: &[&str],
    ) -> Option<&PathBuf> {
        let mut found = self.module_file(root, &module);
        for seg in rest {
            module.push(seg.to_string());
            match self.module_file(root, &module) {
                Some(f) => found = Some(f),
                None => break,
            }
        }
        found
    }

    fn module_file(&self, root: &CrateRoot, module: &[String]) -> Option<&PathBuf> {
        if module.is_empty() {
            return self.files.get_key_value(&root.file).map(|(k, _)| k);
        }
        let mut dir = root.dir.clone();
        dir.extend(module);
        let flat = dir.with_extension("rs");
        let nested = dir.join("mod.rs");
        [flat, nested]
            .into_iter()
            .find_map(|f| self.files.get_key_value(&f).map(|(k, _)| k))
    }
}

/// `path` with `.` components removed, so `./src/a.rs` and `src/a.rs`
/// compare equal. Not canonicalized: symlinks and `..` are left as given.
fn normalize(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

/// Every package (keyed by its directory) that owns a corpus file,
/// found through each file's nearest `Cargo.toml`.
fn discover_packages<'a>(files: impl Iterator<Item = &'a PathBuf>) -> HashMap<PathBuf, Package> {
    let mut packages: HashMap<PathBuf, Package> = HashMap::new();
    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    for file in files {
        let Some(dir) = nearest_manifest_dir(file, &mut seen_dirs) else {
            continue;
        };
        packages
            .entry(dir)
            .or_insert_with_key(|dir| read_package(dir));
    }
    packages
}

fn nearest_manifest_dir(file: &Path, seen: &mut HashSet<PathBuf>) -> Option<PathBuf> {
    let found = file
        .ancestors()
        .skip(1)
        .find(|dir| seen.contains(*dir) || manifest_path(dir).is_file())?;
    seen.insert(found.to_path_buf());
    Some(found.to_path_buf())
}

fn manifest_path(dir: &Path) -> PathBuf {
    if dir.as_os_str().is_empty() {
        PathBuf::from("Cargo.toml")
    } else {
        dir.join("Cargo.toml")
    }
}

/// Reads the library name and the declared library/binary paths from
/// `dir/Cargo.toml`, filling in Cargo's defaults. A manifest that does
/// not parse yields a package with only the default roots.
fn read_package(dir: &Path) -> Package {
    let manifest: toml::Value = fs::read_to_string(manifest_path(dir))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(toml::Value::Table(Default::default()));
    let lib_path = table_str(&manifest, "lib", "path").unwrap_or("src/lib.rs");
    let lib = crate_root(dir.join(lib_path));
    let mut roots = vec![lib.clone()];
    roots.extend(declared_bin_roots(dir, &manifest));
    roots.push(crate_root(dir.join("src/main.rs")));
    Package {
        lib_name: lib_name(&manifest),
        lib: Some(lib),
        roots,
    }
}

fn table_str<'v>(manifest: &'v toml::Value, table: &str, key: &str) -> Option<&'v str> {
    manifest.get(table)?.get(key)?.as_str()
}

/// `[lib] name`, else `[package] name` with `-` read as `_`, as rustc
/// spells the crate in a path.
fn lib_name(manifest: &toml::Value) -> Option<String> {
    let name =
        table_str(manifest, "lib", "name").or_else(|| table_str(manifest, "package", "name"))?;
    Some(name.replace('-', "_"))
}

fn declared_bin_roots(dir: &Path, manifest: &toml::Value) -> Vec<CrateRoot> {
    let Some(bins) = manifest.get("bin").and_then(|b| b.as_array()) else {
        return Vec::new();
    };
    bins.iter()
        .filter_map(|b| b.get("path")?.as_str())
        .map(|p| crate_root(dir.join(p)))
        .collect()
}

fn crate_root(file: PathBuf) -> CrateRoot {
    let file = normalize(&file);
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    CrateRoot { file, dir }
}

/// Places every corpus file: under its package's roots when it has a
/// `Cargo.toml`, otherwise under the nearest directory holding a corpus
/// `lib.rs` or `main.rs`.
fn place_files(
    files: &HashMap<PathBuf, String>,
    packages: &HashMap<PathBuf, Package>,
) -> HashMap<PathBuf, Placement> {
    files
        .keys()
        .filter_map(|f| Some((f.clone(), place_file(f, files, packages)?)))
        .collect()
}

fn place_file(
    file: &Path,
    files: &HashMap<PathBuf, String>,
    packages: &HashMap<PathBuf, Package>,
) -> Option<Placement> {
    let package = file
        .ancestors()
        .skip(1)
        .find_map(|dir| Some((dir, packages.get(dir)?)));
    let roots: Vec<CrateRoot> = match package {
        Some((dir, pkg)) => package_roots_for(file, dir, pkg, files),
        None => loose_roots_for(file, files),
    }
    .into_iter()
    .filter(|r| files.contains_key(&r.file))
    .collect();
    if let Some(own) = roots.iter().find(|r| r.file == file) {
        return placement_under(file, own.clone());
    }
    roots
        .into_iter()
        .find_map(|root| placement_under(file, root))
}

/// The package's roots plus the implicit ones Cargo discovers for `file`
/// (see [`implicit_roots_for`]), deepest module directory first so the
/// closest root owns a file; a library root precedes a binary root that
/// shares its directory.
fn package_roots_for(
    file: &Path,
    pkg_dir: &Path,
    pkg: &Package,
    files: &HashMap<PathBuf, String>,
) -> Vec<CrateRoot> {
    let mut roots = implicit_roots_for(file, pkg_dir, files);
    roots.extend(pkg.roots.iter().cloned());
    roots.sort_by_key(|r| std::cmp::Reverse(r.dir.components().count()));
    roots
}

/// Cargo's per-target roots: a file directly in `src/bin`, `tests`,
/// `examples` or `benches` is a crate root, and so is `main.rs` in a
/// directory directly below one of them. A deeper file with no such
/// `main.rs` is a module the target directory's own roots share (e.g.
/// `tests/common/mod.rs`), so the first of those, by name, owns it.
fn implicit_roots_for(
    file: &Path,
    pkg_dir: &Path,
    files: &HashMap<PathBuf, String>,
) -> Vec<CrateRoot> {
    let mut roots = Vec::new();
    for target in ["src/bin", "tests", "examples", "benches"].map(|t| pkg_dir.join(t)) {
        let Some(first) = sub_dir_of(file, &target) else {
            continue;
        };
        if file.parent() == Some(target.as_path()) {
            roots.push(crate_root(file.to_path_buf()));
            continue;
        }
        roots.push(crate_root(target.join(first).join("main.rs")));
        roots.extend(first_direct_file(&target, files).map(crate_root));
    }
    roots
}

fn first_direct_file(dir: &Path, files: &HashMap<PathBuf, String>) -> Option<PathBuf> {
    files
        .keys()
        .filter(|f| f.parent() == Some(dir))
        .min()
        .cloned()
}

/// The first path component of `file` below `dir`, if `file` is under it.
fn sub_dir_of(file: &Path, dir: &Path) -> Option<PathBuf> {
    let rel = file.strip_prefix(dir).ok()?;
    rel.components()
        .next()
        .map(|c| PathBuf::from(c.as_os_str()))
}

fn loose_roots_for(file: &Path, files: &HashMap<PathBuf, String>) -> Vec<CrateRoot> {
    file.ancestors()
        .skip(1)
        .flat_map(|dir| ["lib.rs", "main.rs"].map(|n| crate_root(dir.join(n))))
        .filter(|r| files.contains_key(&r.file))
        .take(2)
        .collect()
}

/// `file`'s module path under `root`, if `root`'s directory contains it.
fn placement_under(file: &Path, root: CrateRoot) -> Option<Placement> {
    if file == root.file {
        return Some(Placement {
            root,
            module: Vec::new(),
        });
    }
    let rel = file.strip_prefix(&root.dir).ok()?;
    let module = module_path_of(rel)?;
    Some(Placement { root, module })
}

/// `a/b.rs` and `a/b/mod.rs` are module `a::b`.
fn module_path_of(rel: &Path) -> Option<Vec<String>> {
    let mut segs: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let last = segs.pop()?;
    if last != "mod.rs" {
        segs.push(last.strip_suffix(".rs")?.to_string());
    }
    Some(segs).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uses(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&crate::tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        rust_use_paths(&tree, code.as_bytes())
    }

    #[test]
    fn use_trees_expand_to_one_path_per_name() {
        let got = uses(
            "use crate::{a::{self, B}, c::*};\n\
             use super::d as e;\n\
             use ::ext::F;\n\
             use g::{\n    // note\n    h,\n};",
        );
        assert_eq!(
            got,
            [
                "crate::a",
                "crate::a::B",
                "crate::c",
                "super::d",
                "::ext::F",
                "g::h"
            ]
        );
    }

    #[test]
    fn paths_inside_inline_modules_read_from_the_file_module() {
        let got = uses(
            "mod tests {\n\
                 use super::*;\n\
                 use super::helper::X;\n\
                 use super::super::up::Y;\n\
                 use crate::z::W;\n\
                 use self::inner::V;\n\
                 use serde::Serialize;\n\
             }",
        );
        assert_eq!(
            got,
            [
                "self",
                "self::helper::X",
                "super::up::Y",
                "crate::z::W",
                "self",
                "::serde::Serialize"
            ]
        );
    }

    #[test]
    fn a_file_under_no_crate_is_not_placed() {
        let index = RustModuleIndex::build(["/nonexistent/knots-fixture/loose.rs"]);
        let got = index.resolve(
            "/nonexistent/knots-fixture/loose.rs",
            &["crate::x".to_string()],
        );
        assert_eq!(got, None);
    }

    #[test]
    fn a_crate_without_a_manifest_is_rooted_at_its_lib_rs() {
        let files = [
            "/nonexistent/knots-fixture/src/lib.rs",
            "/nonexistent/knots-fixture/src/a.rs",
            "/nonexistent/knots-fixture/src/a/b.rs",
        ];
        let index = RustModuleIndex::build(files);
        let got = index
            .resolve(
                files[2],
                &["super::Thing".to_string(), "crate::Root".to_string()],
            )
            .unwrap();
        let want: HashSet<String> = [files[1], files[0]].map(String::from).into();
        assert_eq!(got, want);
    }
}

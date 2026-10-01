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
//! which is read only for the library's name, the declared library and
//! binary paths and the build script; `src/bin`, `tests`, `examples` and
//! `benches` roots follow Cargo's defaults. A file is left unplaced, and the
//! caller keeps the stem match for it, when it is under no `Cargo.toml`, its
//! crate root is not in the corpus, or it is the build script. A `mod`
//! declaration is not itself an edge — it defines the tree, it does not use
//! it — and is read only to give a module `main.rs` alone declares to the
//! binary when `lib.rs` shares its directory. `#[path]` attributes are not
//! followed, and an edition-2015 crate-relative path adds no edge.

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
        (s, _) if s == inline_depth => vec!["self".to_string()],
        // Climbing out of only some inline modules lands in one that is
        // still in this file.
        (s, _) if s > 0 => return vec!["self".to_string()],
        (_, Some("crate")) | (_, Some("")) => Vec::new(),
        (_, Some("self")) => return vec!["self".to_string()],
        _ => vec![String::new()],
    };
    rest.splice(0..0, head);
    rest
}

/// Where one crate's module tree lives: its root file (`lib.rs`,
/// `main.rs`, a `src/bin` file, ...) and the directory its top-level
/// modules sit in (the root file's directory). A `shared` root is a
/// directory of modules several crate roots include (`tests/common`); it
/// has no root file, so nothing resolves to the crate root from it.
#[derive(Debug, Clone, PartialEq)]
struct CrateRoot {
    file: PathBuf,
    dir: PathBuf,
    shared: bool,
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
    /// The build script, its own crate, never a module of the library.
    build_script: Option<PathBuf>,
    /// Top-level modules only a binary root sharing the library's
    /// directory declares (`main.rs: mod cli;`), and that root.
    bin_mods: HashMap<String, CrateRoot>,
}

/// The corpus's Rust files placed in their crates' module trees, able to
/// resolve a file's expanded `use` paths (from [`rust_use_paths`]) to the
/// other corpus files they depend on.
pub struct RustModuleIndex {
    files: HashMap<PathBuf, String>,
    placements: HashMap<PathBuf, Placement>,
    root_files: HashSet<PathBuf>,
    crates_by_name: HashMap<String, CrateRoot>,
}

impl RustModuleIndex {
    /// Indexes every `.rs` path in `paths` (others are ignored), reading
    /// the nearest `Cargo.toml` of each for crate names and roots.
    pub fn build<'a>(paths: impl IntoIterator<Item = &'a str>) -> Self {
        let files = rust_files(paths);
        let packages = discover_packages(files.keys());
        let placements = place_files(&files, &packages);
        RustModuleIndex {
            crates_by_name: unique_lib_names(&packages, &files),
            root_files: root_files(&placements),
            files,
            placements,
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

    /// The corpus file holding `module` under `root`. Another crate's
    /// root file (`tests/x.rs`, `src/bin/x.rs`) is never a module.
    fn module_file(&self, root: &CrateRoot, module: &[String]) -> Option<&PathBuf> {
        if module.is_empty() {
            return self.corpus_key(&root.file);
        }
        module_candidates(root, module)
            .into_iter()
            .filter(|f| !self.root_files.contains(f))
            .find_map(|f| self.corpus_key(&f))
    }

    /// `file` as the index stores it, if it is in the corpus.
    fn corpus_key(&self, file: &Path) -> Option<&PathBuf> {
        self.files.get_key_value(file).map(|(k, _)| k)
    }
}

/// `module`'s two possible files under `root`: `a/b.rs` and `a/b/mod.rs`.
fn module_candidates(root: &CrateRoot, module: &[String]) -> [PathBuf; 2] {
    let mut dir = root.dir.clone();
    dir.extend(module);
    [dir.with_extension("rs"), dir.join("mod.rs")]
}

/// Every `.rs` path in `paths`, keyed by its normalized form.
fn rust_files<'a>(paths: impl IntoIterator<Item = &'a str>) -> HashMap<PathBuf, String> {
    paths
        .into_iter()
        .filter(|p| p.ends_with(".rs"))
        .map(|p| (normalize(Path::new(p)), p.to_string()))
        .collect()
}

/// The files placed as a crate root (with an empty module path).
fn root_files(placements: &HashMap<PathBuf, Placement>) -> HashSet<PathBuf> {
    placements
        .iter()
        .filter(|(_, p)| p.module.is_empty())
        .map(|(f, _)| f.clone())
        .collect()
}

/// Each library name exactly one package with its library in the corpus
/// claims. A name two packages share (vendored copies, fixtures) names
/// neither, as an ambiguous stem does in the generic resolver.
fn unique_lib_names(
    packages: &HashMap<PathBuf, Package>,
    files: &HashMap<PathBuf, String>,
) -> HashMap<String, CrateRoot> {
    libs_by_name(packages)
        .into_iter()
        .filter_map(|(name, libs)| Some((name, only(libs)?)))
        .filter(|(_, lib)| files.contains_key(&lib.file))
        .collect()
}

fn libs_by_name(packages: &HashMap<PathBuf, Package>) -> HashMap<String, Vec<CrateRoot>> {
    let mut by_name: HashMap<String, Vec<CrateRoot>> = HashMap::new();
    for pkg in packages.values() {
        if let (Some(name), Some(lib)) = (&pkg.lib_name, &pkg.lib) {
            by_name.entry(name.clone()).or_default().push(lib.clone());
        }
    }
    by_name
}

/// The single element of `items`, if there is exactly one.
fn only<T>(mut items: Vec<T>) -> Option<T> {
    (items.len() == 1).then(|| items.remove(0))
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
    let mut has_manifest: HashMap<PathBuf, bool> = HashMap::new();
    for file in files {
        let Some(dir) = nearest_manifest_dir(file, &mut has_manifest) else {
            continue;
        };
        packages
            .entry(dir)
            .or_insert_with_key(|dir| read_package(dir));
    }
    packages
}

/// The nearest ancestor of `file` holding a `Cargo.toml`, remembering the
/// answer for every directory it checks.
fn nearest_manifest_dir(file: &Path, has_manifest: &mut HashMap<PathBuf, bool>) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|dir| {
            *has_manifest
                .entry(dir.to_path_buf())
                .or_insert_with(|| manifest_path(dir).is_file())
        })
        .map(Path::to_path_buf)
}

fn manifest_path(dir: &Path) -> PathBuf {
    if dir.as_os_str().is_empty() {
        PathBuf::from("Cargo.toml")
    } else {
        dir.join("Cargo.toml")
    }
}

/// Reads the library name, the declared library/binary paths and the build
/// script from `dir/Cargo.toml`, filling in Cargo's defaults. A manifest
/// that does not parse yields a package with only the default roots.
fn read_package(dir: &Path) -> Package {
    let manifest = read_manifest(dir);
    let lib_path = table_str(&manifest, "lib", "path").unwrap_or("src/lib.rs");
    let lib = crate_root(dir.join(lib_path));
    let roots = package_roots(dir, &manifest, &lib);
    Package {
        lib_name: lib_name(&manifest),
        bin_mods: bin_only_mods(&lib, &roots),
        build_script: build_script(dir, &manifest),
        lib: Some(lib),
        roots,
    }
}

fn read_manifest(dir: &Path) -> toml::Value {
    fs::read_to_string(manifest_path(dir))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(toml::Value::Table(Default::default()))
}

/// The library, the declared binaries, then the default `src/main.rs`.
fn package_roots(dir: &Path, manifest: &toml::Value, lib: &CrateRoot) -> Vec<CrateRoot> {
    let mut roots = vec![lib.clone()];
    roots.extend(declared_bin_roots(dir, manifest));
    roots.push(crate_root(dir.join("src/main.rs")));
    roots
}

/// `[package] build`, else Cargo's default `build.rs`; `build = false`
/// means none.
fn build_script(dir: &Path, manifest: &toml::Value) -> Option<PathBuf> {
    match manifest.get("package").and_then(|p| p.get("build")) {
        Some(toml::Value::String(path)) => Some(normalize(&dir.join(path))),
        Some(_) => None,
        None => Some(normalize(&dir.join("build.rs"))),
    }
}

/// For each binary root sharing the library's directory, the top-level
/// modules it declares and the library does not. Read from the two root
/// files' `mod name;` lines only to decide which root owns a file both
/// could; a module either both or neither declare stays the library's.
fn bin_only_mods(lib: &CrateRoot, roots: &[CrateRoot]) -> HashMap<String, CrateRoot> {
    let lib_mods = declared_mods(&lib.file);
    let mut out = HashMap::new();
    for bin in bins_beside(lib, roots) {
        for name in declared_mods(&bin.file).difference(&lib_mods) {
            out.entry(name.clone()).or_insert_with(|| bin.clone());
        }
    }
    out
}

fn bins_beside<'r>(
    lib: &'r CrateRoot,
    roots: &'r [CrateRoot],
) -> impl Iterator<Item = &'r CrateRoot> {
    roots
        .iter()
        .filter(move |r| r.dir == lib.dir && r.file != lib.file)
}

/// The names of `file`'s top-level `mod name;` declarations (not inline
/// `mod name { ... }`), or none when it can't be read or parsed.
fn declared_mods(file: &Path) -> HashSet<String> {
    let Ok(source) = fs::read_to_string(file) else {
        return HashSet::new();
    };
    parse_rust(&source)
        .map(|tree| mod_declaration_names(tree.root_node(), source.as_bytes()))
        .unwrap_or_default()
}

fn parse_rust(source: &str) -> Option<Tree> {
    let mut parser = tree_sitter::Parser::new();
    let language: tree_sitter::Language = crate::tree_sitter_rust::LANGUAGE.into();
    parser.set_language(&language).ok()?;
    parser.parse(source, None)
}

fn mod_declaration_names(root: Node, source: &[u8]) -> HashSet<String> {
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .filter(is_mod_declaration)
        .filter_map(|n| n.child_by_field_name("name"))
        .map(|n| node_text(n, source).to_string())
        .collect()
}

/// `mod name;`, as opposed to an inline `mod name { ... }`.
fn is_mod_declaration(node: &Node) -> bool {
    node.kind() == "mod_item" && node.child_by_field_name("body").is_none()
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
    CrateRoot {
        file,
        dir,
        shared: false,
    }
}

fn shared_root(dir: PathBuf) -> CrateRoot {
    let dir = normalize(&dir);
    CrateRoot {
        file: dir.clone(),
        dir,
        shared: true,
    }
}

/// Places every corpus file under a `Cargo.toml` among its package's
/// roots. A file under none is left unplaced.
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
    let (pkg_dir, pkg) = package_of(file, packages)?;
    if pkg.build_script.as_deref() == Some(file) {
        return None;
    }
    let placement = candidate_roots(file, pkg_dir, pkg, files)
        .into_iter()
        .find_map(|root| placement_under(file, root))?;
    Some(owned_by_bin(placement, pkg, files))
}

/// The roots that could own `file`, present in the corpus (or shared), in
/// the order to try them. A file that is itself a crate root is that root,
/// not a module of a shallower one, so its own root comes first.
fn candidate_roots(
    file: &Path,
    pkg_dir: &Path,
    pkg: &Package,
    files: &HashMap<PathBuf, String>,
) -> Vec<CrateRoot> {
    let mut roots = package_roots_for(file, pkg_dir, pkg);
    roots.retain(|r| r.shared || files.contains_key(&r.file));
    roots.sort_by_key(|r| r.file != file);
    roots
}

/// Moves a library-placed file to the binary root that alone declares its
/// top-level module, when the two roots share a directory.
fn owned_by_bin(
    placement: Placement,
    pkg: &Package,
    files: &HashMap<PathBuf, String>,
) -> Placement {
    let bin = placement
        .module
        .first()
        .filter(|_| pkg.lib.as_ref() == Some(&placement.root))
        .and_then(|top| pkg.bin_mods.get(top))
        .filter(|bin| files.contains_key(&bin.file));
    match bin {
        Some(bin) => Placement {
            root: bin.clone(),
            module: placement.module,
        },
        None => placement,
    }
}

fn package_of<'p>(
    file: &'p Path,
    packages: &'p HashMap<PathBuf, Package>,
) -> Option<(&'p Path, &'p Package)> {
    file.ancestors()
        .skip(1)
        .find_map(|dir| Some((dir, packages.get(dir)?)))
}

/// The package's roots plus the implicit ones Cargo discovers for `file`
/// (see [`implicit_roots_for`]), deepest module directory first so the
/// closest root owns a file; a library root precedes a binary root that
/// shares its directory.
fn package_roots_for(file: &Path, pkg_dir: &Path, pkg: &Package) -> Vec<CrateRoot> {
    let mut roots = implicit_roots_for(file, pkg_dir);
    roots.extend(pkg.roots.iter().cloned());
    roots.sort_by_key(|r| std::cmp::Reverse(r.dir.components().count()));
    roots
}

/// Cargo's per-target roots: a file directly in `src/bin`, `tests`,
/// `examples` or `benches` is a crate root, and so is `main.rs` in a
/// directory directly below one of them. A deeper file with no such
/// `main.rs` (e.g. `tests/common/mod.rs`) is a module the target
/// directory's roots share, under a shared root.
fn implicit_roots_for(file: &Path, pkg_dir: &Path) -> Vec<CrateRoot> {
    ["src/bin", "tests", "examples", "benches"]
        .iter()
        .flat_map(|t| target_roots(file, pkg_dir.join(t)))
        .collect()
}

fn target_roots(file: &Path, target: PathBuf) -> Vec<CrateRoot> {
    let Some(first) = sub_dir_of(file, &target) else {
        return Vec::new();
    };
    if file.parent() == Some(target.as_path()) {
        return vec![crate_root(file.to_path_buf())];
    }
    vec![
        crate_root(target.join(first).join("main.rs")),
        shared_root(target),
    ]
}

/// The first path component of `file` below `dir`, if `file` is under it.
fn sub_dir_of(file: &Path, dir: &Path) -> Option<PathBuf> {
    let rel = file.strip_prefix(dir).ok()?;
    rel.components()
        .next()
        .map(|c| PathBuf::from(c.as_os_str()))
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
    fn a_super_that_stays_inside_the_inline_modules_is_this_file() {
        let got = uses(
            "mod outer {\n\
                 pub mod helper {}\n\
                 mod inner {\n\
                     use super::helper::X;\n\
                     use super::super::sibling::Y;\n\
                 }\n\
             }",
        );
        assert_eq!(got, ["self", "self::sibling::Y"]);
    }

    /// Writes `files` (path, contents) under a fresh temp directory.
    fn write_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = fresh_temp_dir(name);
        for (rel, body) in files {
            let path = dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
        dir
    }

    fn fresh_temp_dir(name: &str) -> PathBuf {
        let id = std::process::id();
        let dir = std::env::temp_dir().join(format!("knots-rust-modules-{name}-{id}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    /// Resolves `uses` from `from` over every `.rs` file in `files`, as
    /// paths relative to `dir`, sorted; `None` when `from` is unplaced.
    fn resolve_in(
        dir: &Path,
        files: &[(&str, &str)],
        from: &str,
        uses: &[&str],
    ) -> Option<Vec<String>> {
        let index = index_over(dir, files);
        let uses: Vec<String> = uses.iter().map(|u| u.to_string()).collect();
        let targets = index.resolve(&joined(dir, from), &uses)?;
        Some(relative_sorted(dir, targets))
    }

    fn index_over(dir: &Path, files: &[(&str, &str)]) -> RustModuleIndex {
        let paths: Vec<String> = files.iter().map(|(f, _)| joined(dir, f)).collect();
        RustModuleIndex::build(paths.iter().map(String::as_str))
    }

    fn joined(dir: &Path, rel: &str) -> String {
        dir.join(rel).to_string_lossy().into_owned()
    }

    fn relative_sorted(dir: &Path, targets: HashSet<String>) -> Vec<String> {
        let mut out: Vec<String> = targets
            .iter()
            .map(|t| Path::new(t).strip_prefix(dir).unwrap())
            .map(|t| t.to_string_lossy().into_owned())
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_lib_name_two_packages_share_names_neither() {
        let files = [
            ("c1/Cargo.toml", "[package]\nname = \"same\"\n"),
            ("c1/src/lib.rs", ""),
            ("c2/Cargo.toml", "[package]\nname = \"same\"\n"),
            ("c2/src/lib.rs", ""),
            ("user/Cargo.toml", "[package]\nname = \"user\"\n"),
            ("user/src/lib.rs", ""),
        ];
        let dir = write_tree("dup", &files);
        let got = resolve_in(&dir, &files, "user/src/lib.rs", &["same::S1"]);
        fs::remove_dir_all(&dir).ok();
        assert_eq!(got, Some(vec![]));
    }

    #[test]
    fn a_module_only_main_rs_declares_belongs_to_main() {
        let files = [
            ("Cargo.toml", "[package]\nname = \"pkg\"\n"),
            ("src/lib.rs", "pub mod m;\npub struct Root;\n"),
            ("src/main.rs", "mod cli;\nstruct MainThing;\nfn main() {}\n"),
            ("src/cli.rs", ""),
            ("src/m.rs", ""),
        ];
        let dir = write_tree("bin-mod", &files);
        let cli = resolve_in(
            &dir,
            &files,
            "src/cli.rs",
            &["crate::MainThing", "super::MainThing"],
        );
        let m = resolve_in(&dir, &files, "src/m.rs", &["crate::Root"]);
        fs::remove_dir_all(&dir).ok();
        assert_eq!(cli, Some(vec!["src/main.rs".to_string()]));
        assert_eq!(m, Some(vec!["src/lib.rs".to_string()]));
    }

    #[test]
    fn a_build_script_is_not_a_library_module() {
        let files = [
            (
                "Cargo.toml",
                "[package]\nname = \"nb\"\n[lib]\npath = \"lib.rs\"\n",
            ),
            ("lib.rs", "pub mod k;\n"),
            ("k.rs", ""),
            ("build.rs", "fn main() {}\n"),
        ];
        let dir = write_tree("build-rs", &files);
        let got = resolve_in(&dir, &files, "build.rs", &["crate::k::Z"]);
        fs::remove_dir_all(&dir).ok();
        assert_eq!(got, None);
    }

    #[test]
    fn a_shared_test_module_reaches_no_single_test_crate() {
        let files = [
            ("Cargo.toml", "[package]\nname = \"tc\"\n"),
            ("src/lib.rs", ""),
            ("tests/a_first.rs", "mod common;\n"),
            ("tests/b_second.rs", "mod common;\npub struct Shared;\n"),
            ("tests/common/mod.rs", ""),
            ("tests/common/util.rs", ""),
        ];
        let dir = write_tree("tests-common", &files);
        let got = resolve_in(
            &dir,
            &files,
            "tests/common/mod.rs",
            &[
                "crate::Shared",
                "super::Shared",
                "crate::b_second",
                "self::util::U",
            ],
        );
        let first = resolve_in(
            &dir,
            &files,
            "tests/a_first.rs",
            &["common::X", "crate::b_second"],
        );
        fs::remove_dir_all(&dir).ok();
        assert_eq!(got, Some(vec!["tests/common/util.rs".to_string()]));
        assert_eq!(first, Some(vec!["tests/common/mod.rs".to_string()]));
    }

    #[test]
    fn a_file_under_no_cargo_toml_is_not_placed_even_beside_a_lib_rs() {
        let files = [
            "/nonexistent/knots-fixture/src/lib.rs",
            "/nonexistent/knots-fixture/src/a.rs",
        ];
        let index = RustModuleIndex::build(files);
        assert_eq!(index.resolve(files[1], &["crate::Root".to_string()]), None);
    }
}

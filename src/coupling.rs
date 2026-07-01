//! File-level import coupling: Efferent (Ce), Afferent (Ca), and Instability
//! metrics, built on the substrate's syntactic import extraction
//! (`lang_parsing_substrate::import_sources`).
//!
//! Resolution here is a best-effort heuristic, not real module resolution:
//! each import's *candidate* module name is derived by stripping a path
//! prefix, a known source-file extension, and a namespace separator (`::`
//! or `.`) down to a single trailing identifier, then matched against every
//! corpus file's stem (`Path::file_stem`). Ambiguous stems — two files in
//! the corpus sharing one, e.g. `mod.rs` in different packages, or
//! `utils.py` and `utils.js` — resolve to no edge rather than an arbitrary
//! pick, since a wrong edge is worse than a missing one.
//!
//! Imports that don't resolve to any corpus file (third-party libraries,
//! stdlib, unresolvable dynamic `require`s) contribute no edge — Ce/Ca
//! measure coupling *within the analyzed corpus*, the same scope the
//! classic package-coupling metrics (Robert Martin's Ce/Ca/Instability)
//! assume. Counting external dependencies toward Ce would inflate every
//! file that imports a third-party library, pushing Instability toward 1.0
//! for nearly the whole corpus and erasing the signal.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::complexity::apply_aird_ce_multiplier;
use crate::FunctionMetrics;

/// Ce, Ca, and Instability for one file in the corpus.
#[derive(Debug, Clone, PartialEq)]
pub struct FileCoupling {
    pub file_path: String,
    /// Efferent coupling — distinct corpus files this file imports.
    pub ce: u32,
    /// Afferent coupling — distinct corpus files that import this file.
    pub ca: u32,
    /// Ce / (Ce + Ca). `0.0` (maximally stable) when both are zero.
    pub instability: f64,
}

/// The resolved, corpus-internal import graph: `file -> distinct corpus
/// files it imports`.
pub struct ImportGraph {
    edges: HashMap<String, HashSet<String>>,
}

/// Phase 1: resolve each file's raw import strings (as extracted by
/// `lang_parsing_substrate::import_sources`) to other files in the same
/// corpus, building the graph phase 2 derives metrics from.
pub fn build_import_graph<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a [String])>,
) -> ImportGraph {
    let known_extensions = known_source_extensions();
    let files: Vec<(&str, &[String])> = files.into_iter().collect();
    let (key_counts, key_owner) = index_module_keys(&files);

    let mut edges: HashMap<String, HashSet<String>> = HashMap::new();
    for (path, imports) in &files {
        let targets = resolve_targets(path, imports, &known_extensions, &key_counts, &key_owner);
        edges.insert(path.to_string(), targets);
    }
    ImportGraph { edges }
}

fn known_source_extensions() -> HashSet<&'static str> {
    lang_parsing_substrate::languages()
        .iter()
        .flat_map(|l| l.extensions.iter().chain(l.explicit_only.iter()))
        .copied()
        .collect()
}

/// Maps each corpus file's module key to how many files share it (so
/// ambiguous keys can be rejected) and to one owning file path.
fn index_module_keys<'a>(
    files: &[(&'a str, &[String])],
) -> (HashMap<String, u32>, HashMap<String, &'a str>) {
    let mut key_counts: HashMap<String, u32> = HashMap::new();
    let mut key_owner: HashMap<String, &str> = HashMap::new();
    for (path, _) in files {
        let key = module_key(path);
        *key_counts.entry(key.clone()).or_insert(0) += 1;
        key_owner.insert(key, path);
    }
    (key_counts, key_owner)
}

/// Resolves one file's raw imports to the distinct set of other corpus
/// files they name, skipping unresolved and ambiguous candidates.
fn resolve_targets(
    path: &str,
    imports: &[String],
    known_extensions: &HashSet<&str>,
    key_counts: &HashMap<String, u32>,
    key_owner: &HashMap<String, &str>,
) -> HashSet<String> {
    let mut targets = HashSet::new();
    for import in imports {
        let candidate = resolve_candidate(import, known_extensions);
        if key_counts.get(&candidate).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(&target) = key_owner.get(&candidate) {
            if target != path {
                targets.insert(target.to_string());
            }
        }
    }
    targets
}

impl ImportGraph {
    /// Phase 2: derive Ce/Ca/Instability for every file from the graph.
    pub fn coupling(&self) -> Vec<FileCoupling> {
        let ca_counts = count_incoming_edges(&self.edges);
        let mut out: Vec<FileCoupling> = self
            .edges
            .iter()
            .map(|(path, targets)| file_coupling(path, targets, &ca_counts))
            .collect();
        out.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        out
    }
}

fn count_incoming_edges(edges: &HashMap<String, HashSet<String>>) -> HashMap<&str, u32> {
    let mut ca_counts: HashMap<&str, u32> = HashMap::new();
    for targets in edges.values() {
        for t in targets {
            *ca_counts.entry(t.as_str()).or_insert(0) += 1;
        }
    }
    ca_counts
}

fn file_coupling(
    path: &str,
    targets: &HashSet<String>,
    ca_counts: &HashMap<&str, u32>,
) -> FileCoupling {
    let ce = targets.len() as u32;
    let ca = ca_counts.get(path).copied().unwrap_or(0);
    let instability = if ce + ca == 0 {
        0.0
    } else {
        ce as f64 / (ce + ca) as f64
    };
    FileCoupling {
        file_path: path.to_string(),
        ce,
        ca,
        instability,
    }
}

/// Folds each function's file-level Ce into its AIRD score via
/// [`apply_aird_ce_multiplier`], and records the raw Ce on `file_ce` so
/// violation output can show it as a distinct driver. Run as a post-pass
/// after `collect_all_metrics`, once the corpus-wide import graph exists —
/// AIRD itself is computed eagerly per-function, before any file outside the
/// current one is known.
pub fn apply_file_ce_to_aird(metrics: &mut [FunctionMetrics], coupling: &[FileCoupling]) {
    let ce_by_file = index_ce_by_file(coupling);
    for func in metrics.iter_mut() {
        let ce = ce_by_file
            .get(func.file_path.as_str())
            .copied()
            .unwrap_or(0);
        func.file_ce = ce;
        func.aird = apply_aird_ce_multiplier(func.aird, ce);
    }
}

fn index_ce_by_file(coupling: &[FileCoupling]) -> HashMap<&str, u32> {
    coupling
        .iter()
        .map(|c| (c.file_path.as_str(), c.ce))
        .collect()
}

fn module_key(file_path: &str) -> String {
    Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Reduces a raw import string down to the single trailing identifier most
/// likely to match a corpus file's stem: strip a path prefix, then a known
/// source-file extension, then a namespace separator (`::` or `.`).
fn resolve_candidate(import: &str, known_extensions: &HashSet<&str>) -> String {
    let mut candidate = import.trim();

    if let Some(idx) = candidate.rfind(['/', '\\']) {
        candidate = &candidate[idx + 1..];
    }

    if let Some(idx) = candidate.rfind('.') {
        let ext = &candidate[idx + 1..];
        if known_extensions.contains(ext) {
            candidate = &candidate[..idx];
        }
    }

    if let Some(idx) = candidate.rfind("::") {
        candidate = &candidate[idx + 2..];
    } else if let Some(idx) = candidate.rfind('.') {
        candidate = &candidate[idx + 1..];
    }

    candidate.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_js_import_by_stem() {
        let files = [
            ("src/main.js", vec!["./bar".to_string()]),
            ("src/bar.js", vec![]),
        ];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        let main = coupling
            .iter()
            .find(|c| c.file_path == "src/main.js")
            .unwrap();
        let bar = coupling
            .iter()
            .find(|c| c.file_path == "src/bar.js")
            .unwrap();
        assert_eq!(main.ce, 1);
        assert_eq!(main.ca, 0);
        assert_eq!(bar.ce, 0);
        assert_eq!(bar.ca, 1);
    }

    #[test]
    fn resolves_java_style_import_by_class_name() {
        let files = [
            ("src/Main.java", vec!["com.example.util.Helper".to_string()]),
            ("src/util/Helper.java", vec![]),
        ];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        let main = coupling
            .iter()
            .find(|c| c.file_path == "src/Main.java")
            .unwrap();
        assert_eq!(main.ce, 1);
    }

    #[test]
    fn external_imports_do_not_create_edges() {
        let files = [("src/main.py", vec!["numpy".to_string()])];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        let main = &coupling[0];
        assert_eq!(main.ce, 0);
        assert_eq!(main.ca, 0);
        assert_eq!(main.instability, 0.0);
    }

    #[test]
    fn ambiguous_stem_resolves_to_no_edge() {
        let files = [
            ("pkg_a/utils.py", vec!["utils".to_string()]),
            ("pkg_b/utils.py", vec![]),
            ("pkg_c/main.py", vec!["utils".to_string()]),
        ];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        for c in &coupling {
            assert_eq!(c.ce, 0, "{} should have no resolved edges", c.file_path);
            assert_eq!(c.ca, 0, "{} should have no resolved edges", c.file_path);
        }
    }

    #[test]
    fn instability_reflects_ce_ca_ratio() {
        // a imports b (a is unstable: ce=1, ca=0 -> instability 1.0)
        // b is imported by a only (b is stable: ce=0, ca=1 -> instability 0.0)
        let files = [("a.py", vec!["b".to_string()]), ("b.py", vec![])];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        let a = coupling.iter().find(|c| c.file_path == "a.py").unwrap();
        let b = coupling.iter().find(|c| c.file_path == "b.py").unwrap();
        assert_eq!(a.instability, 1.0);
        assert_eq!(b.instability, 0.0);
    }

    #[test]
    fn c_header_include_resolves_by_stripped_extension() {
        // `lang_parsing_substrate::import_sources` already strips the
        // surrounding quotes/angle-brackets from `#include`, so raw import
        // strings never carry them here.
        let files = [
            ("src/main.c", vec!["foo.h".to_string()]),
            ("src/foo.h", vec![]),
        ];
        let refs: Vec<(&str, &[String])> = files.iter().map(|(p, i)| (*p, i.as_slice())).collect();
        let graph = build_import_graph(refs);
        let coupling = graph.coupling();
        let main = coupling
            .iter()
            .find(|c| c.file_path == "src/main.c")
            .unwrap();
        assert_eq!(main.ce, 1);
    }

    #[test]
    fn apply_file_ce_to_aird_bumps_high_ce_file_and_records_ce() {
        let coupling = vec![
            FileCoupling {
                file_path: "src/hub.rs".to_string(),
                ce: 8,
                ca: 0,
                instability: 1.0,
            },
            FileCoupling {
                file_path: "src/leaf.rs".to_string(),
                ce: 0,
                ca: 1,
                instability: 0.0,
            },
        ];
        let mut metrics = vec![
            test_function_metrics("src/hub.rs", 50),
            test_function_metrics("src/leaf.rs", 50),
        ];
        apply_file_ce_to_aird(&mut metrics, &coupling);
        assert_eq!(metrics[0].file_ce, 8);
        assert!(metrics[0].aird > 50);
        assert_eq!(metrics[1].file_ce, 0);
        assert_eq!(metrics[1].aird, 50);
    }

    fn test_function_metrics(file_path: &str, aird: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: "f".to_string(),
            file_path: file_path.to_string(),
            start_line: 1,
            end_line: 2,
            mccabe: 0,
            cognitive: 0,
            nesting: 0,
            sloc: 0,
            abc_magnitude: 0.0,
            return_count: 0,
            test_scoring: crate::complexity::TestScoringMetric {
                signature_score: 0,
                dependency_score: 0,
                observable_score: 0,
                implementation_score: 0,
                documentation_score: 0,
                total_score: 0,
            },
            aird,
            aicp: 0,
            external_calls: 0,
            state_coupling: 0,
            file_ce: 0,
            suppressed: HashSet::new(),
        }
    }
}

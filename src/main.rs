use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Tree, TreeCursor};
use walkdir::WalkDir;

mod complexity;
use complexity::{
    calculate_abc_complexity, calculate_aicp, calculate_aird, calculate_cognitive_complexity,
    calculate_state_coupling,
    calculate_mccabe_complexity, calculate_nesting_depth, calculate_return_count, calculate_sloc,
    calculate_sloc_python, calculate_test_scoring, TestScoringMetric,
};
use knots::{is_source_extension, language_for_file};

fn get_complexity_emoji(complexity: u32) -> &'static str {
    match complexity {
        1..=10 => "😊",  // Smiley - good complexity
        11..=20 => "😐", // Neutral - okay complexity
        21..=49 => "😠", // Angry - bad complexity
        _ => "😢",       // Sad - worst complexity (50+)
    }
}

/// Compilation database entry from compile_commands.json
#[derive(Debug, Clone, Deserialize)]
struct CompileCommand {
    #[serde(default)]
    directory: String,
    #[serde(default)]
    _command: String,
    file: String,
    #[serde(default)]
    #[serde(rename = "arguments")]
    _arguments: Vec<String>,
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
    fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read filter file: {}", path.display()))?;
        let rules: FilterRules = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse filter JSON: {}", path.display()))?;
        Ok(rules)
    }

    /// Check if a file path matches the patterns
    fn matches_file(&self, file_path: &str) -> bool {
        if self.file_patterns.is_empty() {
            return true;
        }

        let mut included = false;
        let mut excluded = false;

        for pattern in &self.file_patterns {
            if let Some(neg_pattern) = pattern.strip_prefix('!') {
                // Negation pattern - exclude
                if glob_match(neg_pattern, file_path) {
                    excluded = true;
                }
            } else {
                // Include pattern
                if glob_match(pattern, file_path) {
                    included = true;
                }
            }
        }

        // If we have include patterns, file must match at least one
        // Then check if it's explicitly excluded
        if !self.file_patterns.iter().any(|p| !p.starts_with('!')) {
            // No positive patterns, only negative ones
            !excluded
        } else {
            included && !excluded
        }
    }

    /// Check if a function name matches the patterns
    fn matches_function(&self, function_name: &str) -> bool {
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
    fn matches_complexity(&self, complexity: u32) -> bool {
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

/// Simple glob matching (supports * and **)
fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_regex = pattern
        .replace(".", "\\.")
        .replace("**", "<!DOUBLESTAR!>")
        .replace("*", "[^/]*")
        .replace("<!DOUBLESTAR!>", ".*");

    if let Ok(re) = Regex::new(&format!("^{}$", pattern_regex)) {
        re.is_match(path)
    } else {
        false
    }
}

#[derive(Parser, Debug)]
#[command(name = "knots")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Analyzes source code complexity with visual indicators: 😊 (1-10), 😐 (11-20), 😠 (21-49), 😢 (50+)", long_about = None)]
// Last-wins on repeated flags — allows a consumer who bakes thresholds
// into a wrapper script or local hook override to append a second
// occurrence and have it win rather than error.
#[command(args_override_self = true)]
struct Args {
    /// Path(s) to source files or directories to analyze
    #[arg(
        value_name = "FILE",
        required_unless_present_any = ["compile_commands", "explain"],
        num_args = 1..
    )]
    files: Vec<PathBuf>,

    /// Recursively process all supported source files in directories
    #[arg(short, long)]
    recursive: bool,

    /// Use compile_commands.json to get list of files to analyze (C/C++ only; generated by CMake, Bear, etc.)
    #[arg(long, value_name = "FILE", conflicts_with = "files")]
    compile_commands: Option<PathBuf>,

    /// Show detailed per-function analysis
    #[arg(short, long)]
    verbose: bool,

    /// Show testability matrix categorization
    #[arg(short, long)]
    matrix: bool,

    /// Include filter rules from JSON file (whitelist files/functions)
    #[arg(long, value_name = "FILE")]
    include: Option<PathBuf>,

    /// Exclude filter rules from JSON file (blacklist files/functions)
    #[arg(long, value_name = "FILE")]
    exclude: Option<PathBuf>,

    /// Exclude files whose path matches this regex (repeatable; same syntax as pre-commit exclude:)
    #[arg(long, value_name = "PATTERN", action = clap::ArgAction::Append)]
    exclude_path: Vec<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Exit 1 if any function exceeds this McCabe complexity (default: off)
    #[arg(long, value_name = "N")]
    mccabe_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this cognitive complexity (default: off)
    #[arg(long, value_name = "N")]
    cognitive_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this nesting depth (default: off)
    #[arg(long, value_name = "N")]
    nesting_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this SLOC count (default: off)
    #[arg(long, value_name = "N")]
    sloc_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this ABC magnitude (default: off)
    #[arg(long, value_name = "F")]
    abc_threshold: Option<f64>,

    /// Exit 1 if any function exceeds this return count (default: off)
    #[arg(long, value_name = "N")]
    return_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this AIRD (AI Reasoning Difficulty) score
    /// (default: off, recommended: 85). Run `knots --explain aird`.
    #[arg(long, value_name = "N")]
    aird_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this AICP (AI Context Pressure) score
    /// (default: off). Run `knots --explain aicp`.
    #[arg(long, value_name = "N")]
    aicp_threshold: Option<u32>,

    /// Exit 1 if any function exceeds this external call count (default: off)
    #[arg(long, value_name = "N")]
    external_calls_threshold: Option<u32>,

    /// Write detailed per-function report to this file (opt-in; omit to suppress the file)
    #[arg(long, value_name = "FILE")]
    report: Option<PathBuf>,

    /// Ratchet against a baseline file: gate only on regressions (a new
    /// over-threshold function, or a baselined one whose score got worse).
    /// Combine with --write-baseline to (re)generate the file.
    #[arg(long, value_name = "FILE")]
    baseline: Option<PathBuf>,

    /// Snapshot current per-function scores to the --baseline file and exit
    /// without gating. Use to adopt the gate on a legacy codebase.
    #[arg(long, requires = "baseline")]
    write_baseline: bool,

    /// Scope threshold gating to functions that overlap lines changed since
    /// this git ref (e.g. HEAD, main, a commit SHA). Untouched pre-existing
    /// offenders are ignored. Compares the working tree against <REF>.
    #[arg(long, value_name = "REF", conflicts_with = "changed")]
    since: Option<String>,

    /// Scope threshold gating to functions you have changed in the working
    /// tree (uncommitted edits, plus untracked files). Sugar for `--since HEAD`.
    #[arg(long)]
    changed: bool,

    /// Explain what a metric measures and how to lower it, then exit. No files
    /// needed. E.g. `knots --explain aird`.
    #[arg(long, value_name = "METRIC", value_enum)]
    explain: Option<ExplainMetric>,
}

/// Metrics that `--explain` can describe at the command line.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ExplainMetric {
    Mccabe,
    Cognitive,
    Nesting,
    Sloc,
    Abc,
    Returns,
    Aird,
    Aicp,
    ExternalCalls,
}

/// A terminal-friendly explanation of a metric: what it measures and how to
/// lower it. Distilled from docs/metrics-reference.rst so a user who meets
/// "AIRD 98 > 85" mid-commit doesn't have to leave the terminal.
fn explain_metric(metric: ExplainMetric) -> &'static str {
    match metric {
        ExplainMetric::Aird => "\
AIRD — AI Reasoning Difficulty (0–100)

Predicts how much reasoning effort an AI model needs to safely modify a
function. Higher = harder. Cognitive complexity dominates the score.

  AIRD = cognitive/75 ×55 + sloc/200 ×15 + nesting/8 ×15
       + test_score/20 ×15 − doc_score/10 ×15        (clamped 0–100)

Recommended CI threshold: 85 (validated against Sonnet 4.6 and Opus 4.8).

Lower it by: reducing cognitive complexity first — it carries 55 of the 100
points. Extract deeply nested branches into well-named helpers, then trim
SLOC and nesting. Adding doc comments reduces the score.",

        ExplainMetric::Aicp => "\
AICP — AI Context Pressure (0–100)

Predicts how much surrounding context an AI must load before it can act.
Complements AIRD: a function can be cheap to load but hard to reason about,
or expensive to load but trivial once context is assembled.

  AICP = external_calls/20 ×60 + sloc/200 ×40 − doc_score/10 ×15

External-call breadth is the primary driver (p99 ceiling = 20 calls).

Lower it by: reducing the number of distinct out-of-file functions/macros the
function calls — consolidate dependencies and narrow its collaborators — then
trimming SLOC.",

        ExplainMetric::Mccabe => "\
McCabe — Cyclomatic Complexity

Counts linearly independent paths through a function: decision points + 1
(if / while / for / case / ternary / && / || / except). 100% match with
pmccabe across the validation corpus.

Thresholds: ≤10 good, 11–20 moderate, 21+ consider refactoring.
Lower it by collapsing conditionals, using early returns, and table-driving
repetitive switches.",

        ExplainMetric::Cognitive => "\
Cognitive Complexity (Campbell / SonarSource)

How hard code is to *understand*. Like McCabe, but nesting is penalized more,
else-if chains cost less than independent ifs, and a switch is a single
increment regardless of arm count.

This is the #1 driver of AIRD. Lower it by flattening nesting (guard clauses,
early returns) and extracting deeply nested blocks into named helpers.",

        ExplainMetric::Nesting => "\
Nesting Depth

Maximum depth of nested control structures (if / for / while / switch /
closures) within a function. Deeper than 4 levels strongly correlates with
hard-to-maintain code.

Lower it with guard clauses, early returns, and by extracting inner blocks
into helpers.",

        ExplainMetric::Sloc => "\
SLOC — Source Lines of Code

Non-blank, non-comment lines within the function body. Functions over ~50
SLOC often benefit from decomposition.

Lower it by extracting cohesive sub-steps into named helpers.",

        ExplainMetric::Abc => "\
ABC Complexity

Magnitude of the (Assignments, Branches/calls, Conditions) vector:
√(A² + B² + C²) — a broad measure of how much a function does.

Lower it by splitting multi-purpose functions and reducing assignment, call,
and branch density.",

        ExplainMetric::Returns => "\
Return Count

Number of return statements in a function. A high count can signal tangled
control flow, though guard-clause early returns are often fine.

Lower it by consolidating exit points where it improves clarity.",

        ExplainMetric::ExternalCalls => "\
External Calls

Count of distinct call targets not defined in the same file (out-of-file
functions and function-like macros) — a measure of dependency breadth and the
primary driver of AICP. p99 across the validation corpus = 20.

Lower it by consolidating dependencies and narrowing the function's
collaborators.",
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output (default)
    Text,
    /// SARIF 2.1.0 JSON output for editor/CI integration
    Sarif,
    /// JSON array of per-function metrics
    Json,
    /// Newline-delimited JSON (one record per line) — composable across files via find/xargs
    Ndjson,
    /// CSV with per-function metrics (header + rows)
    Csv,
}

/// Parse a source file into a tree-sitter Tree, selecting the grammar by extension.
fn parse_file(file: &Path, source_code: &str) -> Result<Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language_for_file(file))
        .context("Failed to set language")?;
    parser
        .parse(source_code, None)
        .with_context(|| format!("Failed to parse {}", file.display()))
}

struct Thresholds {
    mccabe: Option<u32>,
    cognitive: Option<u32>,
    nesting: Option<u32>,
    sloc: Option<u32>,
    abc: Option<f64>,
    returns: Option<u32>,
    aird: Option<u32>,
    aicp: Option<u32>,
    external_calls: Option<u32>,
}

impl Thresholds {
    fn active(&self) -> bool {
        self.mccabe.is_some()
            || self.cognitive.is_some()
            || self.nesting.is_some()
            || self.sloc.is_some()
            || self.abc.is_some()
            || self.returns.is_some()
            || self.aird.is_some()
            || self.aicp.is_some()
            || self.external_calls.is_some()
    }
}

/// Records a violation when `value` exceeds `limit`. In baseline mode `baseline`
/// carries this function's previously-snapshotted value for the metric; a
/// pre-existing offender that did not get *worse* (`value <= baseline`) is
/// tolerated. `baseline == None` means either "not in baseline mode" or "new
/// function not present in the baseline" — both correctly report the violation.
fn check_u32_threshold(
    out: &mut Vec<String>,
    label: &str,
    limit: Option<u32>,
    value: u32,
    baseline: Option<u32>,
) {
    if let Some(lim) = limit {
        if value > lim {
            if let Some(base) = baseline {
                if value <= base {
                    return;
                }
            }
            out.push(format!("{} {} > {}", label, value, lim));
        }
    }
}

fn check_f64_threshold(
    out: &mut Vec<String>,
    label: &str,
    limit: Option<f64>,
    value: f64,
    baseline: Option<f64>,
) {
    if let Some(lim) = limit {
        if value > lim {
            if let Some(base) = baseline {
                if value <= base {
                    return;
                }
            }
            out.push(format!("{} {:.2} > {:.2}", label, value, lim));
        }
    }
}

fn aird_term(label: &str, value: f64, max: f64, cap_label: &str) -> String {
    let capped = if value >= max { cap_label } else { "" };
    format!("{}: {:.1}/{:.0}{}", label, value, max, capped)
}

fn format_aird_breakdown(func: &FunctionMetrics) -> String {
    let cognitive_contrib = (func.cognitive as f64 / 75.0).min(1.0) * 55.0;
    let sloc_contrib      = (func.sloc as f64 / 200.0).min(1.0) * 15.0;
    let nesting_contrib   = (func.nesting as f64 / 8.0).min(1.0) * 15.0;
    let test_contrib      = (func.test_scoring.total_score.max(0) as f64 / 20.0).min(1.0) * 15.0;
    let doc_contrib       = (func.test_scoring.documentation_score.max(0) as f64 / 10.0).min(1.0) * 15.0;
    let coupling_contrib  = (func.state_coupling as f64 / 12.0).min(1.0) * 10.0;

    let cog  = aird_term("cognitive", cognitive_contrib, 55.0, " [capped]");
    let sloc = aird_term("sloc",      sloc_contrib,      15.0, " [capped]");
    let nest = aird_term("nesting",   nesting_contrib,   15.0, " [capped]");
    let test = aird_term("test",      test_contrib,      15.0, " [capped]");
    let doc  = format!("doc: -{:.1}/15", doc_contrib);
    let coup = format!("coupling: +{:.1}/10", coupling_contrib);

    format!("    {}, {}, {}, {}, {}, {}", cog, sloc, nest, test, doc, coup)
}

/// One function's snapshotted scores in a baseline file. Keyed on
/// `file` + `function` (line numbers are deliberately omitted so the baseline
/// stays stable as code moves). Records every gateable metric so any threshold
/// combination can be ratcheted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BaselineEntry {
    file: String,
    function: String,
    mccabe: u32,
    cognitive: u32,
    nesting: u32,
    sloc: u32,
    abc_magnitude: f64,
    return_count: u32,
    aird: u32,
    aicp: u32,
    external_calls: u32,
}

/// A baseline snapshot: the set of per-function scores at the time the gate was
/// adopted. On a later run, only functions that are new or worse than their
/// entry here fail the gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    version: u32,
    functions: Vec<BaselineEntry>,
}

impl Baseline {
    fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read baseline file: {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse baseline file: {}", path.display()))
    }

    /// Lookup table keyed by `(file, function)`. On duplicate keys (same-named
    /// functions in one file) the last entry wins — an accepted limitation
    /// shared with clippy/eslint-style baselines.
    fn index(&self) -> HashMap<(&str, &str), &BaselineEntry> {
        self.functions
            .iter()
            .map(|e| ((e.file.as_str(), e.function.as_str()), e))
            .collect()
    }
}

/// Snapshot the analyzed functions into a Baseline, sorted by (file, function)
/// for deterministic, diff-friendly output.
fn baseline_from_metrics(metrics: &[FunctionMetrics]) -> Baseline {
    let mut functions: Vec<BaselineEntry> = metrics
        .iter()
        .map(|f| BaselineEntry {
            file: f.file_path.clone(),
            function: f.name.clone(),
            mccabe: f.mccabe,
            cognitive: f.cognitive,
            nesting: f.nesting,
            sloc: f.sloc,
            abc_magnitude: f.abc_magnitude,
            return_count: f.return_count,
            aird: f.aird,
            aicp: f.aicp,
            external_calls: f.external_calls,
        })
        .collect();
    functions.sort_by(|a, b| (&a.file, &a.function).cmp(&(&b.file, &b.function)));
    Baseline {
        version: 1,
        functions,
    }
}

fn write_baseline(path: &Path, metrics: &[FunctionMetrics]) -> Result<()> {
    let baseline = baseline_from_metrics(metrics);
    let json = serde_json::to_string_pretty(&baseline)?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("Failed to write baseline file: {}", path.display()))?;
    Ok(())
}

/// Returns the top `top_n` AIRD components that drove the score, as
/// `(label, raw value)` pairs ranked by weighted contribution. Only positive
/// contributors are considered (doc *reduces* AIRD, so it is never a driver),
/// and zero-contribution components are dropped. Used to fold a concise
/// `(drivers: cognitive 215, sloc 492)` hint onto the violation line itself.
fn aird_drivers(func: &FunctionMetrics, top_n: usize) -> Vec<(&'static str, i64)> {
    let test_score = func.test_scoring.total_score.max(0);
    let mut comps: [(&'static str, i64, f64); 5] = [
        ("cognitive", func.cognitive as i64, (func.cognitive as f64 / 75.0).min(1.0) * 55.0),
        ("sloc",      func.sloc as i64,      (func.sloc as f64 / 200.0).min(1.0) * 15.0),
        ("nesting",   func.nesting as i64,   (func.nesting as f64 / 8.0).min(1.0) * 15.0),
        ("test",      test_score as i64,     (test_score as f64 / 20.0).min(1.0) * 15.0),
        ("coupling",  func.state_coupling as i64, (func.state_coupling as f64 / 12.0).min(1.0) * 10.0),
    ];
    comps.sort_by(|a, b| b.2.total_cmp(&a.2));
    comps
        .iter()
        .filter(|(_, _, contrib)| *contrib > 0.0)
        .take(top_n)
        .map(|(label, raw, _)| (*label, *raw))
        .collect()
}

fn aird_tips(func: &FunctionMetrics) -> Vec<String> {
    let mut tips = Vec::new();
    let cognitive_capped = func.cognitive >= 75;
    let sloc_capped = func.sloc >= 200;

    if cognitive_capped && sloc_capped {
        tips.push(
            "    Tip: cognitive and sloc are both capped — incremental extractions won't move the \
             needle until you break through at least one cap. Push for a larger extraction that \
             drops cognitive below 75 or sloc below 200."
                .to_string(),
        );
    } else if cognitive_capped {
        tips.push(
            "    Tip: cognitive is capped at 75 (contributing full 55 pts). \
             Extract until cognitive drops below 75 to see AIRD improve."
                .to_string(),
        );
    }

    if func.state_coupling > 0 {
        tips.push(
            "    Tip: if early extractions are not reducing AIRD, the function may still be \
             above the cognitive cap. Continue extracting — once cognitive drops below 75 \
             the cognitive term will fall sharply and AIRD will follow."
                .to_string(),
        );
    }

    tips
}

/// The set of line ranges that changed in the current working tree relative to
/// a git ref, used by `--changed` / `--since` to scope gating to touched code.
/// Keyed by canonicalized absolute path; ranges are in the *current* (post-image)
/// file so they line up with the line numbers knots reports.
struct ChangedLines {
    map: HashMap<PathBuf, Vec<(u32, u32)>>,
}

impl ChangedLines {
    /// True if `[start, end]` intersects any changed range in `file_path`.
    /// A file with no recorded changes never overlaps (its functions are skipped).
    fn overlaps(&self, file_path: &str, start: u32, end: u32) -> bool {
        let key = canonicalize_path(Path::new(file_path));
        match self.map.get(&key) {
            Some(ranges) => ranges.iter().any(|&(s, e)| start <= e && s <= end),
            None => false,
        }
    }
}

/// Canonicalize for stable map keys; fall back to the path as-given if it can't
/// be resolved (e.g. it no longer exists on disk).
fn canonicalize_path(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Run `git` with the given args, returning stdout. Errors carry git's stderr so
/// a bad ref or "not a git repository" surfaces clearly.
fn git_output(args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .output()
        .context("failed to run git (is it installed and on PATH?)")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse the new-file range from a unified-diff hunk header
/// (`@@ -a,b +c,d @@`), returning `(start, len)` for the `+c,d` side. `len`
/// defaults to 1 when omitted. Returns `None` if the header is malformed.
fn parse_hunk_new_range(line: &str) -> Option<(u32, u32)> {
    let plus = line.split_whitespace().find(|t| t.starts_with('+'))?;
    let mut parts = plus[1..].splitn(2, ',');
    let start: u32 = parts.next()?.parse().ok()?;
    let len: u32 = match parts.next() {
        Some(l) => l.parse().ok()?,
        None => 1,
    };
    Some((start, len))
}

/// Collect the changed line ranges of the working tree vs. `reference`. Modified
/// and added regions come from `git diff --unified=0`; brand-new untracked files
/// are treated as entirely changed so all of their functions are in scope.
fn collect_changed_lines(reference: &str) -> Result<ChangedLines> {
    let root = git_output(&["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(root.trim());

    let diff = git_output(&["diff", "--unified=0", "--no-color", reference])?;
    let mut map: HashMap<PathBuf, Vec<(u32, u32)>> = HashMap::new();
    let mut current: Option<PathBuf> = None;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ ") {
            // `+++ b/<path>` for the post-image; `+++ /dev/null` for a deletion
            // (nothing in the current tree to attribute, so drop scope).
            let rest = rest.split('\t').next().unwrap_or(rest);
            current = if rest == "/dev/null" {
                None
            } else {
                let rel = rest.strip_prefix("b/").unwrap_or(rest);
                Some(canonicalize_path(&root.join(rel)))
            };
        } else if line.starts_with("@@") {
            if let Some(path) = &current {
                if let Some((start, len)) = parse_hunk_new_range(line) {
                    if len > 0 {
                        map.entry(path.clone())
                            .or_default()
                            .push((start, start + len - 1));
                    }
                }
            }
        }
    }

    // Untracked files are newly added in full; cover the whole file so every
    // function counts as touched.
    let untracked = git_output(&["ls-files", "--others", "--exclude-standard"])?;
    for rel in untracked.lines() {
        if rel.is_empty() {
            continue;
        }
        map.entry(canonicalize_path(&root.join(rel)))
            .or_default()
            .push((1, u32::MAX));
    }

    Ok(ChangedLines { map })
}

/// A one-line pointer printed under AI-metric violations, expanding the acronyms
/// and pointing at `--explain`. Returns None when no AI metric was violated.
fn ai_metric_pointer(any_aird: bool, any_aicp: bool) -> Option<String> {
    match (any_aird, any_aicp) {
        (true, true) => Some(
            "  AIRD = AI Reasoning Difficulty, AICP = AI Context Pressure — run \
             `knots --explain aird` / `knots --explain aicp` for how to lower them."
                .to_string(),
        ),
        (true, false) => Some(
            "  AIRD = AI Reasoning Difficulty — run `knots --explain aird` for how to lower it."
                .to_string(),
        ),
        (false, true) => Some(
            "  AICP = AI Context Pressure — run `knots --explain aicp` for how to lower it."
                .to_string(),
        ),
        (false, false) => None,
    }
}

fn check_thresholds(
    metrics: &[FunctionMetrics],
    t: &Thresholds,
    baseline: Option<&Baseline>,
    changed: Option<&ChangedLines>,
) -> Result<()> {
    if !t.active() {
        return Ok(());
    }

    // In baseline mode, look each function up so per-metric scores can be
    // compared against its snapshot. None for every metric when baseline is
    // off, or when the function is new (absent from the baseline).
    let index = baseline.map(|b| b.index());

    let mut output_lines: Vec<String> = Vec::new();
    let mut violation_count: usize = 0;
    // Track whether any AI metric was violated so we can append a one-line
    // pointer expanding the acronym (users meet AIRD/AICP first at the CLI).
    let mut any_aird = false;
    let mut any_aicp = false;

    for func in metrics {
        // In --changed / --since mode, only gate functions that overlap a
        // changed line range — untouched pre-existing offenders are skipped.
        if let Some(changed) = changed {
            if !changed.overlaps(&func.file_path, func.start_line, func.end_line) {
                continue;
            }
        }

        let base: Option<&BaselineEntry> = index
            .as_ref()
            .and_then(|idx| idx.get(&(func.file_path.as_str(), func.name.as_str())).copied());

        let mut fv: Vec<String> = Vec::new();
        check_u32_threshold(&mut fv, "McCabe",        t.mccabe,         func.mccabe,         base.map(|b| b.mccabe));
        check_u32_threshold(&mut fv, "Cognitive",     t.cognitive,      func.cognitive,      base.map(|b| b.cognitive));
        check_u32_threshold(&mut fv, "Nesting",       t.nesting,        func.nesting,        base.map(|b| b.nesting));
        check_u32_threshold(&mut fv, "SLOC",          t.sloc,           func.sloc,           base.map(|b| b.sloc));
        check_f64_threshold(&mut fv, "ABC",           t.abc,            func.abc_magnitude,  base.map(|b| b.abc_magnitude));
        check_u32_threshold(&mut fv, "Returns",       t.returns,        func.return_count,   base.map(|b| b.return_count));
        check_u32_threshold(&mut fv, "AIRD",          t.aird,           func.aird,           base.map(|b| b.aird));
        check_u32_threshold(&mut fv, "AICP",          t.aicp,           func.aicp,           base.map(|b| b.aicp));
        check_u32_threshold(&mut fv, "ExternalCalls", t.external_calls, func.external_calls, base.map(|b| b.external_calls));

        if !fv.is_empty() {
            violation_count += 1;
            let loc = if func.file_path.is_empty() {
                format!("{}:{}", func.name, func.start_line)
            } else {
                format!("{}:{}:{}", func.file_path, func.start_line, func.name)
            };
            let aird_violated = fv.iter().any(|s| s.starts_with("AIRD"));
            any_aird |= aird_violated;
            any_aicp |= fv.iter().any(|s| s.starts_with("AICP"));
            let drivers_suffix = if aird_violated {
                let drivers = aird_drivers(func, 2);
                if drivers.is_empty() {
                    String::new()
                } else {
                    let parts: Vec<String> =
                        drivers.iter().map(|(l, v)| format!("{} {}", l, v)).collect();
                    format!("  (drivers: {})", parts.join(", "))
                }
            } else {
                String::new()
            };
            output_lines.push(format!("  {} — {}{}", loc, fv.join(", "), drivers_suffix));

            if aird_violated {
                output_lines.push(format_aird_breakdown(func));
                output_lines.extend(aird_tips(func));
            }
        }
    }

    if violation_count > 0 {
        if baseline.is_some() {
            eprintln!("New or worsened threshold violations vs. baseline ({violation_count}):");
        } else {
            eprintln!("Threshold violations ({violation_count}):");
        }
        for line in &output_lines {
            eprintln!("{}", line);
        }
        if let Some(pointer) = ai_metric_pointer(any_aird, any_aicp) {
            eprintln!("{}", pointer);
        }
        if baseline.is_some() {
            anyhow::bail!(
                "{} function(s) regressed beyond the baseline. Run with --write-baseline to accept the current state.",
                violation_count
            );
        }
        anyhow::bail!(
            "{} function(s) exceeded complexity thresholds",
            violation_count
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // --explain prints a metric's meaning and exits; no files required.
    if let Some(metric) = args.explain {
        println!("{}", explain_metric(metric));
        return Ok(());
    }

    // Load filter rules
    let include_rules = if let Some(path) = &args.include {
        Some(FilterRules::from_file(path)?)
    } else {
        None
    };

    let exclude_rules = if let Some(path) = &args.exclude {
        Some(FilterRules::from_file(path)?)
    } else {
        None
    };

    let exclude_path_patterns: Vec<regex::Regex> = args
        .exclude_path
        .iter()
        .map(|p| {
            regex::Regex::new(p)
                .unwrap_or_else(|e| panic!("Invalid --exclude-path pattern '{p}': {e}"))
        })
        .collect();

    // Collect files to process
    let files = if let Some(compile_commands_path) = &args.compile_commands {
        // Load files from compile_commands.json
        load_compile_commands(compile_commands_path, &include_rules, &exclude_rules)?
    } else if !args.files.is_empty() {
        // One or more file/directory paths (supports pre-commit passing multiple staged files)
        let mut collected = Vec::new();
        for path in &args.files {
            collected.extend(collect_files(
                path,
                args.recursive,
                &include_rules,
                &exclude_rules,
                &exclude_path_patterns,
            )?);
        }
        collected
    } else {
        anyhow::bail!("Either FILE or --compile-commands must be specified");
    };

    if files.is_empty() {
        return Ok(());
    }

    let thresholds = Thresholds {
        mccabe: args.mccabe_threshold,
        cognitive: args.cognitive_threshold,
        nesting: args.nesting_threshold,
        sloc: args.sloc_threshold,
        abc: args.abc_threshold,
        returns: args.return_threshold,
        aird: args.aird_threshold,
        aicp: args.aicp_threshold,
        external_calls: args.external_calls_threshold,
    };

    // Write-baseline mode: snapshot every analyzed function and exit without
    // gating. Handled before format/mode dispatch so `--write-baseline` behaves
    // identically regardless of --format. clap's `requires` guarantees a path.
    if args.write_baseline {
        let (all_metrics, _skipped) =
            collect_all_metrics(&files, &include_rules, &exclude_rules);
        let path = args
            .baseline
            .as_ref()
            .expect("--write-baseline requires --baseline (enforced by clap)");
        write_baseline(path, &all_metrics)?;
        eprintln!(
            "Wrote baseline: {} function(s) -> {}",
            all_metrics.len(),
            path.display()
        );
        return Ok(());
    }

    // Load the baseline for ratchet-mode gating (read-only here).
    let baseline = match &args.baseline {
        Some(path) => Some(Baseline::from_file(path)?),
        None => None,
    };

    // Resolve --changed / --since to a git ref, then collect the changed line
    // ranges used to scope gating. `--changed` is sugar for `--since HEAD`.
    let changed_ref: Option<&str> = if args.changed {
        Some("HEAD")
    } else {
        args.since.as_deref()
    };
    let changed = match changed_ref {
        Some(reference) => Some(collect_changed_lines(reference)?),
        None => None,
    };

    // Structured output modes: collect all metrics then emit and exit.
    // These bypass text/matrix output so only the structured data goes to stdout.
    match args.format {
        OutputFormat::Sarif => {
            let (all_metrics, _skipped_files) =
                collect_all_metrics(&files, &include_rules, &exclude_rules);
            emit_sarif(&all_metrics)?;
            return Ok(());
        }
        OutputFormat::Json => {
            let (all_metrics, _skipped_files) =
                collect_all_metrics(&files, &include_rules, &exclude_rules);
            emit_json(&all_metrics)?;
            return Ok(());
        }
        OutputFormat::Ndjson => {
            let (all_metrics, _skipped_files) =
                collect_all_metrics(&files, &include_rules, &exclude_rules);
            emit_ndjson(&all_metrics)?;
            return Ok(());
        }
        OutputFormat::Csv => {
            let (all_metrics, _skipped_files) =
                collect_all_metrics(&files, &include_rules, &exclude_rules);
            emit_csv(&all_metrics)?;
            return Ok(());
        }
        OutputFormat::Text => {}
    }

    // For matrix mode
    if args.matrix {
        let mut all_metrics = Vec::new();
        let mut skipped_files = 0;

        for file in &files {
            let source_code = match fs::read_to_string(file) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Warning: Skipping {}: {}", file.display(), e);
                    skipped_files += 1;
                    continue;
                }
            };

            let tree = match parse_file(file, &source_code) {
                Ok(t) => t,
                Err(_) => {
                    let hint = if file.extension().and_then(|e| e.to_str()) == Some("h") {
                        " (if this file contains C++, rename to .hpp)"
                    } else {
                        ""
                    };
                    eprintln!("Warning: Failed to parse {}{}", file.display(), hint);
                    skipped_files += 1;
                    continue;
                }
            };

            let metrics = collect_function_metrics(
                &tree,
                &source_code,
                file.to_str().unwrap_or(""),
                &include_rules,
                &exclude_rules,
            );
            all_metrics.extend(metrics);
        }

        if all_metrics.is_empty() {
            anyhow::bail!(
                "No functions found in any files (skipped {} files)",
                skipped_files
            );
        }

        display_testability_matrix(&all_metrics, files.len(), skipped_files);
        check_thresholds(&all_metrics, &thresholds, baseline.as_ref(), changed.as_ref())?;
        return Ok(());
    }

    // For single file mode, use traditional output
    if files.len() == 1 {
        let file = &files[0];
        let source_code = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let tree = parse_file(file, &source_code)?;

        analyze_code(
            &tree,
            &source_code,
            args.verbose,
            &include_rules,
            &exclude_rules,
        )?;
        let metrics = collect_function_metrics(
            &tree,
            &source_code,
            file.to_str().unwrap_or(""),
            &include_rules,
            &exclude_rules,
        );
        check_thresholds(&metrics, &thresholds, baseline.as_ref(), changed.as_ref())?;
        return Ok(());
    }

    // For recursive mode with multiple files: collect all metrics, write report, show summary
    let mut all_metrics = Vec::new();
    let mut skipped_files = 0;

    for file in &files {
        let source_code = match fs::read_to_string(file) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Warning: Skipping {}: {}", file.display(), e);
                skipped_files += 1;
                continue;
            }
        };

        let tree = match parse_file(file, &source_code) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("Warning: Failed to parse {}", file.display());
                skipped_files += 1;
                continue;
            }
        };

        let metrics = collect_function_metrics(
            &tree,
            &source_code,
            file.to_str().unwrap_or(""),
            &include_rules,
            &exclude_rules,
        );
        all_metrics.extend(metrics);
    }

    if all_metrics.is_empty() {
        anyhow::bail!(
            "No functions found in any files (skipped {} files)",
            skipped_files
        );
    }

    if let Some(report_path) = &args.report {
        write_detailed_report(&all_metrics, args.verbose, report_path)?;
    }

    // Display summary with top 5 worst functions and totals/averages
    display_recursive_summary(&all_metrics, files.len(), skipped_files, args.report.as_deref());

    check_thresholds(&all_metrics, &thresholds, baseline.as_ref(), changed.as_ref())?;
    Ok(())
}

/// Load file paths from compile_commands.json
fn load_compile_commands(
    compile_commands_path: &Path,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(compile_commands_path).with_context(|| {
        format!(
            "Failed to read compile_commands.json: {}",
            compile_commands_path.display()
        )
    })?;

    let commands: Vec<CompileCommand> = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse compile_commands.json: {}",
            compile_commands_path.display()
        )
    })?;

    let mut files = Vec::new();

    for cmd in commands {
        let file_path = PathBuf::from(&cmd.file);

        // Only process supported source files
        if let Some(ext) = file_path.extension() {
            if is_source_extension(ext) {
                let file_str = file_path.to_string_lossy();
                if should_process_file(&file_str, include_rules, exclude_rules) {
                    // Use absolute path if available, otherwise relative
                    if file_path.is_absolute() {
                        files.push(file_path);
                    } else {
                        // Try to make it absolute using the directory from compile command
                        let abs_path = if !cmd.directory.is_empty() {
                            PathBuf::from(&cmd.directory).join(&file_path)
                        } else {
                            file_path.clone()
                        };

                        if abs_path.exists() {
                            files.push(abs_path);
                        } else if file_path.exists() {
                            files.push(file_path);
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        anyhow::bail!("No supported source files found in compile_commands.json");
    }

    Ok(files)
}

/// Collect files to process based on the path and recursive flag
fn collect_files(
    path: &PathBuf,
    recursive: bool,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
    exclude_path_patterns: &[regex::Regex],
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        // Single file mode — skip unsupported types rather than passing them to the parser
        let supported = path
            .extension()
            .map(|e| is_source_extension(e))
            .unwrap_or(false);
        if !supported {
            return Ok(files);
        }
        let file_str = path.to_string_lossy();
        if !path_is_excluded(&file_str, exclude_path_patterns)
            && should_process_file(&file_str, include_rules, exclude_rules)
        {
            files.push(path.clone());
        }
    } else if path.is_dir() {
        if !recursive {
            anyhow::bail!(
                "Path '{}' is a directory. Use -r/--recursive to process directories recursively.",
                path.display()
            );
        }

        // Recursive directory mode - scan source files (not headers)
        // Headers often contain inline/vendor code
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if is_source_extension(ext) {
                        let file_str = file_path.to_string_lossy();
                        if !path_is_excluded(&file_str, exclude_path_patterns)
                            && should_process_file(&file_str, include_rules, exclude_rules)
                        {
                            files.push(file_path.to_path_buf());
                        }
                    }
                }
            }
        }

        if files.is_empty() {
            anyhow::bail!(
                "No supported source files found in directory: {}",
                path.display()
            );
        }
    } else {
        anyhow::bail!("Path '{}' does not exist", path.display());
    }

    Ok(files)
}

fn path_is_excluded(path: &str, patterns: &[regex::Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(path))
}

/// Check if a file should be processed based on include/exclude rules
fn should_process_file(
    file_path: &str,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> bool {
    // Check include rules first (whitelist)
    if let Some(rules) = include_rules {
        if !rules.matches_file(file_path) {
            return false;
        }
    }

    // Check exclude rules (blacklist) - if it matches exclude, DON'T process
    if let Some(rules) = exclude_rules {
        if rules.matches_file(file_path) {
            return false;
        }
    }

    true
}

/// Collect function metrics from a file
/// Collects all function and macro names defined in this translation unit.
/// Used to classify call sites as local vs. external.
fn collect_local_names(root: Node, source_code: &str) -> HashSet<String> {
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
        | "method_definition"
        | "generator_function_declaration"
        | "generator_function" => {
            if let Some(name) = get_function_name(node, source_code) {
                names.insert(name);
            }
        }
        // C/C++ preprocessor macros
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

/// Counts unique external call targets in a function body — identifiers called
/// via call_expression that are not defined in the same translation unit.
/// Captures both out-of-file function calls and function-like macro invocations.
fn calculate_external_calls(
    func_node: Node,
    source_code: &str,
    local_names: &HashSet<String>,
) -> u32 {
    let mut external: HashSet<String> = HashSet::new();
    collect_external_calls_recursive(func_node, source_code, local_names, &mut external);
    external.len() as u32
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
            // Simple name call: foo() — check against local definitions
            if let Ok(name) = func_node.utf8_text(source_code.as_bytes()) {
                if !local_names.contains(name) {
                    external.insert(name.to_string());
                }
            }
        }
        // Rust path (Foo::bar()), Python attribute (obj.method()), JS member (obj.method()) —
        // all qualify as external by definition
        "scoped_identifier" | "attribute" | "member_expression" => {
            if let Ok(name) = func_node.utf8_text(source_code.as_bytes()) {
                external.insert(name.to_string());
            }
        }
        "field_expression" => {
            // Rust method call: self.foo(), vec.push() — check the method name, not the receiver
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
    if node.kind() == "call_expression" || node.kind() == "call" {
        handle_call_node(node, source_code, local_names, external);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_external_calls_recursive(child, source_code, local_names, external);
    }
}

fn collect_function_metrics(
    tree: &Tree,
    source_code: &str,
    file_path: &str,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Vec<FunctionMetrics> {
    let root_node = tree.root_node();
    let local_names = collect_local_names(root_node, source_code);
    let mut cursor = root_node.walk();
    let mut metrics = Vec::new();

    let is_python = file_path.ends_with(".py");
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            let mccabe = calculate_mccabe_complexity(node, src.as_bytes());
            let cognitive = calculate_cognitive_complexity(node, src.as_bytes());
            let nesting = calculate_nesting_depth(node);
            let sloc = if is_python {
                calculate_sloc_python(node, src.as_bytes())
            } else {
                calculate_sloc(node, src.as_bytes())
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

/// Check if a function should be processed based on include/exclude rules
fn should_process_function(
    function_name: &str,
    complexity: u32,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> bool {
    // Check include rules first (whitelist)
    if let Some(rules) = include_rules {
        if !rules.matches_function(function_name) {
            return false;
        }
        if !rules.matches_complexity(complexity) {
            return false;
        }
    }

    // Check exclude rules (blacklist) - if it matches exclude, DON'T process
    // Only apply function/complexity filters if they're actually specified
    if let Some(rules) = exclude_rules {
        let matches_func =
            !rules.function_patterns.is_empty() && rules.matches_function(function_name);
        let matches_complexity = (rules.min_complexity.is_some() || rules.max_complexity.is_some())
            && rules.matches_complexity(complexity);

        // If no function patterns specified, only check complexity
        // If no complexity bounds specified, only check function patterns
        // If both specified, require both to match
        let should_exclude = if rules.function_patterns.is_empty()
            && rules.min_complexity.is_none()
            && rules.max_complexity.is_none()
        {
            // No function-level filters, don't exclude based on function criteria
            false
        } else if rules.function_patterns.is_empty() {
            // Only complexity filter
            matches_complexity
        } else if rules.min_complexity.is_none() && rules.max_complexity.is_none() {
            // Only function pattern filter
            matches_func
        } else {
            // Both specified, require both
            matches_func && matches_complexity
        };

        if should_exclude {
            return false;
        }
    }

    true
}

fn analyze_code(
    tree: &Tree,
    source_code: &str,
    verbose: bool,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Result<()> {
    let metrics = collect_function_metrics(tree, source_code, "", include_rules, exclude_rules);

    let mut total_mccabe = 0;
    let mut total_cognitive = 0;
    let mut total_nesting = 0;
    let mut total_sloc = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count = 0;
    let mut total_test_score: i64 = 0;
    let mut total_aird: u64 = 0;
    let mut total_aicp: u64 = 0;

    for func in &metrics {
        total_mccabe += func.mccabe;
        total_cognitive += func.cognitive;
        total_nesting += func.nesting;
        total_sloc += func.sloc;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count;
        total_test_score += func.test_scoring.total_score as i64;
        total_aird += func.aird as u64;
        total_aicp += func.aicp as u64;

        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            println!("Function: {} {}", func.name, emoji);
            println!("  McCabe Complexity: {}", func.mccabe);
            println!("  Cognitive Complexity: {}", func.cognitive);
            println!("  Nesting Depth: {}", func.nesting);
            println!("  SLOC: {}", func.sloc);
            println!("  ABC Magnitude: {:.2}", func.abc_magnitude);
            println!("  Return Count: {}", func.return_count);
            println!(
                "  Test Scoring: {} ({})",
                func.test_scoring.total_score,
                func.test_scoring.classification()
            );
            println!("    - Signature: {}", func.test_scoring.signature_score);
            println!("    - Dependency: {}", func.test_scoring.dependency_score);
            println!("    - Observable: {}", func.test_scoring.observable_score);
            println!(
                "    - Implementation: {}",
                func.test_scoring.implementation_score
            );
            println!(
                "    - Documentation: {}",
                func.test_scoring.documentation_score
            );
            println!("  AIRD Score: {}", func.aird);
            println!("  AICP Score: {}", func.aicp);
            println!("  External Calls: {}", func.external_calls);
            println!("  State Coupling: {}", func.state_coupling);
            println!("  Max Complexity: {}", func.max_complexity());
            println!();
        } else {
            println!(
                "{} {} (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {})",
                emoji, func.name, func.mccabe, func.cognitive, func.nesting, func.sloc,
                func.abc_magnitude, func.return_count, func.test_scoring.total_score, func.aird,
                func.aicp, func.external_calls
            );
        }
    }

    let function_count = metrics.len();

    // Print summary
    println!();
    println!("Summary:");
    println!("  Total Functions: {}", function_count);
    println!("  Total McCabe Complexity: {}", total_mccabe);
    println!("  Total Cognitive Complexity: {}", total_cognitive);
    println!("  Total Nesting Depth: {}", total_nesting);
    println!("  Total SLOC: {}", total_sloc);
    println!("  Total ABC Magnitude: {:.2}", total_abc_magnitude);
    println!("  Total Return Count: {}", total_return_count);
    println!("  Total Test Score: {}", total_test_score);
    println!("  Total AIRD Score: {}", total_aird);
    println!("  Total AICP Score: {}", total_aicp);

    if function_count > 0 {
        println!(
            "  Average McCabe Complexity: {:.2}",
            total_mccabe as f64 / function_count as f64
        );
        println!(
            "  Average Cognitive Complexity: {:.2}",
            total_cognitive as f64 / function_count as f64
        );
        println!(
            "  Average Nesting Depth: {:.2}",
            total_nesting as f64 / function_count as f64
        );
        println!(
            "  Average SLOC: {:.2}",
            total_sloc as f64 / function_count as f64
        );
        println!(
            "  Average ABC Magnitude: {:.2}",
            total_abc_magnitude / function_count as f64
        );
        println!(
            "  Average Return Count: {:.2}",
            total_return_count as f64 / function_count as f64
        );
        println!(
            "  Average Test Score: {:.2}",
            total_test_score as f64 / function_count as f64
        );
        println!(
            "  Average AIRD Score: {:.2}",
            total_aird as f64 / function_count as f64
        );
        println!(
            "  Average AICP Score: {:.2}",
            total_aicp as f64 / function_count as f64
        );
    }

    Ok(())
}

/// Collect FunctionMetrics across multiple files, skipping unreadable/unparseable ones.
/// Returns (metrics, skipped_file_count). Used by SARIF mode.
fn collect_all_metrics(
    files: &[PathBuf],
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> (Vec<FunctionMetrics>, usize) {
    let mut all_metrics = Vec::new();
    let mut skipped = 0;

    for file in files {
        let source_code = match fs::read_to_string(file) {
            Ok(code) => code,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let tree = match parse_file(file, &source_code) {
            Ok(t) => t,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let metrics = collect_function_metrics(
            &tree,
            &source_code,
            file.to_str().unwrap_or(""),
            include_rules,
            exclude_rules,
        );
        all_metrics.extend(metrics);
    }

    (all_metrics, skipped)
}

/// SARIF level for a complexity bucket. Mirrors the emoji buckets:
/// 1-10 healthy (no result emitted), 11-20 note, 21-49 warning, 50+ error.
fn sarif_level(complexity: u32) -> &'static str {
    match complexity {
        0..=10 => "none",
        11..=20 => "note",
        21..=49 => "warning",
        _ => "error",
    }
}

/// Emit a SARIF 2.1.0 log to stdout describing functions whose max(McCabe, Cognitive)
/// exceeds the healthy threshold (>10). One result per offending function.
fn emit_sarif(all_metrics: &[FunctionMetrics]) -> Result<()> {
    use serde_json::json;

    let rules = json!([
        {
            "id": "knots/high-complexity",
            "name": "HighComplexity",
            "shortDescription": { "text": "Function exceeds complexity threshold" },
            "fullDescription": {
                "text": "Reports functions whose McCabe or cognitive complexity exceeds the healthy threshold (10). Severity escalates at 21 (warning) and 50 (error)."
            },
            "defaultConfiguration": { "level": "note" },
            "helpUri": "https://github.com/brandon-arrendondo/knots"
        }
    ]);

    let mut results = Vec::new();
    for func in all_metrics {
        let max = std::cmp::max(func.mccabe, func.cognitive);
        if max <= 10 {
            continue;
        }

        let uri = path_to_sarif_uri(&func.file_path);
        let message = format!(
            "{} has high complexity (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {})",
            func.name,
            func.mccabe,
            func.cognitive,
            func.nesting,
            func.sloc,
            func.abc_magnitude,
            func.return_count,
            func.test_scoring.total_score
        );

        results.push(json!({
            "ruleId": "knots/high-complexity",
            "level": sarif_level(max),
            "message": { "text": message },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": uri },
                    "region": {
                        "startLine": func.start_line,
                        "endLine": func.end_line
                    }
                },
                "logicalLocations": [{
                    "name": func.name,
                    "kind": "function"
                }]
            }],
            "properties": {
                "mccabe": func.mccabe,
                "cognitive": func.cognitive,
                "nesting": func.nesting,
                "sloc": func.sloc,
                "abcMagnitude": func.abc_magnitude,
                "returnCount": func.return_count,
                "testScore": func.test_scoring.total_score
            }
        }));
    }

    let log = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "knots",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/brandon-arrendondo/knots",
                    "rules": rules
                }
            },
            "results": results
        }]
    });

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &log).context("Failed to write SARIF JSON")?;
    writeln!(handle)?;
    Ok(())
}

fn emit_json(all_metrics: &[FunctionMetrics]) -> Result<()> {
    use serde_json::json;

    let records: Vec<_> = all_metrics
        .iter()
        .map(|f| {
            json!({
                "file": f.file_path,
                "function": f.name,
                "start_line": f.start_line,
                "end_line": f.end_line,
                "mccabe": f.mccabe,
                "cognitive": f.cognitive,
                "nesting": f.nesting,
                "sloc": f.sloc,
                "abc_magnitude": f.abc_magnitude,
                "return_count": f.return_count,
                "test_score": f.test_scoring.total_score,
                "doc_score": f.test_scoring.documentation_score,
                "aird": f.aird,
                "aicp": f.aicp,
                "external_calls": f.external_calls
            })
        })
        .collect();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &records).context("Failed to write JSON")?;
    writeln!(handle)?;
    Ok(())
}

fn emit_ndjson(all_metrics: &[FunctionMetrics]) -> Result<()> {
    use serde_json::json;
    use std::io::Write;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for f in all_metrics {
        let record = json!({
            "file": f.file_path,
            "function": f.name,
            "start_line": f.start_line,
            "end_line": f.end_line,
            "mccabe": f.mccabe,
            "cognitive": f.cognitive,
            "nesting": f.nesting,
            "sloc": f.sloc,
            "abc_magnitude": f.abc_magnitude,
            "return_count": f.return_count,
            "test_score": f.test_scoring.total_score,
            "doc_score": f.test_scoring.documentation_score,
            "aird": f.aird,
            "aicp": f.aicp,
            "external_calls": f.external_calls
        });
        serde_json::to_writer(&mut handle, &record).context("Failed to write NDJSON")?;
        writeln!(handle)?;
    }
    Ok(())
}

fn emit_csv(all_metrics: &[FunctionMetrics]) -> Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    writeln!(
        handle,
        "file,function,start_line,end_line,mccabe,cognitive,nesting,sloc,abc_magnitude,return_count,test_score,doc_score,aird,aicp,external_calls"
    )?;

    for f in all_metrics {
        // Escape function names that might contain commas (e.g. C++ templates)
        let name = if f.name.contains(',') {
            format!("\"{}\"", f.name.replace('"', "\"\""))
        } else {
            f.name.clone()
        };
        writeln!(
            handle,
            "{},{},{},{},{},{},{},{},{:.4},{},{},{},{},{},{}",
            f.file_path,
            name,
            f.start_line,
            f.end_line,
            f.mccabe,
            f.cognitive,
            f.nesting,
            f.sloc,
            f.abc_magnitude,
            f.return_count,
            f.test_scoring.total_score,
            f.test_scoring.documentation_score,
            f.aird,
            f.aicp,
            f.external_calls
        )?;
    }
    Ok(())
}

/// Convert a filesystem path string to a relative URI suitable for SARIF
/// artifactLocation. Uses a path relative to the current working directory
/// when possible, otherwise falls back to the raw path.
fn path_to_sarif_uri(file_path: &str) -> String {
    let path = Path::new(file_path);
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    file_path.replace('\\', "/")
}

fn write_detailed_report(all_metrics: &[FunctionMetrics], verbose: bool, path: &Path) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create report file: {}", path.display()))?;

    for func in all_metrics {
        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            writeln!(
                file,
                "Function: {} {} [{}]",
                func.name, emoji, func.file_path
            )?;
            writeln!(file, "  McCabe Complexity: {}", func.mccabe)?;
            writeln!(file, "  Cognitive Complexity: {}", func.cognitive)?;
            writeln!(file, "  Nesting Depth: {}", func.nesting)?;
            writeln!(file, "  SLOC: {}", func.sloc)?;
            writeln!(file, "  ABC Magnitude: {:.2}", func.abc_magnitude)?;
            writeln!(file, "  Return Count: {}", func.return_count)?;
            writeln!(
                file,
                "  Test Scoring: {} ({})",
                func.test_scoring.total_score,
                func.test_scoring.classification()
            )?;
            writeln!(
                file,
                "    - Signature: {}",
                func.test_scoring.signature_score
            )?;
            writeln!(
                file,
                "    - Dependency: {}",
                func.test_scoring.dependency_score
            )?;
            writeln!(
                file,
                "    - Observable: {}",
                func.test_scoring.observable_score
            )?;
            writeln!(
                file,
                "    - Implementation: {}",
                func.test_scoring.implementation_score
            )?;
            writeln!(
                file,
                "    - Documentation: {}",
                func.test_scoring.documentation_score
            )?;
            writeln!(file, "  AIRD Score: {}", func.aird)?;
            writeln!(file, "  AICP Score: {}", func.aicp)?;
            writeln!(file, "  External Calls: {}", func.external_calls)?;
            writeln!(file, "  Max Complexity: {}", func.max_complexity())?;
            writeln!(file)?;
        } else {
            writeln!(
                file,
                "{} {} [{}] (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {})",
                emoji, func.name, func.file_path, func.mccabe, func.cognitive, func.nesting,
                func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score,
                func.aird, func.aicp, func.external_calls
            )?;
        }
    }

    Ok(())
}

fn display_recursive_summary(
    all_metrics: &[FunctionMetrics],
    total_files: usize,
    skipped_files: usize,
    report_path: Option<&Path>,
) {
    // Sort by worst complexity (max of McCabe and Cognitive)
    let mut sorted = all_metrics.to_vec();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.max_complexity()));

    println!("\n=== TOP 5 WORST FUNCTIONS ===\n");
    for (i, func) in sorted.iter().take(5).enumerate() {
        let emoji = get_complexity_emoji(func.max_complexity());
        println!("{}. {} {} [{}]", i + 1, emoji, func.name, func.file_path);
        println!("   McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {}",
            func.mccabe, func.cognitive, func.nesting, func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score, func.aird, func.aicp, func.external_calls
        );
    }

    // Calculate totals and averages
    let mut total_mccabe: u64 = 0;
    let mut total_cognitive: u64 = 0;
    let mut total_nesting: u64 = 0;
    let mut total_sloc: u64 = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count: u64 = 0;
    let mut total_test_score: i64 = 0;
    let mut total_aird: u64 = 0;
    let mut total_aicp: u64 = 0;

    for func in all_metrics {
        total_mccabe += func.mccabe as u64;
        total_cognitive += func.cognitive as u64;
        total_nesting += func.nesting as u64;
        total_sloc += func.sloc as u64;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count as u64;
        total_test_score += func.test_scoring.total_score as i64;
        total_aird += func.aird as u64;
        total_aicp += func.aicp as u64;
    }

    let function_count = all_metrics.len();

    println!("\n=== TOTALS & AVERAGES ===\n");
    println!("  Total Functions: {}", function_count);
    println!("  Total McCabe Complexity: {}", total_mccabe);
    println!("  Total Cognitive Complexity: {}", total_cognitive);
    println!("  Total Nesting Depth: {}", total_nesting);
    println!("  Total SLOC: {}", total_sloc);
    println!("  Total ABC Magnitude: {:.2}", total_abc_magnitude);
    println!("  Total Return Count: {}", total_return_count);
    println!("  Total Test Score: {}", total_test_score);
    println!("  Total AIRD Score: {}", total_aird);
    println!("  Total AICP Score: {}", total_aicp);

    if function_count > 0 {
        println!();
        println!(
            "  Average McCabe Complexity: {:.2}",
            total_mccabe as f64 / function_count as f64
        );
        println!(
            "  Average Cognitive Complexity: {:.2}",
            total_cognitive as f64 / function_count as f64
        );
        println!(
            "  Average Nesting Depth: {:.2}",
            total_nesting as f64 / function_count as f64
        );
        println!(
            "  Average SLOC: {:.2}",
            total_sloc as f64 / function_count as f64
        );
        println!(
            "  Average ABC Magnitude: {:.2}",
            total_abc_magnitude / function_count as f64
        );
        println!(
            "  Average Return Count: {:.2}",
            total_return_count as f64 / function_count as f64
        );
        println!(
            "  Average Test Score: {:.2}",
            total_test_score as f64 / function_count as f64
        );
        println!(
            "  Average AIRD Score: {:.2}",
            total_aird as f64 / function_count as f64
        );
        println!(
            "  Average AICP Score: {:.2}",
            total_aicp as f64 / function_count as f64
        );
    }

    if let Some(path) = report_path {
        println!("\nDetailed per-function output written to {}", path.display());
    }
    println!("\n=== FILES PROCESSED ===\n");
    println!("  Total files found: {}", total_files);
    println!("  Successfully processed: {}", total_files - skipped_files);
    if skipped_files > 0 {
        println!("  Skipped (encoding/parse errors): {}", skipped_files);
    }
}

#[derive(Debug, Clone)]
struct FunctionMetrics {
    name: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
    mccabe: u32,
    cognitive: u32,
    nesting: u32,
    sloc: u32,
    abc_magnitude: f64,
    return_count: u32,
    test_scoring: TestScoringMetric,
    aird: u32,
    aicp: u32,
    external_calls: u32,
    state_coupling: u32,
}

impl FunctionMetrics {
    fn max_complexity(&self) -> u32 {
        std::cmp::max(self.mccabe, self.cognitive)
    }
}

/// Display testability matrix for all functions
fn display_testability_matrix(
    all_metrics: &[FunctionMetrics],
    total_files: usize,
    skipped_files: usize,
) {
    // Categorize functions into quadrants
    let mut quick_wins = Vec::new();
    let mut invest_tests = Vec::new();
    let mut add_docs = Vec::new();
    let mut refactor = Vec::new();

    for func in all_metrics {
        let low_complexity = func.mccabe <= 10;
        let easy_to_test = func.test_scoring.total_score <= 10;

        match (low_complexity, easy_to_test) {
            (true, true) => quick_wins.push(func),
            (false, true) => invest_tests.push(func),
            (true, false) => add_docs.push(func),
            (false, false) => refactor.push(func),
        }
    }

    // Print matrix results
    println!("\n=== TESTABILITY MATRIX ===\n");

    println!("📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!");
    println!("=========================================================");
    if quick_wins.is_empty() {
        println!("  (none)");
    } else {
        for func in &quick_wins {
            if func.file_path.is_empty() {
                println!(
                    "  ✓ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ✓ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
            }
        }
    }
    println!();

    println!("🎯 INVEST IN TESTS (High Complexity, Easy to Test)");
    println!("==================================================");
    if invest_tests.is_empty() {
        println!("  (none)");
    } else {
        for func in &invest_tests {
            if func.file_path.is_empty() {
                println!(
                    "  → {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  → {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
            }
        }
    }
    println!();

    println!("📝 ADD DOCS (Low Complexity, Hard to Test)");
    println!("===========================================");
    if add_docs.is_empty() {
        println!("  (none)");
    } else {
        for func in &add_docs {
            if func.file_path.is_empty() {
                println!(
                    "  ⚠ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ⚠ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
            }
        }
    }
    println!();

    println!("🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!");
    println!("========================================================");
    if refactor.is_empty() {
        println!("  (none)");
    } else {
        for func in &refactor {
            if func.file_path.is_empty() {
                println!(
                    "  ⛔ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ⛔ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
            }
        }
    }
    println!();

    // Print summary
    println!("=== SUMMARY ===\n");
    println!("  Quick Wins:    {} functions", quick_wins.len());
    println!("  Invest Tests:  {} functions", invest_tests.len());
    println!("  Add Docs:      {} functions", add_docs.len());
    println!("  Refactor:      {} functions", refactor.len());
    println!("  Total:         {} functions", all_metrics.len());

    if total_files > 1 {
        println!();
        println!("=== FILES PROCESSED ===\n");
        println!("  Total files found: {}", total_files);
        println!("  Successfully processed: {}", total_files - skipped_files);
        if skipped_files > 0 {
            println!("  Skipped (encoding/parse errors): {}", skipped_files);
        }
    }
}

fn visit_functions<F>(cursor: &mut TreeCursor, source_code: &str, callback: &mut F)
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
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
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

fn get_function_name(node: Node, source_code: &str) -> Option<String> {
    // Rust function_item has a direct 'name' field
    if node.kind() == "function_item" {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
            .map(|s| s.to_string());
    }

    // JavaScript: function_declaration, function_expression, method_definition,
    // generator_function_declaration, and generator_function all have a direct 'name' field.
    // function_expression and generator_function may be anonymous (name field absent).
    if matches!(
        node.kind(),
        "function_declaration"
            | "function_expression"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
    ) {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source_code.as_bytes()).ok())
            .map(|s| s.to_string());
    }

    // Python function_definition also has a direct 'name' field.
    // C/C++ function_definition uses 'declarator' instead, so child_by_field_name("name")
    // returns None for them and we fall through to the declarator chain below.
    if node.kind() == "function_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            return name_node
                .utf8_text(source_code.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }

    // C/C++ function_definition uses a declarator chain
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(child, source_code);
        } else if child.kind() == "pointer_declarator" {
            // For functions returning pointers, the function_declarator is nested inside
            if let Some(name) = get_function_name_from_declarator(child, source_code) {
                return Some(name);
            }
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    /// args_override_self: the pre-commit hook entry bakes default
    /// thresholds (e.g. --abc-threshold=10.0) and a consumer's `args:`
    /// append their own, producing a duplicate. Verify the last value
    /// wins instead of clap erroring "cannot be used multiple times".
    #[test]
    fn test_threshold_arg_override_last_wins() {
        let args = Args::try_parse_from([
            "knots",
            "--abc-threshold=10.0",
            "--abc-threshold=20.0",
            "file.c",
        ])
        .expect("duplicate --abc-threshold should override, not error");
        assert_eq!(args.abc_threshold, Some(20.0));
    }

    #[test]
    fn test_threshold_arg_single_occurrence_unchanged() {
        let args = Args::try_parse_from(["knots", "--abc-threshold=10.0", "file.c"])
            .expect("single --abc-threshold should parse normally");
        assert_eq!(args.abc_threshold, Some(10.0));
    }

    /// Parse C++ code and collect discovered function names via visit_functions + get_function_name.
    fn discover_cpp_functions(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        let mut names = Vec::new();
        visit_functions(&mut cursor, code, &mut |node, src| {
            if let Some(name) = get_function_name(node, src) {
                names.push(name);
            }
        });
        names
    }

    #[test]
    fn test_cpp_discover_namespace_function() {
        let names = discover_cpp_functions(r#"namespace myns { void func() { int x = 0; } }"#);
        assert_eq!(names, vec!["func"]);
    }

    #[test]
    fn test_cpp_discover_class_method() {
        let names = discover_cpp_functions(r#"class Foo { void method() { int x = 0; } };"#);
        assert_eq!(names, vec!["method"]);
    }

    #[test]
    fn test_cpp_discover_template_function() {
        let names =
            discover_cpp_functions(r#"template<typename T> T add(T a, T b) { return a + b; }"#);
        assert_eq!(names, vec!["add"]);
    }

    #[test]
    fn test_cpp_discover_qualified_name() {
        let names = discover_cpp_functions(r#"void Foo::bar() { int x = 0; }"#);
        assert_eq!(names, vec!["Foo::bar"]);
    }

    #[test]
    fn test_cpp_discover_operator() {
        let names = discover_cpp_functions(r#"Foo operator+(Foo a, Foo b) { return a; }"#);
        assert_eq!(names.len(), 1);
        assert!(
            names[0].contains("operator+"),
            "Expected operator+, got: {}",
            names[0]
        );
    }

    #[test]
    fn test_cpp_discover_destructor() {
        let names = discover_cpp_functions(r#"class Foo { ~Foo() { int x = 0; } };"#);
        assert_eq!(names, vec!["~Foo"]);
    }

    // ---- Rust function discovery tests ----

    fn discover_rust_functions(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        let mut names = Vec::new();
        visit_functions(&mut cursor, code, &mut |node, src| {
            if let Some(name) = get_function_name(node, src) {
                names.push(name);
            }
        });
        names
    }

    #[test]
    fn test_rust_discover_simple_function() {
        let names = discover_rust_functions("fn simple() { let x = 1; }");
        assert_eq!(names, vec!["simple"]);
    }

    #[test]
    fn test_rust_discover_multiple_functions() {
        let names = discover_rust_functions("fn foo() {} fn bar() {} fn baz() {}");
        assert_eq!(names, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_rust_discover_function_with_params() {
        let names = discover_rust_functions("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(names, vec!["add"]);
    }

    #[test]
    fn test_rust_discover_impl_method() {
        let names =
            discover_rust_functions("struct Foo; impl Foo { fn method(&self) -> i32 { 0 } }");
        assert_eq!(names, vec!["method"]);
    }

    fn rust_external_calls(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        let root = tree.root_node();
        let local_names = collect_local_names(root, code);
        // Find the first function_item to use as the function node
        fn find_fn(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
            if node.kind() == "function_item" {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(f) = find_fn(child) {
                    return Some(f);
                }
            }
            None
        }
        let func_node = find_fn(root).expect("no function_item found");
        let mut external: std::collections::HashSet<String> = std::collections::HashSet::new();
        collect_external_calls_recursive(func_node, code, &local_names, &mut external);
        let mut v: Vec<String> = external.into_iter().collect();
        v.sort();
        v
    }

    #[test]
    fn test_rust_field_expression_external_call() {
        // vec.push() — push is not a locally defined function → external
        let code = "fn f(mut v: Vec<i32>) { v.push(1); }";
        let calls = rust_external_calls(code);
        assert!(
            calls.contains(&"push".to_string()),
            "push should be external: {:?}",
            calls
        );
    }

    #[test]
    fn test_rust_field_expression_self_local_not_counted() {
        // self.helper() where helper is defined locally → NOT external
        let code = "struct S; impl S { fn f(&self) { self.helper(); } fn helper(&self) {} }";
        let calls = rust_external_calls(code);
        assert!(
            !calls.contains(&"helper".to_string()),
            "helper is local: {:?}",
            calls
        );
    }

    #[test]
    fn test_rust_field_expression_chain_counts_each() {
        // iter.map(...).collect() — map and collect are external
        let code = "fn f(v: Vec<i32>) -> Vec<i32> { v.iter().map(|x| x * 2).collect() }";
        let calls = rust_external_calls(code);
        assert!(
            calls.contains(&"map".to_string()),
            "map should be external: {:?}",
            calls
        );
        assert!(
            calls.contains(&"collect".to_string()),
            "collect should be external: {:?}",
            calls
        );
    }

    // ---- Python function discovery tests ----

    fn discover_python_functions(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::language())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        let mut names = Vec::new();
        visit_functions(&mut cursor, code, &mut |node, src| {
            if let Some(name) = get_function_name(node, src) {
                names.push(name);
            }
        });
        names
    }

    #[test]
    fn test_python_discover_simple_function() {
        let names = discover_python_functions("def simple(x):\n    return x\n");
        assert_eq!(names, vec!["simple"]);
    }

    #[test]
    fn test_python_discover_multiple_functions() {
        let names = discover_python_functions("def foo():\n    pass\ndef bar():\n    pass\n");
        assert_eq!(names, vec!["foo", "bar"]);
    }

    #[test]
    fn test_python_discover_class_method() {
        let names = discover_python_functions(
            "class Foo:\n    def method(self):\n        pass\n    def other(self):\n        pass\n",
        );
        assert_eq!(names, vec!["method", "other"]);
    }

    #[test]
    fn test_python_discover_nested_function() {
        let names = discover_python_functions(
            "def outer():\n    def inner():\n        pass\n    return inner\n",
        );
        assert!(names.contains(&"outer".to_string()));
        assert!(names.contains(&"inner".to_string()));
    }

    #[test]
    fn test_python_discover_decorated_function() {
        let names = discover_python_functions("@decorator\ndef decorated(x):\n    return x\n");
        assert_eq!(names, vec!["decorated"]);
    }

    fn discover_js_functions(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&knots::tree_sitter_javascript::language())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        let mut names = Vec::new();
        visit_functions(&mut cursor, code, &mut |node, src| {
            if let Some(name) = get_function_name(node, src) {
                names.push(name);
            }
        });
        names
    }

    #[test]
    fn test_js_discover_function_declaration() {
        let names = discover_js_functions("function add(a, b) { return a + b; }");
        assert_eq!(names, vec!["add"]);
    }

    #[test]
    fn test_js_discover_multiple_functions() {
        let code = "function foo() {} function bar() {}";
        let mut names = discover_js_functions(code);
        names.sort();
        assert_eq!(names, vec!["bar", "foo"]);
    }

    #[test]
    fn test_js_discover_named_function_expression() {
        let code = "const fn = function myFunc() { return 1; };";
        let names = discover_js_functions(code);
        assert_eq!(names, vec!["myFunc"]);
    }

    #[test]
    fn test_js_discover_class_method() {
        let code = "class Foo { bar() { return 1; } }";
        let names = discover_js_functions(code);
        assert_eq!(names, vec!["bar"]);
    }

    #[test]
    fn test_js_discover_generator_function() {
        let code = "function* gen() { yield 1; }";
        let names = discover_js_functions(code);
        assert_eq!(names, vec!["gen"]);
    }

    /// Build a FunctionMetrics fixture with the given AIRD-component raw values;
    /// fields not relevant to AIRD drivers are left at neutral defaults.
    fn fixture(cognitive: u32, sloc: u32, nesting: u32, test: i32, coupling: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: "f".into(),
            file_path: "src/x.rs".into(),
            start_line: 1,
            end_line: 2,
            mccabe: 0,
            cognitive,
            nesting,
            sloc,
            abc_magnitude: 0.0,
            return_count: 0,
            test_scoring: TestScoringMetric {
                signature_score: 0,
                dependency_score: 0,
                observable_score: 0,
                implementation_score: 0,
                documentation_score: 0,
                total_score: test,
            },
            aird: 0,
            aicp: 0,
            external_calls: 0,
            state_coupling: coupling,
        }
    }

    /// The motivating case from FEEDBACK.md #1: a function dominated by cognitive
    /// then sloc should report those two as drivers, by *raw* value.
    #[test]
    fn test_aird_drivers_cognitive_then_sloc() {
        let func = fixture(215, 492, 1, 0, 0);
        let drivers = aird_drivers(&func, 2);
        assert_eq!(drivers, vec![("cognitive", 215), ("sloc", 492)]);
    }

    /// Drivers are ranked by weighted contribution, not raw magnitude: sloc 200
    /// (capped, 15 pts) outranks a larger-but-lower-weight component.
    #[test]
    fn test_aird_drivers_ranked_by_contribution_not_raw() {
        // cognitive 10 -> 7.33 pts; sloc 200 -> 15 pts (capped). sloc should lead.
        let func = fixture(10, 200, 0, 0, 0);
        let drivers = aird_drivers(&func, 2);
        assert_eq!(drivers[0].0, "sloc");
    }

    /// Zero-contribution components are dropped, and doc is never a driver even
    /// when present (it only reduces AIRD).
    #[test]
    fn test_aird_drivers_drops_zero_contributors() {
        let func = fixture(30, 0, 0, 0, 0);
        let drivers = aird_drivers(&func, 2);
        assert_eq!(drivers, vec![("cognitive", 30)]);
    }

    /// A function with no positive contributors yields no drivers (suffix omitted).
    #[test]
    fn test_aird_drivers_empty_when_all_zero() {
        let func = fixture(0, 0, 0, 0, 0);
        assert!(aird_drivers(&func, 2).is_empty());
    }

    // ---- baseline / ratchet mode ----

    fn func_aird(file: &str, name: &str, aird: u32) -> FunctionMetrics {
        let mut f = fixture(0, 0, 0, 0, 0);
        f.file_path = file.into();
        f.name = name.into();
        f.aird = aird;
        f
    }

    fn aird_thresholds(n: u32) -> Thresholds {
        Thresholds {
            mccabe: None,
            cognitive: None,
            nesting: None,
            sloc: None,
            abc: None,
            returns: None,
            aird: Some(n),
            aicp: None,
            external_calls: None,
        }
    }

    fn baseline_with_aird(file: &str, name: &str, aird: u32) -> Baseline {
        baseline_from_metrics(&[func_aird(file, name, aird)])
    }

    /// Without a baseline, any over-threshold function fails (today's behavior).
    #[test]
    fn test_no_baseline_flags_violation() {
        let metrics = vec![func_aird("a.rs", "f", 98)];
        assert!(check_thresholds(&metrics, &aird_thresholds(85), None, None).is_err());
    }

    /// A pre-existing offender at exactly its baselined score is tolerated.
    #[test]
    fn test_baseline_tolerates_preexisting_equal() {
        let metrics = vec![func_aird("a.rs", "f", 98)];
        let b = baseline_with_aird("a.rs", "f", 98);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_ok());
    }

    /// Still over threshold but better than baseline (98 -> 90) is tolerated.
    #[test]
    fn test_baseline_tolerates_improvement() {
        let metrics = vec![func_aird("a.rs", "f", 90)];
        let b = baseline_with_aird("a.rs", "f", 98);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_ok());
    }

    /// A baselined function that got worse (98 -> 99) is a regression and fails.
    #[test]
    fn test_baseline_flags_regression() {
        let metrics = vec![func_aird("a.rs", "f", 99)];
        let b = baseline_with_aird("a.rs", "f", 98);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_err());
    }

    /// An over-threshold function absent from the baseline is new debt and fails.
    #[test]
    fn test_baseline_flags_new_function() {
        let metrics = vec![func_aird("a.rs", "g", 98)];
        let b = baseline_with_aird("a.rs", "f", 98); // different name -> miss
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_err());
    }

    /// The key is (file, function): same name in a different file is not matched.
    #[test]
    fn test_baseline_key_is_file_and_function() {
        let metrics = vec![func_aird("b.rs", "f", 98)];
        let b = baseline_with_aird("a.rs", "f", 98); // different file -> miss
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_err());
    }

    /// A function under threshold never fails, even if baselined higher.
    #[test]
    fn test_baseline_under_threshold_ok() {
        let metrics = vec![func_aird("a.rs", "f", 10)];
        let b = baseline_with_aird("a.rs", "f", 98);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), None).is_ok());
    }

    /// Baseline survives a serialize/parse round-trip and indexes by (file, fn).
    #[test]
    fn test_baseline_roundtrip() {
        let b = baseline_from_metrics(&[func_aird("a.rs", "f", 98)]);
        let json = serde_json::to_string(&b).unwrap();
        let parsed: Baseline = serde_json::from_str(&json).unwrap();
        let idx = parsed.index();
        assert_eq!(idx.get(&("a.rs", "f")).unwrap().aird, 98);
    }

    // ---- --changed / --since scoping ----

    /// `@@ -a,b +c,d @@` yields the new-file range `(c, d)`.
    #[test]
    fn test_parse_hunk_new_range_full() {
        assert_eq!(parse_hunk_new_range("@@ -10,3 +12,5 @@ fn foo()"), Some((12, 5)));
    }

    /// A single-line hunk omits the count on the `+` side; it defaults to 1.
    #[test]
    fn test_parse_hunk_new_range_default_len() {
        assert_eq!(parse_hunk_new_range("@@ -10 +14 @@"), Some((14, 1)));
    }

    /// A header with no `+` token (or garbage) parses to None.
    #[test]
    fn test_parse_hunk_new_range_malformed() {
        assert_eq!(parse_hunk_new_range("@@ no plus here @@"), None);
        assert_eq!(parse_hunk_new_range("not a hunk"), None);
    }

    /// Build a ChangedLines keyed by a path that does not exist on disk so
    /// `canonicalize_path` falls back to the literal path on both insert and
    /// lookup — lets us unit-test overlap logic without touching the filesystem.
    fn changed_with(file: &str, ranges: &[(u32, u32)]) -> ChangedLines {
        let mut map = HashMap::new();
        map.insert(canonicalize_path(Path::new(file)), ranges.to_vec());
        ChangedLines { map }
    }

    #[test]
    fn test_overlaps_intersecting() {
        let c = changed_with("nope_a.rs", &[(10, 20)]);
        assert!(c.overlaps("nope_a.rs", 15, 25)); // straddles the start
        assert!(c.overlaps("nope_a.rs", 5, 12)); // straddles the end
        assert!(c.overlaps("nope_a.rs", 1, 100)); // contains it
        assert!(c.overlaps("nope_a.rs", 20, 20)); // touches the boundary
    }

    #[test]
    fn test_overlaps_disjoint_and_missing_file() {
        let c = changed_with("nope_b.rs", &[(10, 20)]);
        assert!(!c.overlaps("nope_b.rs", 1, 9)); // entirely before
        assert!(!c.overlaps("nope_b.rs", 21, 30)); // entirely after
        assert!(!c.overlaps("other.rs", 10, 20)); // file not in the diff
    }

    /// An over-threshold function that overlaps a changed range fails the gate.
    #[test]
    fn test_changed_gates_touched_function() {
        let mut f = func_aird("nope_c.rs", "f", 98);
        f.start_line = 10;
        f.end_line = 30;
        let metrics = vec![f];
        let c = changed_with("nope_c.rs", &[(15, 16)]);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), None, Some(&c)).is_err());
    }

    /// An over-threshold function with no overlapping change is skipped (passes).
    #[test]
    fn test_changed_skips_untouched_function() {
        let mut f = func_aird("nope_d.rs", "f", 98);
        f.start_line = 10;
        f.end_line = 30;
        let metrics = vec![f];
        let c = changed_with("nope_d.rs", &[(100, 110)]);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), None, Some(&c)).is_ok());
    }

    // ---- --explain / AI-metric pointer ----

    /// Every ExplainMetric variant yields a non-empty explanation that leads
    /// with the metric's name (so `--explain <m>` is always useful).
    #[test]
    fn test_explain_metric_nonempty() {
        use ExplainMetric::*;
        for m in [
            Mccabe, Cognitive, Nesting, Sloc, Abc, Returns, Aird, Aicp, ExternalCalls,
        ] {
            let text = explain_metric(m);
            assert!(!text.is_empty());
            assert!(text.lines().next().unwrap().len() > 3);
        }
        assert!(explain_metric(Aird).contains("AI Reasoning Difficulty"));
        assert!(explain_metric(Aicp).contains("AI Context Pressure"));
    }

    /// The pointer expands only the acronyms that were actually violated, and is
    /// absent when no AI metric is involved.
    #[test]
    fn test_ai_metric_pointer() {
        assert!(ai_metric_pointer(false, false).is_none());
        let aird = ai_metric_pointer(true, false).unwrap();
        assert!(aird.contains("AI Reasoning Difficulty") && !aird.contains("Context Pressure"));
        let aicp = ai_metric_pointer(false, true).unwrap();
        assert!(aicp.contains("AI Context Pressure") && !aicp.contains("Reasoning Difficulty"));
        let both = ai_metric_pointer(true, true).unwrap();
        assert!(both.contains("AI Reasoning Difficulty") && both.contains("AI Context Pressure"));
    }

    /// --changed composes with --baseline: a touched function that regressed
    /// beyond its baseline still fails; the changed-scope only narrows the set.
    #[test]
    fn test_changed_composes_with_baseline() {
        let mut f = func_aird("nope_e.rs", "f", 99);
        f.start_line = 10;
        f.end_line = 30;
        let metrics = vec![f];
        let b = baseline_with_aird("nope_e.rs", "f", 98); // worsened 98 -> 99
        let c = changed_with("nope_e.rs", &[(12, 12)]);
        assert!(check_thresholds(&metrics, &aird_thresholds(85), Some(&b), Some(&c)).is_err());
    }
}

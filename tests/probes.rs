//! Conformance to metric definitions: each probe in validation/probes states the
//! value the published definition gives, and knots must match it or the
//! difference must be recorded in validation/probes/conformance.toml.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(serde::Deserialize)]
struct Manifest {
    departure: Vec<Departure>,
}

#[derive(serde::Deserialize)]
struct Departure {
    probe: String,
    function: String,
    metric: String,
    definition: i64,
    knots: i64,
    status: String,
    reason: String,
}

/// One `expect` value: the probe, function, metric and the definition's value.
struct Expected {
    probe: String,
    function: String,
    metric: String,
    value: i64,
}

fn probes_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("validation/probes")
}

fn load_manifest() -> Manifest {
    let text = std::fs::read_to_string(probes_dir().join("conformance.toml")).unwrap();
    toml::from_str(&text).unwrap()
}

/// The `expect ...` text of a `//` or `--` comment line, if it is one.
fn expect_clause(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let comment = line
        .strip_prefix("//")
        .or_else(|| line.strip_prefix("--"))?;
    comment.trim().strip_prefix("expect ")
}

/// `(function, metric, value)` triples from one clause. Dotted keys
/// (`mccabe.modified`) name a variant other tools use and are not checked.
fn parse_clause(clause: &str) -> Vec<(String, String, i64)> {
    let (function, pairs) = clause.split_once(' ').unwrap_or((clause, ""));
    checked_pairs(pairs)
        .into_iter()
        .map(|(metric, value)| (function.to_string(), metric, value))
        .collect()
}

fn checked_pairs(pairs: &str) -> Vec<(String, i64)> {
    pairs
        .split_whitespace()
        .filter_map(|pair| pair.split_once('='))
        .filter(|(metric, _)| !metric.contains('.'))
        .map(|(metric, value)| (metric.to_string(), value.parse().unwrap()))
        .collect()
}

fn expectations(probe: &str, path: &Path) -> Vec<Expected> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter_map(expect_clause)
        .flat_map(parse_clause)
        .map(|(function, metric, value)| Expected {
            probe: probe.to_string(),
            function,
            metric,
            value,
        })
        .collect()
}

fn run_knots(path: &Path) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knots"));
    let output = command
        .arg(path)
        .args(["--format", "ndjson"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn knots_rows(path: &Path) -> HashMap<String, serde_json::Value> {
    let stdout = run_knots(path);
    let rows = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok());
    rows.map(keyed_by_function).collect()
}

fn keyed_by_function(row: serde_json::Value) -> (String, serde_json::Value) {
    (
        row["function"].as_str().unwrap_or_default().to_string(),
        row,
    )
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(dir).unwrap().flatten();
    entries.map(|e| e.path()).collect()
}

/// `(language/file, path)` for every probe, sorted.
fn probe_files() -> Vec<(String, PathBuf)> {
    let root = probes_dir();
    let paths = language_dirs(&root)
        .into_iter()
        .flat_map(|dir| files_in(&dir));
    let mut files: Vec<(String, PathBuf)> = paths.map(|p| (relative_name(&root, &p), p)).collect();
    files.sort();
    files
}

fn language_dirs(root: &Path) -> Vec<PathBuf> {
    files_in(root)
        .into_iter()
        .filter(|dir| dir.is_dir())
        .collect()
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

fn find_departure(manifest: &Manifest, e: &Expected) -> Option<usize> {
    manifest
        .departure
        .iter()
        .position(|d| d.probe == e.probe && d.function == e.function && d.metric == e.metric)
}

/// The failure, if any, for one expectation given knots' value and its departure.
fn judge(e: &Expected, got: i64, departure: Option<&Departure>) -> Option<String> {
    let at = format!("{} {} {}", e.probe, e.function, e.metric);
    match (got == e.value, departure) {
        (true, None) => None,
        (true, Some(_)) => Some(format!(
            "{at}: now matches the definition ({}); remove its departure",
            e.value
        )),
        (false, None) => Some(format!(
            "{at}: knots {got}, definition {}; fix knots or record a departure",
            e.value
        )),
        (false, Some(d)) => mismatched_record(&at, e.value, got, d),
    }
}

fn mismatched_record(at: &str, want: i64, got: i64, d: &Departure) -> Option<String> {
    let agrees = d.definition == want && d.knots == got;
    (!agrees).then(|| {
        format!(
            "{at}: recorded {} / {}, probe gives {want} / {got}",
            d.definition, d.knots
        )
    })
}

fn check_probe(probe: &str, path: &Path, manifest: &Manifest, used: &mut [bool]) -> Vec<String> {
    let rows = knots_rows(path);
    let expected = expectations(probe, path);
    if expected.is_empty() {
        return vec![format!("{probe}: no `expect` lines")];
    }
    expected
        .iter()
        .filter_map(|e| check_one(e, &rows, manifest, used))
        .collect()
}

fn check_one(
    e: &Expected,
    rows: &HashMap<String, serde_json::Value>,
    manifest: &Manifest,
    used: &mut [bool],
) -> Option<String> {
    let departure = find_departure(manifest, e);
    departure.iter().for_each(|&i| used[i] = true);
    verdict(e, rows, departure.map(|i| &manifest.departure[i]))
}

fn verdict(
    e: &Expected,
    rows: &HashMap<String, serde_json::Value>,
    departure: Option<&Departure>,
) -> Option<String> {
    match rows
        .get(&e.function)
        .and_then(|row| row[&e.metric].as_i64())
    {
        Some(got) => judge(e, got, departure),
        None => Some(format!(
            "{}: knots reports no {} for {}",
            e.probe, e.metric, e.function
        )),
    }
}

fn manifest_problems(manifest: &Manifest, used: &[bool]) -> Vec<String> {
    let mut problems = Vec::new();
    for (d, was_used) in manifest.departure.iter().zip(used) {
        let at = format!("conformance.toml: {} {} {}", d.probe, d.function, d.metric);
        if !was_used {
            problems.push(format!("{at}: matches no probe expectation"));
        }
        if d.status.is_empty() || d.reason.trim().is_empty() {
            problems.push(format!("{at}: needs a status and a reason"));
        }
    }
    problems
}

#[test]
fn knots_matches_each_definition_or_a_recorded_departure() {
    let manifest = load_manifest();
    let mut used = vec![false; manifest.departure.len()];
    let mut failures = Vec::new();
    for (probe, path) in probe_files() {
        failures.extend(check_probe(&probe, &path, &manifest, &mut used));
    }
    failures.extend(manifest_problems(&manifest, &used));
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

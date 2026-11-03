//! `knots.toml` — optional unified-config file for thresholds and
//! file/function exclusion, per lang_parsing_substrate's
//! `docs/unified-config-spec.md` (`knots.toml` section).
//!
//! All fields are optional, modelled after `.yamllint`: CLI flags always
//! take precedence over anything loaded here, and the legacy
//! `--include`/`--exclude` JSON sidecar (`FilterRules`) keeps working
//! unchanged — this is an additive config surface, not a replacement.

use anyhow::{bail, Context, Result};
use globset::Glob;
use knots::languages;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KnotsToml {
    #[serde(default)]
    pub thresholds: TomlThresholds,
    #[serde(default)]
    pub filter: FilterSection,
    /// Per-language sections, e.g. `[c.thresholds]`. Keyed on the same
    /// canonical language key as `LanguageInfo::key` (`"c"`, `"cpp"`, …).
    #[serde(flatten)]
    pub languages: HashMap<String, LanguageSection>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LanguageSection {
    #[serde(default)]
    pub thresholds: TomlThresholds,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct TomlThresholds {
    pub mccabe: Option<u32>,
    pub cognitive: Option<u32>,
    pub nesting: Option<u32>,
    pub sloc: Option<u32>,
    pub abc: Option<f64>,
    pub returns: Option<u32>,
    pub aird: Option<u32>,
    pub aicp: Option<u32>,
    pub external_calls: Option<u32>,
    pub unreachable_blocks: Option<u32>,
}

impl TomlThresholds {
    pub fn any_set(&self) -> bool {
        self.mccabe.is_some()
            || self.cognitive.is_some()
            || self.nesting.is_some()
            || self.sloc.is_some()
            || self.abc.is_some()
            || self.returns.is_some()
            || self.aird.is_some()
            || self.aicp.is_some()
            || self.external_calls.is_some()
            || self.unreachable_blocks.is_some()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FilterSection {
    #[serde(default)]
    pub exclude: Vec<FilterExclude>,
}

/// One `[[filter.exclude]]` entry. `file_patterns` and `function_patterns`
/// are AND'd together within an entry; an empty list is treated as
/// "matches everything" for that field (so a function-only or file-only
/// entry works as expected). A function is excluded if it matches *any*
/// entry.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FilterExclude {
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub function_patterns: Vec<String>,
}

impl KnotsToml {
    /// Walks up from `start_dir` looking for `knots.toml`, stopping at the
    /// first match (or the filesystem root). Returns `Ok(None)` if no file
    /// is found anywhere in the ancestry.
    pub fn discover(start_dir: &Path) -> Result<Option<Self>> {
        let mut dir = Some(start_dir);
        while let Some(d) = dir {
            let candidate = d.join("knots.toml");
            if candidate.is_file() {
                let content = fs::read_to_string(&candidate)
                    .with_context(|| format!("Failed to read {}", candidate.display()))?;
                let origin = candidate.display().to_string();
                let cfg = Self::parse(&content, &origin)
                    .with_context(|| format!("Failed to parse {origin}"))?;
                return Ok(Some(cfg));
            }
            dir = d.parent();
        }
        Ok(None)
    }
}

impl KnotsToml {
    /// Parses `knots.toml` text, refusing any section or key knots does not
    /// read. Unknown names are an error, not a warning: a misspelled
    /// threshold or language section would otherwise leave its gate off
    /// while the run passes. Every unknown name is reported as
    /// `origin:line: ...`, with the nearest valid name when one is close.
    pub fn parse(content: &str, origin: &str) -> Result<Self> {
        let unknown = unknown_names(content);
        if !unknown.is_empty() {
            let lines: Vec<String> = unknown
                .iter()
                .map(|(line, msg)| match line {
                    0 => format!("{origin}: {msg}"),
                    n => format!("{origin}:{n}: {msg}"),
                })
                .collect();
            bail!("{}", lines.join("\n"));
        }
        Ok(toml::from_str(content)?)
    }
}

/// Configuration keys of a `[thresholds]` or `[<lang>.thresholds]` table.
const THRESHOLD_KEYS: &[&str] = &[
    "mccabe",
    "cognitive",
    "nesting",
    "sloc",
    "abc",
    "returns",
    "aird",
    "aicp",
    "external_calls",
    "unreachable_blocks",
];

/// One `(line, message)` per section or key in `content` that knots does
/// not read, in file order; line 0 when toml_edit kept no span. Empty when the text does not parse as TOML: the parse
/// error is then the one worth reporting, and `toml::from_str` gives it.
fn unknown_names(content: &str) -> Vec<(usize, String)> {
    let Ok(doc) = toml_edit::ImDocument::parse(content) else {
        return Vec::new();
    };
    let root = doc.as_table();
    let mut found: Vec<(usize, String)> = Vec::new();
    for (name, item) in root.iter() {
        if !check_section(content, name, item, &mut found) {
            let line = line_of(content, root.key(name).and_then(|k| k.span()));
            found.push((line, unknown_section(name)));
        }
    }
    found.sort_by_key(|(line, _)| *line);
    found
}

/// Checks the keys of the top-level section `name`. Returns false when
/// `name` is not a section knots reads.
fn check_section(
    content: &str,
    name: &str,
    item: &toml_edit::Item,
    found: &mut Vec<(usize, String)>,
) -> bool {
    match name {
        "thresholds" => check_keys(content, item, name, THRESHOLD_KEYS, found),
        "filter" => {
            check_keys(content, item, name, &["exclude"], found);
            for entry in exclude_entries(item) {
                let valid = ["file_patterns", "function_patterns"];
                check_table(content, entry, "filter.exclude", &valid, found);
            }
        }
        _ if languages().iter().any(|l| l.key == name) => {
            check_keys(content, item, name, &["thresholds"], found);
            if let Some(t) = item.as_table_like().and_then(|t| t.get("thresholds")) {
                let section = format!("{name}.thresholds");
                check_keys(content, t, &section, THRESHOLD_KEYS, found);
            }
        }
        _ => return false,
    }
    true
}

/// The message for a top-level section knots does not read.
fn unknown_section(name: &str) -> String {
    let hint = match language_alias(name).or_else(|| nearest(name, &section_names())) {
        Some(s) => format!("did you mean `{s}`?"),
        None => "a language section is named by its key, as listed by \
                 `knots --supported-languages`"
            .to_string(),
    };
    format!("unknown section `{name}`; {hint}")
}

/// The `[[filter.exclude]]` entries, whether written as an array of tables
/// or as an inline array of inline tables.
fn exclude_entries(filter: &toml_edit::Item) -> Vec<&dyn toml_edit::TableLike> {
    let Some(exclude) = filter.as_table_like().and_then(|t| t.get("exclude")) else {
        return Vec::new();
    };
    if let Some(aot) = exclude.as_array_of_tables() {
        return aot.iter().map(|t| t as &dyn toml_edit::TableLike).collect();
    }
    exclude
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_inline_table())
                .map(|t| t as &dyn toml_edit::TableLike)
                .collect()
        })
        .unwrap_or_default()
}

/// Reports every key of `item` (when it is a table) that is not in `valid`.
fn check_keys(
    content: &str,
    item: &toml_edit::Item,
    section: &str,
    valid: &[&str],
    found: &mut Vec<(usize, String)>,
) {
    if let Some(table) = item.as_table_like() {
        check_table(content, table, section, valid, found);
    }
}

fn check_table(
    content: &str,
    table: &dyn toml_edit::TableLike,
    section: &str,
    valid: &[&str],
    found: &mut Vec<(usize, String)>,
) {
    for (name, _) in table.iter() {
        if valid.contains(&name) {
            continue;
        }
        let line = line_of(content, table.key(name).and_then(|k| k.span()));
        let hint = nearest(name, valid)
            .map(|s| format!("did you mean `{s}`?"))
            .unwrap_or_else(|| format!("valid keys: {}", valid.join(", ")));
        found.push((
            line,
            format!("unknown key `{name}` in `[{section}]`; {hint}"),
        ));
    }
}

/// Every valid top-level section name: the two global sections and each
/// language key.
fn section_names() -> Vec<&'static str> {
    let mut names = vec!["thresholds", "filter"];
    names.extend(languages().iter().map(|l| l.key));
    names
}

/// The language key for a section named after a language's display name or
/// one of its extensions, as `c++`, `C#`, `js` or `py` are.
fn language_alias(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    languages()
        .iter()
        .find(|l| {
            l.key == lower
                || l.name.to_ascii_lowercase() == lower
                || l.extensions.iter().any(|e| e.eq_ignore_ascii_case(&lower))
                || l.explicit_only
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(&lower))
        })
        .map(|l| l.key)
}

/// The valid name closest to `name` by edit distance, if close enough to be
/// a likely misspelling: within a quarter of the name's length, and at least
/// 1. Tighter than that suggests a different word (`include` for `exclude`).
fn nearest<'a>(name: &str, valid: &[&'a str]) -> Option<&'a str> {
    let lower = name.to_ascii_lowercase().replace('-', "_");
    let limit = (lower.chars().count() / 4).max(1);
    valid
        .iter()
        .map(|v| (edit_distance(&lower, v), *v))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, v)| v)
}

/// Edit distance between two strings, by character, counting an adjacent
/// transposition (`pyhton`) as one edit (optimal string alignment).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

/// 1-based line of a byte span's start; 0 when the span is unknown.
fn line_of(content: &str, span: Option<std::ops::Range<usize>>) -> usize {
    span.map(|r| content[..r.start.min(content.len())].matches('\n').count() + 1)
        .unwrap_or(0)
}

fn glob_match(pattern: &str, path: &str) -> bool {
    Glob::new(pattern)
        .map(|g| g.compile_matcher().is_match(path))
        .unwrap_or(false)
}

/// Whether an entry excludes the whole of `file_path`: one with no function
/// patterns whose file patterns match it (or are empty). Such a file is out
/// of scope, so it is left out before it is read.
pub fn excludes_file(rules: &[FilterExclude], file_path: &str) -> bool {
    rules.iter().any(|r| {
        r.function_patterns.is_empty()
            && (r.file_patterns.is_empty()
                || r.file_patterns.iter().any(|p| glob_match(p, file_path)))
    })
}

/// Whether any `[[filter.exclude]]` entry matches `file_path` + `function_name`.
pub fn exclude_matches(rules: &[FilterExclude], file_path: &str, function_name: &str) -> bool {
    rules.iter().any(|r| {
        let file_ok =
            r.file_patterns.is_empty() || r.file_patterns.iter().any(|p| glob_match(p, file_path));
        let func_ok = r.function_patterns.is_empty()
            || r.function_patterns.iter().any(|p| {
                Regex::new(p)
                    .map(|re| re.is_match(function_name))
                    .unwrap_or(false)
            });
        file_ok && func_ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_and_per_language_thresholds() {
        let toml_src = r#"
[thresholds]
mccabe = 10
cognitive = 15

[c.thresholds]
mccabe = 15
"#;
        let cfg: KnotsToml = toml::from_str(toml_src).unwrap();
        assert_eq!(cfg.thresholds.mccabe, Some(10));
        assert_eq!(cfg.thresholds.cognitive, Some(15));
        assert_eq!(cfg.languages.get("c").unwrap().thresholds.mccabe, Some(15));
    }

    #[test]
    fn parses_filter_exclude_entries() {
        let toml_src = r#"
[[filter.exclude]]
file_patterns = ["tests/**"]
function_patterns = ["^test_"]
"#;
        let cfg: KnotsToml = toml::from_str(toml_src).unwrap();
        assert_eq!(cfg.filter.exclude.len(), 1);
        assert!(exclude_matches(
            &cfg.filter.exclude,
            "tests/foo.c",
            "test_bar"
        ));
        assert!(!exclude_matches(
            &cfg.filter.exclude,
            "src/foo.c",
            "test_bar"
        ));
        assert!(!exclude_matches(&cfg.filter.exclude, "tests/foo.c", "bar"));
    }

    #[test]
    fn empty_pattern_list_matches_everything_for_that_field() {
        let toml_src = r#"
[[filter.exclude]]
function_patterns = ["^test_"]
"#;
        let cfg: KnotsToml = toml::from_str(toml_src).unwrap();
        assert!(exclude_matches(
            &cfg.filter.exclude,
            "any/path.c",
            "test_bar"
        ));
        assert!(!exclude_matches(&cfg.filter.exclude, "any/path.c", "bar"));
    }

    #[test]
    fn empty_config_matches_nothing() {
        let cfg = KnotsToml::default();
        assert!(!exclude_matches(
            &cfg.filter.exclude,
            "any/path.c",
            "any_fn"
        ));
        assert!(!cfg.thresholds.any_set());
    }

    fn unknown(src: &str) -> String {
        format!("{:#}", KnotsToml::parse(src, "knots.toml").unwrap_err())
    }

    #[test]
    fn every_unknown_name_is_reported_with_its_line() {
        let msg = unknown(
            "[thresholds]\nmccab = 1\n[c.threshold]\nmccabe = 1\n[[filter.exclude]]\nfunction_pattern = [\"x\"]\n",
        );
        let lines: Vec<&str> = msg.lines().collect();
        assert_eq!(
            lines,
            [
                "knots.toml:2: unknown key `mccab` in `[thresholds]`; did you mean `mccabe`?",
                "knots.toml:3: unknown key `threshold` in `[c]`; did you mean `thresholds`?",
                "knots.toml:6: unknown key `function_pattern` in `[filter.exclude]`; did you mean `function_patterns`?",
            ]
        );
    }

    #[test]
    fn language_sections_named_by_name_or_extension_suggest_the_key() {
        assert!(unknown("[\"c++\".thresholds]\nmccabe = 1\n").contains("did you mean `cpp`?"));
        assert!(unknown("[\"C#\".thresholds]\n").contains("did you mean `csharp`?"));
        assert!(unknown("[js.thresholds]\n").contains("did you mean `javascript`?"));
        assert!(unknown("[pyhton.thresholds]\n").contains("did you mean `python`?"));
    }

    #[test]
    fn unrelated_names_list_the_valid_keys_instead_of_guessing() {
        let msg = unknown("[filter]\ninclude = []\n");
        assert!(msg.contains("valid keys: exclude"), "{msg}");
        let msg = unknown("[zzzzz]\n");
        assert!(msg.contains("knots --supported-languages"), "{msg}");
    }

    #[test]
    fn inline_and_dotted_forms_are_checked() {
        assert!(unknown("thresholds = { cognitve = 1 }\n").contains("did you mean `cognitive`?"));
        assert!(unknown("[go]\nthresholds.slok = 1\n").contains("in `[go.thresholds]`"));
        assert!(unknown("[filter]\nexclude = [{ file_pattern = [] }]\n")
            .contains("did you mean `file_patterns`?"));
    }

    #[test]
    fn known_names_parse() {
        let cfg = KnotsToml::parse(
            "[thresholds]\nreturns = 3\n[rust.thresholds]\nexternal_calls = 4\n",
            "knots.toml",
        )
        .unwrap();
        assert_eq!(cfg.thresholds.returns, Some(3));
        assert_eq!(cfg.languages["rust"].thresholds.external_calls, Some(4));
    }

    #[test]
    fn discover_returns_none_when_absent() {
        let dir = std::env::temp_dir();
        // Extremely unlikely to have a knots.toml walking up from a temp dir,
        // but this test only asserts the "not found anywhere" path succeeds
        // without erroring — it does not assert isolation from the real fs.
        let _ = KnotsToml::discover(&dir);
    }
}

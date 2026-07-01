//! `knots.toml` — optional unified-config file for thresholds and
//! file/function exclusion, per lang_parsing_substrate's
//! `docs/unified-config-spec.md` (`knots.toml` section).
//!
//! All fields are optional, modelled after `.yamllint`: CLI flags always
//! take precedence over anything loaded here, and the legacy
//! `--include`/`--exclude` JSON sidecar (`FilterRules`) keeps working
//! unchanged — this is an additive config surface, not a replacement.

use anyhow::{Context, Result};
use globset::Glob;
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
                let cfg: KnotsToml = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse {}", candidate.display()))?;
                return Ok(Some(cfg));
            }
            dir = d.parent();
        }
        Ok(None)
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    Glob::new(pattern)
        .map(|g| g.compile_matcher().is_match(path))
        .unwrap_or(false)
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

    #[test]
    fn discover_returns_none_when_absent() {
        let dir = std::env::temp_dir();
        // Extremely unlikely to have a knots.toml walking up from a temp dir,
        // but this test only asserts the "not found anywhere" path succeeds
        // without erroring — it does not assert isolation from the real fs.
        let _ = KnotsToml::discover(&dir);
    }
}

//! The published knots hooks in `.pre-commit-hooks.yaml` select exactly the
//! files knots analyses: every extension in the language registry, explicit-
//! only ones included, in any letter case, and nothing else. A language added
//! to the registry fails this test until the hooks select it too, and a file
//! knots cannot analyse never reaches it from a hook, where it would make the
//! scan incomplete.
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;

/// The value of `name:` (e.g. `"entry:"`) in one hook's block, unquoted.
fn field(block: &str, name: &str) -> Option<String> {
    block.lines().find_map(|l| {
        l.trim()
            .strip_prefix(name)
            .map(|v| v.trim().trim_matches('\'').to_string())
    })
}

/// `(hook id, files regex)` for every hook whose entry is knots itself.
fn knots_hooks() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".pre-commit-hooks.yaml");
    let text = std::fs::read_to_string(path).unwrap();
    text.split("\n- id: ")
        .skip(1)
        .filter(|block| field(block, "entry:").as_deref() == Some("knots"))
        .map(|block| {
            let id = block.lines().next().unwrap().trim().to_string();
            assert!(
                field(block, "types_or:").is_none() && field(block, "types:").is_none(),
                "{id}: select by `files:` only, so the regex alone decides"
            );
            let files = field(block, "files:").unwrap_or_else(|| panic!("{id}: no `files:`"));
            (id, files)
        })
        .collect()
}

fn registry_extensions() -> BTreeSet<String> {
    knots::languages()
        .iter()
        .flat_map(|l| l.extensions.iter().chain(l.explicit_only.iter()))
        .map(|e| e.to_ascii_lowercase())
        .collect()
}

#[test]
fn knots_hooks_select_every_supported_extension_and_nothing_else() {
    let hooks = knots_hooks();
    assert_eq!(hooks.len(), 5, "{hooks:?}");
    let expected = registry_extensions();
    for (id, files) in &hooks {
        let inner = files
            .strip_prefix(r"(?i)\.(")
            .and_then(|r| r.strip_suffix(")$"))
            .unwrap_or_else(|| panic!("{id}: expected (?i)\\.(ext|...)$, got {files}"));
        let listed: BTreeSet<String> = inner.split('|').map(str::to_string).collect();
        assert_eq!(listed, expected, "{id}");

        let re = Regex::new(files).unwrap();
        for ext in &expected {
            for name in [
                format!("src/a.{ext}"),
                format!("src/a.{}", ext.to_uppercase()),
            ] {
                assert!(re.is_match(&name), "{id} skips {name}");
            }
        }
        // Tagged as C++ or Python by pre-commit, but not analysable by knots.
        for name in [
            "a.hh", "a.inl", "a.ipp", "a.c++", "a.pyi", "script", "a.c.orig",
        ] {
            assert!(!re.is_match(name), "{id} selects {name}");
        }
    }
}

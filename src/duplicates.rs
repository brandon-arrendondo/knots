//! Corpus-wide duplicate function detection, built on the substrate's
//! structural fingerprinting (`lang_parsing_substrate::fingerprint`).
//!
//! Two functions group together when their AST subtrees hash identically —
//! same node kinds in the same shape, regardless of identifier/literal text.
//! That catches Type-1 (byte-identical) and Type-2 (renamed) clones; it does
//! not catch Type-3 near-misses (e.g. one extra statement), which would need
//! a fuzzy similarity threshold the substrate deliberately doesn't provide.
//!
//! Gated behind `--find-duplicates` (see `RunContext::find_duplicates`)
//! rather than folded into the default `--recursive` pass: fingerprinting
//! is corpus-wide and requires a second parse of every file, unlike the
//! per-function metrics that come for free from the first pass.

use lang_parsing_substrate::{duplicate_groups, CorpusFingerprint};
use std::collections::HashSet;
use std::path::Path;

/// Functions smaller than this (in AST node count) are skipped — a
/// single-line getter hashing the same as another single-line getter isn't
/// a meaningful clone, just noise.
///
/// This floor alone doesn't catch all boilerplate: a one-line accessor like
/// `fn src(&self) -> &str { self.src }` can clear 20 AST nodes (type
/// annotations, field-access chains) while still being a single line of
/// source. [`is_trivial_noise_group`] catches that case on line span
/// instead of node count.
pub const MIN_DUPLICATE_NODES: usize = 20;

/// A group where every member's body spans this many lines or fewer is
/// "trivial" — a getter, a one-assert test, a dispatch stub — and only
/// counts as noise below [`TRIVIAL_MIN_REPEAT`] repeats. See
/// [`is_trivial_noise_group`].
pub const TRIVIAL_BODY_LINE_SPAN: usize = 3;

/// Trivial (small-body) groups repeating at least this many times are kept
/// regardless of size — a getter duplicated across 4+ files is more likely
/// widespread copy-paste worth a shared trait than incidental shape overlap.
pub const TRIVIAL_MIN_REPEAT: usize = 4;

/// One member of a duplicate group, detached from the borrowed fingerprint
/// data so callers don't need to keep the corpus fingerprint list alive.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMember {
    pub file_path: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub node_count: usize,
    /// Byte-diff percentage against the group's first member (0 = identical
    /// bytes, higher = more textual divergence past shape). `None` for the
    /// first member itself, or when the body was too large to diff cheaply.
    /// Populated by the caller after re-reading source files — see
    /// `byte_diff_percent`; this module has no file I/O of its own.
    pub diff_from_first: Option<u32>,
}

/// Which low-value group categories to drop from a duplicate-detection pass.
/// Both default to "exclude" (the CLI's normal behavior); each corresponds
/// to an `--include-*` opt-out flag.
#[derive(Debug, Clone, Copy)]
pub struct DuplicateFilters {
    pub exclude_fixture_pairs: bool,
    pub exclude_trivial: bool,
}

/// Result of a duplicate-detection pass: the groups worth reporting, plus
/// how many were dropped in each low-value category (see
/// [`is_fixture_pair_group`] and [`is_trivial_noise_group`]).
pub struct DuplicateGroupsResult {
    pub groups: Vec<Vec<DuplicateMember>>,
    pub excluded_fixture_pairs: usize,
    pub excluded_trivial: usize,
}

/// Groups corpus fingerprints into duplicate groups (2+ members sharing an
/// identical structural hash), sorted largest-group-first, then by node
/// count — the clones most worth acting on come first.
///
/// `filters` controls which low-value categories get dropped before
/// reporting; see [`DuplicateFilters`].
pub fn find_duplicate_groups(
    fingerprints: &[CorpusFingerprint<String>],
    filters: DuplicateFilters,
) -> DuplicateGroupsResult {
    let mut groups = sorted_duplicate_groups(fingerprints);
    let excluded_fixture_pairs = drop_matching(
        &mut groups,
        filters.exclude_fixture_pairs,
        is_fixture_pair_group,
    );
    let excluded_trivial =
        drop_matching(&mut groups, filters.exclude_trivial, is_trivial_noise_group);
    DuplicateGroupsResult {
        groups,
        excluded_fixture_pairs,
        excluded_trivial,
    }
}

/// Builds the raw duplicate groups (2+ members sharing a structural hash),
/// sorted largest-group-first then by node count.
fn sorted_duplicate_groups(
    fingerprints: &[CorpusFingerprint<String>],
) -> Vec<Vec<DuplicateMember>> {
    let mut groups = grouped_members(fingerprints);
    groups.sort_by_key(|g| std::cmp::Reverse(group_sort_key(g)));
    groups
}

fn grouped_members(fingerprints: &[CorpusFingerprint<String>]) -> Vec<Vec<DuplicateMember>> {
    duplicate_groups(fingerprints)
        .into_iter()
        .map(|members| members.iter().copied().map(to_duplicate_member).collect())
        .collect()
}

/// `(group size, node count)` — the clones most worth acting on (bigger
/// groups, then bigger bodies) sort first when compared in reverse.
fn group_sort_key(group: &[DuplicateMember]) -> (usize, usize) {
    (group.len(), group[0].node_count)
}

fn to_duplicate_member(m: &CorpusFingerprint<String>) -> DuplicateMember {
    DuplicateMember {
        file_path: m.source.clone(),
        name: m.fingerprint.name.clone(),
        start_line: m.fingerprint.start_line,
        end_line: m.fingerprint.end_line,
        start_byte: m.fingerprint.start_byte,
        end_byte: m.fingerprint.end_byte,
        node_count: m.fingerprint.node_count,
        diff_from_first: None,
    }
}

/// When `enabled`, drops every group matching `predicate` and returns how
/// many were dropped; a no-op returning 0 otherwise.
fn drop_matching(
    groups: &mut Vec<Vec<DuplicateMember>>,
    enabled: bool,
    predicate: impl Fn(&[DuplicateMember]) -> bool,
) -> usize {
    if !enabled {
        return 0;
    }
    let before = groups.len();
    groups.retain(|g| !predicate(g));
    before - groups.len()
}

const FIXTURE_TAG_PAIRS: &[(&str, &str)] = &[
    ("pass", "fail"),
    ("compliant", "noncompliant"),
    ("good", "bad"),
    ("accept", "reject"),
    ("valid", "invalid"),
];

/// The pass/fail (or accept/reject, good/bad) directory-name pair a fixture
/// path belongs to, if any. Matched as a whole path component so
/// `tests/pass/foo.c` matches but `tests/passthrough/foo.c` does not.
fn fixture_tag(file_path: &str) -> Option<&'static str> {
    Path::new(file_path)
        .components()
        .find_map(component_fixture_tag)
}

/// The canonical tag (e.g. `"pass"`) a single path component matches, if any.
fn component_fixture_tag(component: std::path::Component) -> Option<&'static str> {
    let name = component.as_os_str().to_str()?.to_ascii_lowercase();
    FIXTURE_TAG_PAIRS
        .iter()
        .flat_map(|&(a, b)| [a, b])
        .find(|&tag| tag == name)
}

/// Path with its fixture-tag component (e.g. `pass`/`fail`) replaced by a
/// placeholder, so two sibling fixture files normalize to the same string.
fn normalize_fixture_path(file_path: &str, tag: &str) -> String {
    Path::new(file_path)
        .components()
        .map(|c| normalize_fixture_component(c, tag))
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_fixture_component(component: std::path::Component, tag: &str) -> String {
    let name = component.as_os_str().to_string_lossy();
    if name.to_ascii_lowercase() == tag {
        "*".to_string()
    } else {
        name.into_owned()
    }
}

/// True when every member of a duplicate group sits under a recognized
/// pass/fail-style fixture directory, at least two distinct tags are
/// represented (i.e. it's actually a cross pair, not two "pass" files that
/// happen to match), and the paths are otherwise identical once the tag is
/// normalized away. This is the heuristic used to auto-exclude CERT-C-style
/// tests/pass/*.c vs tests/fail/*.c fixtures, which are intentionally
/// near-identical by design and not extraction candidates.
fn is_fixture_pair_group(group: &[DuplicateMember]) -> bool {
    if group.len() < 2 {
        return false;
    }
    let mut tags = HashSet::new();
    let mut normalized = HashSet::new();
    for member in group {
        let Some(tag) = fixture_tag(&member.file_path) else {
            return false;
        };
        tags.insert(tag);
        normalized.insert(normalize_fixture_path(&member.file_path, tag));
    }
    tags.len() >= 2 && normalized.len() == 1
}

/// Number of source lines a member's body spans, inclusive of both ends.
fn line_span(member: &DuplicateMember) -> usize {
    member.end_line.saturating_sub(member.start_line) + 1
}

/// True when a group is low-value noise rather than actionable duplication:
/// every member's body is `TRIVIAL_BODY_LINE_SPAN` lines or fewer (a getter,
/// a one-`assert_eq!` test, a dispatch stub) and the group doesn't repeat
/// often enough (`TRIVIAL_MIN_REPEAT`) to suggest deliberate copy-paste
/// worth extracting. `MIN_DUPLICATE_NODES` alone doesn't catch these: a
/// one-line accessor can clear the node-count floor purely from type
/// annotations and field-access chains while still being a single line of
/// source — and post-refactor, unavoidable per-type accessor glue produced
/// *by* fixing real duplication shows up as new groups here, incorrectly
/// reading as unresolved debt if not filtered.
fn is_trivial_noise_group(group: &[DuplicateMember]) -> bool {
    group.len() < TRIVIAL_MIN_REPEAT && group.iter().all(|m| line_span(m) <= TRIVIAL_BODY_LINE_SPAN)
}

/// Bodies longer than this (in characters) skip the Levenshtein pass —
/// O(n*m) edit distance on two multi-page functions isn't worth the wait,
/// and shape-hash matches on bodies this large are rare anyway.
pub const MAX_DIFF_CHARS: usize = 20_000;

/// Percentage textual difference between two function bodies: 0 means
/// byte-for-byte identical (a true Type-1 clone); higher values mean more
/// divergence past the shared AST shape (renamed identifiers, changed
/// literals, small edits — Type-2 or a shape coincidence). `None` when
/// either body exceeds `MAX_DIFF_CHARS` — known non-identical (the caller
/// already checked equality first) but not worth computing an exact number
/// for.
pub fn byte_diff_percent(a: &str, b: &str) -> Option<u32> {
    if a == b {
        // Two empty strings are also caught here, so the size check below
        // never needs to special-case a zero max length.
        return Some(0);
    }
    let max_len = char_count(a).max(char_count(b));
    (max_len <= MAX_DIFF_CHARS).then(|| diff_percent(levenshtein_distance(a, b), max_len))
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

fn diff_percent(distance: usize, max_len: usize) -> u32 {
    ((distance as f64 / max_len as f64) * 100.0).round() as u32
}

/// Classic two-row edit-distance DP, O(n*m) time and O(min(n,m)) space.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a = to_chars(a);
    let b = to_chars(b);
    let mut prev = identity_row(b.len());
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        fill_edit_row(&prev, &mut curr, ca, &b);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn to_chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn identity_row(len: usize) -> Vec<usize> {
    (0..=len).collect()
}

fn fill_edit_row(prev: &[usize], curr: &mut [usize], ca: char, b: &[char]) {
    for (j, &cb) in b.iter().enumerate() {
        let cost = edit_cost(prev, curr, j, ca, cb);
        curr[j + 1] = cost;
    }
}

fn edit_cost(prev: &[usize], curr: &[usize], j: usize, ca: char, cb: char) -> usize {
    let substitution = usize::from(ca != cb);
    (prev[j + 1] + 1)
        .min(curr[j] + 1)
        .min(prev[j] + substitution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lang_parsing_substrate::Fingerprint;

    fn fp(hash: u64, node_count: usize, name: &str) -> Fingerprint {
        Fingerprint {
            name: Some(name.to_string()),
            kind: "function_definition",
            hash,
            node_count,
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 2,
        }
    }

    const NO_FILTERS: DuplicateFilters = DuplicateFilters {
        exclude_fixture_pairs: false,
        exclude_trivial: false,
    };

    const FIXTURE_PAIRS_ONLY: DuplicateFilters = DuplicateFilters {
        exclude_fixture_pairs: true,
        exclude_trivial: false,
    };

    const TRIVIAL_ONLY: DuplicateFilters = DuplicateFilters {
        exclude_fixture_pairs: false,
        exclude_trivial: true,
    };

    #[test]
    fn groups_matching_hashes_across_files() {
        let fingerprints = vec![
            CorpusFingerprint {
                source: "a.c".to_string(),
                fingerprint: fp(1, 10, "f"),
            },
            CorpusFingerprint {
                source: "b.c".to_string(),
                fingerprint: fp(1, 10, "g"),
            },
            CorpusFingerprint {
                source: "c.c".to_string(),
                fingerprint: fp(2, 10, "h"),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, NO_FILTERS);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].len(), 2);
        assert_eq!(result.excluded_fixture_pairs, 0);
    }

    #[test]
    fn larger_groups_sort_first() {
        let fingerprints = vec![
            CorpusFingerprint {
                source: "a.c".to_string(),
                fingerprint: fp(1, 10, "f"),
            },
            CorpusFingerprint {
                source: "b.c".to_string(),
                fingerprint: fp(1, 10, "g"),
            },
            CorpusFingerprint {
                source: "c.c".to_string(),
                fingerprint: fp(2, 10, "h"),
            },
            CorpusFingerprint {
                source: "d.c".to_string(),
                fingerprint: fp(2, 10, "i"),
            },
            CorpusFingerprint {
                source: "e.c".to_string(),
                fingerprint: fp(2, 10, "j"),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, NO_FILTERS);
        assert_eq!(result.groups.len(), 2);
        assert_eq!(result.groups[0].len(), 3);
        assert_eq!(result.groups[1].len(), 2);
    }

    fn fp_at(hash: u64, node_count: usize, name: &str, start_line: usize) -> Fingerprint {
        Fingerprint {
            start_line,
            end_line: start_line + 1,
            ..fp(hash, node_count, name)
        }
    }

    #[test]
    fn excludes_pass_fail_fixture_pairs_when_requested() {
        let fingerprints = vec![
            CorpusFingerprint {
                source: "tests/pass/CWE-190.c".to_string(),
                fingerprint: fp(1, 30, "main"),
            },
            CorpusFingerprint {
                source: "tests/fail/CWE-190.c".to_string(),
                fingerprint: fp(1, 30, "main"),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, FIXTURE_PAIRS_ONLY);
        assert_eq!(result.groups.len(), 0);
        assert_eq!(result.excluded_fixture_pairs, 1);

        let kept = find_duplicate_groups(&fingerprints, NO_FILTERS);
        assert_eq!(kept.groups.len(), 1);
        assert_eq!(kept.excluded_fixture_pairs, 0);
    }

    #[test]
    fn keeps_real_duplicates_within_the_same_fixture_dir() {
        // Two genuinely duplicated helpers both under tests/pass/ — not a
        // pass/fail pair, so this should never be auto-excluded.
        let fingerprints = vec![
            CorpusFingerprint {
                source: "tests/pass/a.c".to_string(),
                fingerprint: fp_at(1, 30, "helper", 1),
            },
            CorpusFingerprint {
                source: "tests/pass/b.c".to_string(),
                fingerprint: fp_at(1, 30, "helper", 1),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, FIXTURE_PAIRS_ONLY);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.excluded_fixture_pairs, 0);
    }

    fn fp_span(hash: u64, node_count: usize, name: &str, span_lines: usize) -> Fingerprint {
        Fingerprint {
            start_line: 1,
            end_line: span_lines,
            ..fp(hash, node_count, name)
        }
    }

    #[test]
    fn excludes_small_low_repeat_groups_when_requested() {
        // A one-line getter duplicated across only 2 files — classic
        // post-refactor accessor-boilerplate noise, not actionable debt.
        let fingerprints = vec![
            CorpusFingerprint {
                source: "a.rs".to_string(),
                fingerprint: fp_span(1, 25, "src", 1),
            },
            CorpusFingerprint {
                source: "b.rs".to_string(),
                fingerprint: fp_span(1, 25, "src", 1),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, TRIVIAL_ONLY);
        assert_eq!(result.groups.len(), 0);
        assert_eq!(result.excluded_trivial, 1);

        let kept = find_duplicate_groups(&fingerprints, NO_FILTERS);
        assert_eq!(kept.groups.len(), 1);
        assert_eq!(kept.excluded_trivial, 0);
    }

    #[test]
    fn keeps_small_groups_that_repeat_at_or_above_the_threshold() {
        // Same one-line getter, but repeated 4x — treated as deliberate
        // copy-paste worth flagging even though each body is tiny.
        let fingerprints = (0..4)
            .map(|i| CorpusFingerprint {
                source: format!("{i}.rs"),
                fingerprint: fp_span(1, 25, "src", 1),
            })
            .collect::<Vec<_>>();
        let result = find_duplicate_groups(&fingerprints, TRIVIAL_ONLY);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.excluded_trivial, 0);
    }

    #[test]
    fn keeps_small_repeat_groups_with_a_larger_body() {
        // Two matches, but each body is longer than the trivial line-span
        // floor — a real (if small) duplicate, not boilerplate.
        let fingerprints = vec![
            CorpusFingerprint {
                source: "a.rs".to_string(),
                fingerprint: fp_span(1, 25, "helper", 6),
            },
            CorpusFingerprint {
                source: "b.rs".to_string(),
                fingerprint: fp_span(1, 25, "helper", 6),
            },
        ];
        let result = find_duplicate_groups(&fingerprints, TRIVIAL_ONLY);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.excluded_trivial, 0);
    }

    #[test]
    fn byte_diff_percent_is_zero_for_identical_bodies() {
        assert_eq!(byte_diff_percent("fn f() {}", "fn f() {}"), Some(0));
    }

    #[test]
    fn byte_diff_percent_is_nonzero_for_renamed_identifiers() {
        let pct = byte_diff_percent("fn f(x: i32) -> i32 { x }", "fn g(y: i32) -> i32 { y }")
            .expect("small bodies always diff");
        assert!(pct > 0 && pct < 100, "expected a partial diff, got {pct}");
    }

    #[test]
    fn byte_diff_percent_is_high_for_wholly_different_bodies() {
        let pct = byte_diff_percent("a", "completely different text").unwrap();
        assert!(pct > 50, "expected a large diff, got {pct}");
    }

    #[test]
    fn byte_diff_percent_skips_oversized_bodies() {
        let a = "x".repeat(MAX_DIFF_CHARS + 1);
        let b = "y".repeat(MAX_DIFF_CHARS + 1);
        assert_eq!(byte_diff_percent(&a, &b), None);
    }

    #[test]
    fn byte_diff_percent_still_reports_zero_for_oversized_identical_bodies() {
        // Equality is checked before the size cap, so identical huge bodies
        // (e.g. two copies of a generated file) still short-circuit to 0.
        let a = "x".repeat(MAX_DIFF_CHARS + 1);
        assert_eq!(byte_diff_percent(&a, &a), Some(0));
    }
}

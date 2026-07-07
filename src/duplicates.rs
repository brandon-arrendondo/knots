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

/// Functions smaller than this (in AST node count) are skipped — a
/// single-line getter hashing the same as another single-line getter isn't
/// a meaningful clone, just noise.
pub const MIN_DUPLICATE_NODES: usize = 20;

/// One member of a duplicate group, detached from the borrowed fingerprint
/// data so callers don't need to keep the corpus fingerprint list alive.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMember {
    pub file_path: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub node_count: usize,
}

/// Groups corpus fingerprints into duplicate groups (2+ members sharing an
/// identical structural hash), sorted largest-group-first, then by node
/// count — the clones most worth acting on come first.
pub fn find_duplicate_groups(
    fingerprints: &[CorpusFingerprint<String>],
) -> Vec<Vec<DuplicateMember>> {
    let mut groups: Vec<Vec<DuplicateMember>> = duplicate_groups(fingerprints)
        .into_iter()
        .map(|members| {
            members
                .iter()
                .map(|m| DuplicateMember {
                    file_path: m.source.clone(),
                    name: m.fingerprint.name.clone(),
                    start_line: m.fingerprint.start_line,
                    end_line: m.fingerprint.end_line,
                    node_count: m.fingerprint.node_count,
                })
                .collect()
        })
        .collect();
    groups.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then(b[0].node_count.cmp(&a[0].node_count))
    });
    groups
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
        let groups = find_duplicate_groups(&fingerprints);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
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
        let groups = find_duplicate_groups(&fingerprints);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 3);
        assert_eq!(groups[1].len(), 2);
    }
}

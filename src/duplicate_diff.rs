//! Before/after comparison for `--find-duplicates` reports, keyed by the
//! stable, content-derived group ID (`duplicates::group_id`) rather than
//! positional numbering — diffing "group #1 vs group #1" across two runs
//! isn't meaningful once files are added, removed, or reordered, but "group
//! `a1b2c3d4` had 3 members, now has 2" is.
//!
//! This module is pure data transformation (snapshot + diff); the CLI layer
//! (`main.rs`) owns reading/writing the JSON files and printing the result.

use crate::duplicates::{group_id, DuplicateGroupsResult, DuplicateMember};
use serde::{Deserialize, Serialize};

/// A JSON-serializable snapshot of one `--find-duplicates` run, produced by
/// `--dump-duplicates` and consumed by `--diff-duplicates`.
#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateSnapshot {
    pub groups: Vec<GroupSnapshot>,
}

/// One group's identity and shape at snapshot time — enough to tell whether
/// a later run's matching group (by `id`) shrank, grew, or vanished,
/// without needing to re-read the original source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSnapshot {
    pub id: String,
    pub member_count: usize,
    pub node_count: usize,
    pub members: Vec<String>,
}

/// Captures the current state of every reported group for later diffing.
pub fn snapshot(result: &DuplicateGroupsResult) -> DuplicateSnapshot {
    DuplicateSnapshot {
        groups: result
            .groups
            .iter()
            .map(|group| group_snapshot(group))
            .collect(),
    }
}

fn group_snapshot(group: &[DuplicateMember]) -> GroupSnapshot {
    GroupSnapshot {
        id: group_id(group),
        member_count: group.len(),
        node_count: group[0].node_count,
        members: group.iter().map(member_label).collect(),
    }
}

fn member_label(member: &DuplicateMember) -> String {
    let name = member.name.as_deref().unwrap_or("<anonymous>");
    format!(
        "{}:{}-{} {}",
        member.file_path, member.start_line, member.end_line, name
    )
}

/// The result of comparing two snapshots by group ID.
#[derive(Debug, Default)]
pub struct DuplicateDiff {
    /// Present before, absent after — fully resolved.
    pub resolved: Vec<GroupSnapshot>,
    /// Absent before, present after — either newly introduced duplication
    /// or (commonly, per MOLDY_DUP_FEEDBACK.md) irreducible glue a fix
    /// produced; see the interpretation guidance in architecture.rst.
    pub new_groups: Vec<GroupSnapshot>,
    /// Same ID both sides, but with a different member count.
    pub changed: Vec<GroupChange>,
    /// Same ID both sides, same member count — not interesting enough to
    /// list individually, just counted.
    pub unchanged_count: usize,
}

#[derive(Debug)]
pub struct GroupChange {
    pub before: GroupSnapshot,
    pub after: GroupSnapshot,
}

impl GroupChange {
    pub fn shrank(&self) -> bool {
        self.after.member_count < self.before.member_count
    }
}

/// Matches groups between two snapshots by their stable ID and classifies
/// each as resolved, new, changed (shrank/grew), or unchanged.
pub fn diff_snapshots(before: &DuplicateSnapshot, after: &DuplicateSnapshot) -> DuplicateDiff {
    let mut diff = DuplicateDiff::default();
    for before_group in &before.groups {
        match find_by_id(&after.groups, &before_group.id) {
            None => diff.resolved.push(before_group.clone()),
            Some(after_group) => classify_match(&mut diff, before_group, after_group),
        }
    }
    for after_group in &after.groups {
        if find_by_id(&before.groups, &after_group.id).is_none() {
            diff.new_groups.push(after_group.clone());
        }
    }
    diff
}

fn find_by_id<'a>(groups: &'a [GroupSnapshot], id: &str) -> Option<&'a GroupSnapshot> {
    groups.iter().find(|g| g.id == id)
}

fn classify_match(diff: &mut DuplicateDiff, before: &GroupSnapshot, after: &GroupSnapshot) {
    if before.member_count == after.member_count {
        diff.unchanged_count += 1;
    } else {
        diff.changed.push(GroupChange {
            before: before.clone(),
            after: after.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duplicates::{find_duplicate_groups, DuplicateFilters};
    use lang_parsing_substrate::{CorpusFingerprint, Fingerprint};

    const NO_FILTERS: DuplicateFilters = DuplicateFilters {
        exclude_fixture_pairs: false,
        exclude_trivial: false,
    };

    fn fp(hash: u64, name: &str) -> Fingerprint {
        Fingerprint {
            name: Some(name.to_string()),
            kind: "function_definition",
            hash,
            node_count: 30,
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 2,
        }
    }

    fn fingerprints(entries: &[(&str, u64, &str)]) -> Vec<CorpusFingerprint<String>> {
        entries
            .iter()
            .map(|&(source, hash, name)| CorpusFingerprint {
                source: source.to_string(),
                fingerprint: fp(hash, name),
            })
            .collect()
    }

    fn snap(entries: &[(&str, u64, &str)]) -> DuplicateSnapshot {
        snapshot(&find_duplicate_groups(&fingerprints(entries), NO_FILTERS))
    }

    #[test]
    fn detects_a_resolved_group() {
        let before = snap(&[("a.c", 1, "f"), ("b.c", 1, "g")]);
        let after = snap(&[]);
        let diff = diff_snapshots(&before, &after);
        assert_eq!(diff.resolved.len(), 1);
        assert!(diff.new_groups.is_empty());
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn detects_a_new_group() {
        let before = snap(&[]);
        let after = snap(&[("a.c", 1, "f"), ("b.c", 1, "g")]);
        let diff = diff_snapshots(&before, &after);
        assert_eq!(diff.new_groups.len(), 1);
        assert!(diff.resolved.is_empty());
    }

    #[test]
    fn detects_a_shrunk_group_by_stable_id_despite_membership_change() {
        let before = snap(&[("a.c", 1, "f"), ("b.c", 1, "g"), ("c.c", 1, "h")]);
        // "b.c" refactored away; same shape hash, fewer members.
        let after = snap(&[("a.c", 1, "f"), ("c.c", 1, "h")]);
        let diff = diff_snapshots(&before, &after);
        assert!(diff.resolved.is_empty());
        assert!(diff.new_groups.is_empty());
        assert_eq!(diff.changed.len(), 1);
        assert!(diff.changed[0].shrank());
        assert_eq!(diff.changed[0].before.member_count, 3);
        assert_eq!(diff.changed[0].after.member_count, 2);
    }

    #[test]
    fn counts_an_untouched_group_as_unchanged() {
        let before = snap(&[("a.c", 1, "f"), ("b.c", 1, "g")]);
        let after = snap(&[("a.c", 1, "f"), ("b.c", 1, "g")]);
        let diff = diff_snapshots(&before, &after);
        assert_eq!(diff.unchanged_count, 1);
        assert!(diff.resolved.is_empty());
        assert!(diff.new_groups.is_empty());
        assert!(diff.changed.is_empty());
    }
}

# ADR-0004: The changelog tells users what changed; it is not a git log or a task list

**Status:** Proposed, 2026-10-08. Awaiting review and acceptance.

## Context

A knots user needs to know what a release adds, removes or changes, especially
when existing scores change meaning. A list of commits or completed work mixes
those changes with implementation and measurement history. The validation
record already holds comparison evidence; it is not a release note.

Knots has no `CHANGELOG.md` today. The latest GitHub release bodies
(v1.16.0–v1.18.0) contain only version-compare links rather than curated
release notes. ADR-0001 refers to a changelog; this decision establishes it.

[ADR-0001](0001-the-definition-is-the-authority.md) requires counting changes to
be measured and listed under Changed. This decision keeps that requirement
while defining the changelog's wider purpose.

## Decision

Create `CHANGELOG.md` starting with the next release as a curated record for
users of knots. Each release has one dated section, newest first, with
Unreleased on top. Draw the GitHub release body from that release's section;
a compare link may supplement the notes but does not replace them. Write the
entry in the same change that ships the user-visible effect, under Unreleased.
Use these headings only when they have entries:

1. **Added:** a new metric, language, CLI option, output format or analysis
   capability. Describe what the user can now do.
2. **Fixed:** a crash, incorrect output, nondeterminism or other tool defect.
   Describe the failing construct and resulting behavior in user terms.
3. **Removed:** a metric, option, output field, platform or supported behavior
   a user could have relied on. Even small removals must be recorded.
4. **Changed:** a change to existing output's meaning or an option's behavior,
   including changed defaults. It is not a category for internal work.

**A counting-rule change is listed under Changed, as ADR-0001 requires, even
when it corrects a bug.** Explain which metric and constructs change, so users
know to compare scores and reconsider baselines. Describe the correction in
that entry rather than duplicating it under Fixed.

Entries are short publication-ready explanations, not copied commit subjects
or work-item titles. Exclude measurement and corpus work, paper and docs-only
edits, CI and packaging chores, refactors, tests or fixtures added for their
own sake, and dependency changes with no user-visible effect. If any of those
ships a user-visible capability or fix, describe that effect instead.

Do not publish internal tracking references or locate defects in another
project that have not been fixed upstream. The changelog is part of the public
record and will ship in release packages.

## Consequences

- Release notes require editorial review; automation can collect candidates
  but cannot decide that every completed change deserves an entry.
- Group related effects so a release remains readable. Absence of internal
  work from the changelog is correct, not missing attribution.
- Backfilling older releases is a separate maintainer decision. Git history
  remains the implementation record; this ADR authorizes no history rewrite
  or replacement of already-published archives.
- Deprecated and Security headings are not used. A security fix is a Fixed
  entry, published only once fixed; if it involves another project, wait
  until the fix has landed upstream.
- Validation results stay in the pinned validation record. A counting-change
  entry points readers to that evidence without turning the changelog into
  a benchmark report.

Origin: port of aurora-lint ADR-0009, “The changelog tells users what changed;
it is not a git log or a task list”, restated for knots and its ADR-0001.

Source record: [aurora-lint ADR](https://github.com/brandon-arrendondo/aurora-lint/blob/8ad31a842e9d02f97a6d4b3a8cbeef5a9264cf0b/docs/adr/0009-changelog-is-for-users-not-a-git-log.md).

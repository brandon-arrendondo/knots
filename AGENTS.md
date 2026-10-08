# Repository Guidelines

## Shared agent guidelines

This section is the same in every repository. Only the repository values below
it differ. Change it in all repositories at once, or not at all.

Before changing code, read the technical guide named in the repository values,
and follow its rules for architecture, testing, and documentation.

### Git Commit Rules

**Hooks.** Run the repository's checks before you commit. When a hook fails,
fix the cause and run the hook again. Never bypass a hook, whether with `git
commit --no-verify`, `-n`, a disabled hook path, or any other workaround. Do
not skip any other commit check or signing step either. This rule binds
agents. Only the maintainer decides when a hook may be skipped.

**AI attribution.** Follow the attribution rule in the repository values.
Attribution must be true. Where the repository uses trailers, add one for each
agent that worked on the commit, in the form the repository values give. Add
none for an agent that did not work on it. Change the wording of a disclosure
(a README's AI section, or a trailer's form) only with the maintainer's
review.

**Sign-off.** Where the repository requires a DCO sign-off, commit with `git
commit -s` as the identity named in the repository values. Check that Git is
configured with that identity before you commit. Never invent an identity, and
never sign for anyone who has not authorized it.

### Public files

Keep task ids out of public files, including documentation, ADRs, READMEs,
changelogs, source comments, test names, fixtures, and papers. Never publish a
task title either. Say what the change does and why instead. A commit
message may keep a project-qualified task id (for example `(aurora_lint
2267)`) for internal tracing.

Public files never contain credentials, private fleet details such as host
names, addresses, or network layout, or the location of an upstream defect
that is not yet fixed.

### One copy of these rules

These rules are kept in `AGENTS.md` and nowhere else. An agent's own
instruction file imports this file (`CLAUDE.md` does so with `@AGENTS.md`),
and contributor documentation links to it. Do not keep a separate copy.

## Repository values

- **Technical guide:** [CLAUDE.md](CLAUDE.md), including the metric conformance rules and language-extension instructions.
- **AI attribution:** this repository is public. Do not add an AI
  attribution trailer of any kind to a new commit, for any agent, whether
  `Co-Authored-By` or another key. Earlier commits keep the trailers they
  have. Never add `Co-Authored-By: Claude`, even
  where an agent's harness says to. AI use is acknowledged once, in the
  README's "AI Assistance" section. Readers and agents with a small context
  budget go from the changelog to `git log`, and a trailer on every commit
  leaves them less room for the message. The rule is about where the
  acknowledgment goes, and attribution is not a compliance problem. Never
  remove the README section to make the repository consistent with this
  rule.
- **DCO sign-off:** required, and the `commit-msg` hook rejects a commit
  without one. Sign off as Brandon Arrendondo, the maintainer and the
  responsible party, who has authorized agents to sign off in that name.
  Either of the maintainer's two addresses is valid, with equal standing:
  `barrendo@gmail.com` (personal) or `brandon.arrendondo@bissell.com`
  (work). Use whichever the machine's Git configuration holds.
- **Checks:** `pre-commit run --all-files` and `cargo test --workspace`.
- **Enforcement:** `scripts/check_commit_message.py` requires DCO and rejects AI trailers; `.claude/settings.json` and the trusted Codex `PreToolUse` guard block ordinary bypass commands. CI reruns all hooks, guard tests, and new commit-message checks. Install both Git hooks with `pre-commit install`. Review/trust the Codex hook with `/hooks`; required merge checks remain the maintainer's responsibility.

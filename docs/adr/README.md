# Architecture Decision Records

These records explain the decisions behind knots' metrics and gates. Read them
alongside the [metrics reference](../metrics-reference.rst),
[architecture](../architecture.rst) and [validation record](../../validation/README.md).

| Record | Status | Decision |
|---|---|---|
| [0001](0001-the-definition-is-the-authority.md) | Accepted | A metric's published definition is the authority, not another tool. |
| [0002](0002-a-name-is-not-a-function.md) | Accepted | Resolve names to declarations and score real configurations. |
| [0003](0003-gate-defaults-have-a-recorded-basis.md) | Accepted | Gate defaults have a recorded source or calibration. |
| [0004](0004-changelog-is-for-users.md) | Proposed | Keep the changelog focused on user-visible changes. |
| [0005](0005-incomplete-scans-never-pass-as-clean.md) | Proposed | Report incomplete scans and contain failed work; implementation pending. |

Contributor instructions are in [AGENTS.md](../../AGENTS.md); the technical guide
is [CLAUDE.md](../../CLAUDE.md). These records document decisions rather than
copying contributor rules.

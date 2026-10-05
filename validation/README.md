# Validation

Scripts that produce the agreement figures knots publishes, so each figure
can be reproduced and re-checked whenever a counting rule changes
(`docs/adr/0001-the-definition-is-the-authority.md`).

## Corpora (pinned)

The six C calibration corpora, at the commits the AIRD paper and the
aurora-lint oracle use:

| Corpus | Commit |
|--------|--------|
| lua | `40b76de2d77e` |
| libcrc | `7719e2112a9a` |
| mosquitto | `d3ee5c5ca62c` |
| hostap | `dcee60436390` |
| sqlite | `b1a73ba34d05` |
| curl | `3e198f75861c` |

Check out each one detached at its commit before running. A drifted tree
silently changes every figure.

## Scripts

- `pmccabe_compare.py KNOTS_BIN OUT.json CORPUS...` compares knots' McCabe
  with pmccabe's "modified" count, function by function. It tags each
  mismatch with the constructs present (`goto`, `throw`, `#if`, ternary).
- `compare_versions.py OLD_BIN NEW_BIN OUT.json CORPUS...` shows which
  metrics moved between two knots builds, AIRD deltas, band changes, and
  crossings of the 85 gate.

## Results recorded 2026-10-05

**pmccabe agreement, C files (30,461 matched functions):**

| knots | Equal | Differ | Agreement |
|-------|-------|--------|-----------|
| 1.18.0 (`de0c562`, counts `goto`) | 28,285 | 2,176 | 92.9% |
| branch `rigor-definitions-probes` (no `goto`/`throw`/`raise`) | 29,753 | 708 | 97.7% |

In 858 of the 861 functions whose only extra construct was `goto`, knots'
excess equaled the function's `goto` count exactly. After the change, 702 of
the 708 differing functions contain `#if` arms. knots counts decisions in
every arm and pmccabe keeps the first.

**Effect of the change on 33,000 functions (1.18.0 → branch):**
- Cognitive Complexity: unchanged.
- McCabe: 1,711 functions changed.
- Test score: 932 changed.
- AIRD: 819 changed, each by −1 to −3. 58 functions moved down one band; none
  crossed the 85 gate.

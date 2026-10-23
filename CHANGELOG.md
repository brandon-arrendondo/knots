# Changelog

What changed for users of knots, newest release first. Each release lists only
the headings that have entries: Added, Fixed, Removed and Changed. A change to a
counting rule is listed under Changed, even when it corrects a bug, because
existing scores and gate results can move. See
[ADR-0004](docs/adr/0004-changelog-is-for-users.md).

## Unreleased

### Fixed

- Files with upper- or mixed-case extensions, such as `A.C` and `B.H` from
  Windows-built source trees, are analysed: `-r` finds them, `--language c`
  keeps them, and naming one is accepted. `.C` is C, not C++.
- An `#include` that spells a known extension in upper case, such as
  `#include "B.H"`, now resolves to the file it names, so it counts toward
  Ce, Ca and AIRD's coupling factor instead of being dropped.

## 1.19.0 - 2026-10-19

### Changed

- McCabe counts a C, C++ or PHP `switch` as one decision per non-default case.
  `goto`, `throw` and `raise` are no longer decision points.
- Cognitive complexity adds +1 for a recursion cycle, and `throw` and `raise`
  no longer increment it, as the specification defines.
- A C or C++ function is scored as its worst real preprocessor configuration.
- SLOC reads comments from the parse instead of scanning the text.
- The ABC gate default is 20.0 in the self-check and in `knots-strict`.

Scores and gate results can change in this release. Compare against a fresh
baseline before acting on a difference.

### Fixed

- JavaScript, TypeScript, Rust and Ada function definitions that were missed or
  scoped to the wrong function are now found.
- An exclude that names only functions no longer stops file discovery.

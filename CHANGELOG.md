# Changelog

What changed for users of knots, newest release first. Each release lists only
the headings that have entries: Added, Fixed, Removed and Changed. A change to a
counting rule is listed under Changed, even when it corrects a bug, because
existing scores and gate results can move. See
[ADR-0004](docs/adr/0004-changelog-is-for-users.md).

## Unreleased

### Added

- `--completion-report FILE` writes what a scan analysed, and every selected
  file it could not analyse and why, to `FILE` as JSON. It works with any
  `--format`, whose own output is unchanged.

### Fixed

- Files with upper- or mixed-case extensions, such as `A.C` and `B.H` from
  Windows-built source trees, are analysed: `-r` finds them, `--language c`
  keeps them, and naming one is accepted. `.C` is C, not C++.
- An `#include` that spells a known extension in upper case, such as
  `#include "B.H"`, now resolves to the file it names, so it counts toward
  Ce, Ca and AIRD's coupling factor instead of being dropped.
- A threshold now gates the run with `--format json`, `ndjson`, `csv` or
  `sarif` as it does with text output: a violation exits 1 and is listed on
  stderr, after the full document is written. Previously a structured format
  ignored every threshold and exited 0.

### Changed

- A scan that could not analyse a selected file now exits 3, whether or not
  a threshold is set, instead of 0 or 1. A file is not analysed when it
  cannot be read or is not UTF-8, no grammar can parse it, its analysis
  crashes, it is named with an extension knots does not support (such as
  `a.txt`, or a fixed-form Fortran `.f`), or it is named or listed in
  `compile_commands.json` but is not on disk. A named path that does not
  exist used to exit 1. Each such file is named on stderr. Files that
  `-l` or an exclude leaves out do not count; exclude a file to pass a scan
  that cannot read it. A file that parses only with syntax errors and
  yields no function is reported, but is not counted as not analysed.
- The text summary always prints the selected, analysed and not-analysed
  file counts, and the count of files with syntax errors and no functions,
  including zeros. Its "Skipped (encoding/parse errors)" line is gone.
- SARIF output carries `invocations`: `executionSuccessful` is false when a
  selected file was not analysed, with one notification per such file.
- `--write-baseline` refuses a scan that could not analyse a selected file:
  it leaves the baseline file unchanged and exits 3.
- In recursive mode, a file that could not be analysed still counts as the
  target of an `#include` or import, so the files that include it keep
  their Ce and AIRD coupling factor.

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

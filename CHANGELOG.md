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
  `a.txt`, or a fixed-form Fortran `.f`), `compile_commands.json` lists it
  but it is not on disk, or a recursive scan cannot reach it (a directory
  knots cannot read, or a dangling symlink). Each such file is named on
  stderr. Files that `-l`, `--exclude-path`, an include or exclude file, or
  a whole-file `knots.toml` exclude leaves out do not count and are never
  read; exclude a file to pass a scan that cannot read it. A file that
  parses only with syntax errors and yields no function is reported, but is
  not counted as not analysed.
- A path named on the command line that does not exist now exits 2, as an
  invocation error, instead of 1.
- The text summary's FILES PROCESSED block is renamed and always complete:
  "Total files found" is now "Total files selected", "Successfully
  processed" is now "Analysed", and the "Skipped (encoding/parse errors)"
  line, printed only when nonzero, is replaced by "Not analysed" and
  "Syntax errors, no functions recovered", printed even when zero.
  Single-file runs print a one-line form of the same counts.
- SARIF output carries `invocations`: `executionSuccessful` is false when a
  selected file was not analysed, `exitCode` is the run's exit status, and
  there is one notification per such file.
- `--write-baseline` refuses a scan that could not analyse a selected file:
  it leaves the baseline file as it was, or does not create it, and exits 3.
- In recursive mode, a file that could not be analysed still counts as the
  target of an `#include` or import, and still makes a stem it shares with
  another file ambiguous. In a scan that loses a file, the Ce and AIRD
  coupling factor of the files around it can therefore rise or fall
  compared with the previous release, which left the lost file out of the
  import graph. They now match what a complete scan gives.

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

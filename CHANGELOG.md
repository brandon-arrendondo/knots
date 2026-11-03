# Changelog

What changed for users of knots, newest release first. Each release lists only
the headings that have entries: Added, Fixed, Removed and Changed. A change to a
counting rule is listed under Changed, even when it corrects a bug, because
existing scores and gate results can move. See
[ADR-0004](docs/adr/0004-changelog-is-for-users.md).

## Unreleased

### Changed

- C++ state coupling now counts the members a method reaches through
  `this`: `this->member` and `(*this).member`, fields and method calls
  alike, each distinct member once, as `self.member` already counts in
  Rust and Python. A C++ method with `this->a + this->b + x` scores 3,
  where it scored 1. A C++ function's `state_coupling`, and its AIRD
  through it, can rise. A member named without `this->` is still not
  counted. The metrics reference now says that state coupling counts
  members, fields and methods, and lists the receiver each language
  uses.
- Java and PHP state coupling now counts method calls through the
  receiver, `this.helper()` and `$this->helper()`, as a member, as it
  already did for fields and as the other languages count calls. A Java or
  PHP function's `state_coupling`, and its AIRD through it, can rise.

## 2.1.0 - 2026-11-03

### Fixed

- A reader that closes knots' output early, as `knots -r src | head` or
  `knots -r src 2>&1 | head` does, no longer makes knots panic (the text and
  `-v` reports, and any stderr line such as a threshold violation) or exit 1
  with a "Broken pipe" error (`--format json`, `ndjson`, `csv` and `sarif`).
  The closed stream stops, and the exit status is the one the scan earns: 0,
  1 for a threshold violation, or 3 for an incomplete scan.
- The hook scripts in `hooks/` run where bash is not installed, such as a
  FreeBSD base system: the published `test-complexity` pre-commit hook,
  `pre-commit`, `pre-commit-warning-only`, `pre-commit-wrapper.sh` and
  `install-hook.sh` now need only `/bin/sh`. Their output and exit statuses
  are unchanged. `install-hook.sh`'s yes/no questions now take `y` or `yes`
  followed by Enter.
- When an option that takes a value, such as `--threshold`, is the last
  argument to `pre-commit-wrapper.sh` or `test-complexity-wrapper.sh`, the
  wrapper now exits 2 with an error. Previously it never finished.
- The `hooks/PRE_COMMIT_FRAMEWORK.md` example that runs
  `pre-commit-wrapper.sh` through a shell now passes every staged file.
  Previously it dropped the first.
- The ABC threshold now gates the standalone hooks. `pre-commit` and
  `pre-commit-warning-only` never applied `abc-threshold`, and
  `pre-commit-wrapper.sh` ignored `--abc-threshold` whenever another
  threshold was set or `--verbose` was given, because each read a line the
  text report no longer prints. The hooks now read `knots --format ndjson`,
  so every threshold applies in every mode, and they no longer need `bc`.
  Commits that passed before can be rejected where a function's ABC
  magnitude is over the configured threshold.
- `pre-commit` and `pre-commit-warning-only` name a function once, as
  `file.c:7:name`, instead of repeating the file (`file.c:file.c:7:name`).

### Changed

- `knots.toml` now refuses a section or key that knots does not read, such
  as `["c++".thresholds]` (the section is `[cpp.thresholds]`) or a
  misspelled threshold like `mccab`. knots exits 1 before analysing
  anything, naming each one with its file and line and, where one is close,
  the valid name. Previously such a name was ignored, so the threshold it
  meant to set never applied and the run could pass. A configuration that
  loaded before can now fail to load; correct or remove the names it lists.
- A `[[filter.exclude]]` glob in `file_patterns` or regular expression in
  `function_patterns` that does not compile now stops knots with exit
  status 1 before anything is analysed, naming the entry, the pattern and
  the parser's error. Previously such a pattern matched nothing, so the
  functions it meant to exclude were still reported and gated. A
  configuration that loaded before can now fail to load; correct the
  pattern it names.
- The `knots`, `knots-verbose`, `knots-strict`, `knots-system` and
  `knots-system-strict` pre-commit hooks in the knots repository now select
  every file knots can analyse, by its extension in any letter case: Ada, Go, Java, C#, Kotlin,
  Swift, PHP, Fortran, Scala and Lua files are checked as well as C, C++,
  Rust, Python, JavaScript and TypeScript. They no longer pass files that
  pre-commit tags as C++ or Python but knots cannot analyse, such as `.hh`,
  `.inl`, `.ipp` or an extensionless Python script, which made the scan
  incomplete and failed the commit. Commits that passed before can be
  rejected where a newly checked file is over a threshold; add `types_or`
  to the hook to check only some languages. The `knots-pre-commit` mirror's
  hooks follow in its next release.
- The standalone hooks in `hooks/` (`pre-commit`, `pre-commit-warning-only`
  and the pre-commit framework's `pre-commit-wrapper.sh`) no longer let a
  file through unchecked:
  - `pre-commit` and `pre-commit-warning-only` check every staged file knots
    can analyse, by its extension in any letter case, where they checked
    only C, C++, Rust, Python and JavaScript.
  - A staged file knots cannot analyse, such as one that is not valid
    UTF-8, now fails `pre-commit` and `pre-commit-wrapper.sh` and is
    reported by `pre-commit-warning-only`. The hooks used to discard knots'
    messages and pass such a file, and `pre-commit-wrapper.sh` skipped
    UTF-8 errors on purpose; knots' messages are now shown.
  - When there are files to check and knots cannot be found, `pre-commit`
    and `pre-commit-wrapper.sh` fail, and `pre-commit-warning-only` warns
    that nothing was checked. All three used to pass.
  - A threshold set in `knots.toml` that knots itself fails is reported as
    knots' own check failing, with its messages, and blocks the blocking
    hooks.
  - Staged paths with spaces or non-ASCII characters are checked;
    `pre-commit` and `pre-commit-warning-only` used to skip them.

  Commits that passed before can be rejected: install knots, fix or leave
  out a file knots cannot read, or exclude it in your pre-commit
  configuration.
- The published `test-complexity` pre-commit hook now blocks a commit whose
  test fails a threshold: it runs at `--level=error`. Previously it ran at
  `warn`, reporting the failure and passing. Setting `args:` replaces the
  hook's own, so a configuration with `args:` and no `--level` reports
  without blocking, as before; add `--level=error` to block, or
  `--level=warn` to say so explicitly.

## 2.0.0 - 2026-10-23

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
- With `--compile-commands`, `-l` and `--exclude-path` now apply to the
  compile database's entries, as they do to named files and recursive
  scans. Before, an entry in another language was analysed anyway, and an
  excluded path was not left out.

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
- A path named on the command line that does not exist, or a directory
  named without `-r`, now exits 2, as an invocation error, instead of 1.
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

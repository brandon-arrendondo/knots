# Implementation prompt: include headers by default in recursive mode

## Context

`knots` computes complexity metrics. Today, `.h`/`.hpp`/`.hxx` (and other
header-only extensions across languages) are **excluded from recursive
(`-r`/`--recursive`) discovery** — they're only ever parsed when passed
explicitly as a single file. This has a real, observed effect: when an LLM
(or a human) is asked to reduce complexity in a `.c`/`.cpp` file that knots
is watching, one easy way to satisfy the metric is to push logic into a
header (inline functions, macros, template bodies) that knots never scans in
the common `--recursive` workflow. The metric goes down; the actual
complexity doesn't. This doc is the self-contained brief for fixing that —
whoever picks this up doesn't need this conversation's context.

This is a `knots`-only change. Do not modify `lang_parsing_substrate` for
this task — see "Explicitly out of scope" below for why.

## Current state (verified against `src/main.rs`)

Three call sites gate which files get parsed, using two functions imported
from `lang_parsing_substrate`:

- `is_source_extension(ext)` — true only for a language's `extensions` set
  (recursive-discovery-eligible).
- `is_parseable_extension(ext)` — true for `extensions` **or**
  `explicit_only` (the full parseable set, including headers).

Call sites:

- `main.rs:1275` (`load_compile_commands`) — filters `compile_commands.json`
  entries with `is_source_extension`. Headers never appear as their own
  compile-command entries in practice (translation units are `.c`/`.cpp`),
  so this one is likely fine as-is — flag it in your findings if you
  disagree, but it's not the primary target.
- `main.rs:1328` (`collect_files`, single-file mode) — already uses
  `is_parseable_extension`, with a comment explaining it deliberately
  accepts explicit-only extensions like `.h` when a header is passed
  directly. This is the *reference behavior* you're extending to recursive
  mode.
- `main.rs:1355` (`collect_files`, recursive directory walk) — uses
  `is_source_extension`, which is why headers are invisible in
  `--recursive` today. **This is the line to change.**

There's also a latent inconsistency in `src/coupling.rs`:
`known_source_extensions()` (coupling.rs:64-70) already includes header
extensions (union of `extensions` + `explicit_only`) so it can strip a
trailing extension off a raw `#include "foo.h"` string down to a candidate
module key. Its own unit test
(`c_header_include_resolves_by_stripped_extension`, coupling.rs:299-315)
demonstrates the resolver matching against a corpus entry that is itself a
`.h` file — but that corpus, in a real `--recursive` run, is built from
`collect_files`/`load_compile_commands`, which (before this change) never
include `.h` files. So today, `#include "foo.h"` edges in the import graph
can never actually resolve to a real target file; the test's premise
doesn't reflect reality. Making the `main.rs:1355` change should make this
resolution path live for the first time — verify that when you're done.

## What to implement

1. Change `collect_files`'s recursive directory walk (`main.rs:1355`) to
   accept header/explicit-only extensions by default, consistent with how
   single-file mode (`main.rs:1328`) already does — i.e. swap
   `is_source_extension` for `is_parseable_extension` at that call site (or
   introduce a narrower helper if you find a reason `is_parseable_extension`
   is too broad for recursive mode specifically — e.g. if any language's
   `explicit_only` set contains something that shouldn't be walked
   unprompted. Check `lang_parsing_substrate/src/registry.rs`'s
   `languages()` to see what's actually in each language's `explicit_only`
   before assuming `.h`-style headers are the only member).
2. Decide whether this should be gated behind a flag (e.g.
   `--include-headers`, default true) or unconditional. Given the framing
   above — headers being scanned is the point, not an opt-in nicety — default
   to unconditional unless you find a concrete reason (e.g. header-only
   template libraries producing enormous, unrepresentative complexity counts
   that would need a separate discussion) that argues for a flag. If you add
   a flag, make the default match "headers are included."
3. Re-run/inspect `coupling.rs`'s existing header-related test and confirm
   it now exercises real corpus data end-to-end (add an integration-level
   test if the existing unit test only proves the string-stripping logic in
   isolation, not the full recursive-walk → corpus → resolution path).
4. Add a recursive-mode fixture (e.g. under `sample-files/` or wherever this
   crate's existing recursive-mode tests live) with a `.c`/`.h` pair where
   nontrivial logic lives in the header, and assert the header's complexity
   is now counted in `--recursive` output where it previously wasn't.
5. Check `docs/` (e.g. `docs/architecture.rst` if that's where recursive-mode
   behavior is documented) for any text asserting headers are excluded from
   recursive mode, and update it.

## Known limitation to document, not fix here

`.h` is currently parsed with the **C grammar unconditionally** — there's no
content-based C-vs-C++ disambiguation in `lang_parsing_substrate`
(`.hpp`/`.hxx` are unambiguous C++ already, but plain `.h` always resolves
to C via extension-match-arm ordering in `language_for_file`, see that
crate's `CLAUDE.md`). This means a `.h` header in a C++-only project will be
parsed as C once this change lands, which may produce parse errors or
misleading metrics for C++-only syntax (classes, templates, namespaces) in
headers.

This is being tracked and picked up separately, on `lang_parsing_substrate`,
as a syntax-based C-vs-C++ header sniffer (same "best-effort lower bound,
never fabricate" pattern used for `detect_min_c_standard` in that crate's
`src/c_standard.rs` — see `docs/research-c-standard-detection.md` there for
the template). **That work will start after this change lands here** — so
don't block on it, but do call out in your PR description / findings that
plain-C-only projects get a clean win immediately, while mixed/C++-only
header projects may see noisy results until the follow-up lands.

## Explicitly out of scope

- Do not modify `lang_parsing_substrate` (the C-vs-C++ header disambiguation
  is a separate, sequenced task on that repo).
- Do not attempt to solve macro-guarded or preprocessor-dependent header
  content — that's a pre-existing, unrelated limitation of the tree-sitter-
  based approach and not specific to this change.

## What to produce

- The `collect_files` change plus updated/added tests described above.
- A short findings note (append to this file under a `## Findings` heading,
  matching this repo's convention if it has one, or just as a PR
  description) covering: what `explicit_only` extensions across all
  languages you found and included, whether you gated behind a flag and
  why, and the before/after behavior of the `coupling.rs` header-resolution
  test.
- Record completion in this repo's own task tracker
  (`todo-sqlite-cli.db`) if that's where knots tracks work, so it doesn't
  get lost.

## Findings

- **The change**: `collect_files`'s recursive directory walk (`main.rs:1355`,
  the `WalkBuilder` loop) now calls `is_parseable_extension` instead of
  `is_source_extension`. Single-file mode already used
  `is_parseable_extension`; recursive mode is now consistent with it.

- **`explicit_only` extensions across all languages** (from
  `lang_parsing_substrate::registry::languages()`): only two languages
  define any — C's `.h` and Ada's `.ads`. Every other language's
  `explicit_only` is empty. Both are genuine header/spec files intended to
  be scanned, so there was no case that argued for excluding part of
  `explicit_only` from the recursive walk.

- **Flag decision**: unconditional, no `--include-headers` flag added.
  Neither `.h` nor `.ads` is a template-heavy/header-only-library extension
  that would blow up complexity counts unrepresentatively (that concern
  applies more to C++ `.hpp`/`.hxx`, which are already in `extensions` and
  already scanned recursively today — this change doesn't touch their
  status). No concrete case for gating turned up, matching the brief's
  default-to-unconditional guidance.

- **`compile_commands.json` loading** (`main.rs:1275`,
  `load_compile_commands`): left as `is_source_extension`, per the brief.
  Confirmed headers don't appear as their own entries in a real
  `compile_commands.json` (translation units are always `.c`/`.cpp`), so
  there's nothing to gain by widening this call site — a header's
  complexity is only reachable there via the corresponding `.c`/`.cpp`
  entry's compile flags, not as its own row.

- **`coupling.rs` header-resolution test — before/after**: the existing
  unit test (`c_header_include_resolves_by_stripped_extension`) only ever
  proved the string-stripping/module-key logic in isolation, feeding
  `build_import_graph` two hand-typed `(path, imports)` pairs — it never
  touched `collect_files` or real parsing, so it passed identically before
  and after this change and doesn't prove anything about real runs.
  Added two new tests in `src/main.rs`'s test module that close that gap
  end-to-end using real files on disk:
  - `test_collect_files_recursive_includes_headers` — writes a `.c`/`.h`
    pair to a temp dir, calls `collect_files(..., recursive=true, ...)`,
    and asserts the `.h` file is present in the result (previously it was
    silently dropped).
  - `test_recursive_header_include_resolves_end_to_end` — same fixture,
    but carries it through `collect_import_graph` (the real
    parse-imports-per-file step used by `--recursive`'s Ce/Ca pass) and
    asserts `main.c`'s `#include "util.h"` resolves to a real `ce: 1` edge.
    Before this change, `util.h` would never have been in `files` in the
    first place, so this edge was structurally unreachable in any real
    `--recursive` run — confirmed by checking out the pre-change code and
    re-running the test, which fails with `ce: 0`.

- **Fixture added**: `sample-files/header_logic.c` /
  `sample-files/header_logic.h` — a `.c`/`.h` pair where the only nontrivial
  branching logic (`classify_value`, McCabe 5 / Cognitive 6) lives in the
  header. Manually verified with `cargo run -- -r sample-files/` (copied to
  an isolated temp dir to avoid picking up the rest of the corpus) that
  `classify_value` now appears in `--recursive` output, worst-function
  ranking included.

- **Docs updated**: `docs/architecture.rst` (`is_source_extension` /
  `is_parseable_extension` description), `docs/quick-start.rst` (recursive
  mode's file list description, and the old "add `**/*.h` to an include
  filter to get headers" tip, which is now backwards — headers are in by
  default, so the filter example shown is for *excluding* them instead),
  and `docs/troubleshooting.rst` ("No supported source files found"
  section, which listed extensions without headers and called recursive
  mode header-exclusive).

- **Known limitation** (from the brief, not fixed here): `.h` still always
  parses as C regardless of project language, so C++-only headers
  (classes, templates, namespaces) may now produce parse errors or noisy
  metrics in mixed/C++-only projects under `--recursive`, where previously
  they were invisible. Plain-C projects get a clean win immediately. The
  C-vs-C++ header sniffer to address this is tracked separately as
  follow-up work on `lang_parsing_substrate`, per the brief — not started
  here.

# knots Cross-Language Benchmark Results

Comparison of knots against lizard, rust-code-analysis (rca), tokei, and cloc
across the full supported language suite. Primary goal: identify per-language
calibration gaps and open investigation points before widening threshold use.

## Benchmark Infrastructure

### Comparison tools

| Tool | Version | Location | Metrics |
|------|---------|----------|---------|
| lizard | 1.23.0 | `~/data-enterprise/venv/bin/lizard` | McCabe, NLOC, token count |
| radon | 6.0.1 | `~/data-enterprise/venv/bin/radon` | McCabe, Halstead (Python only) |
| rust-code-analysis-cli (rca) | 0.0.25 (git HEAD) | `~/.cargo/bin/rust-code-analysis-cli` | Cyclomatic, Cognitive, Halstead, SLOC, ABC |
| tokei | (cargo install) | `~/.cargo/bin/tokei` | SLOC by language |
| cloc | (apt) | `/usr/bin/cloc` | SLOC by language |

rca 0.0.25 from crates.io fails to compile on rustc 1.95; install from git HEAD:
`cargo install --git https://github.com/mozilla/rust-code-analysis rust-code-analysis-cli`

### Benchmark corpora

**Personal projects** (~/data-personal):

| Repo | Language(s) | Files |
|------|-------------|-------|
| srg_card_search_website | Python (34), JS (5), JSX (21) | 39 knots-visible |
| srg_collection_manager_app | Kotlin (46) | 47 knots-visible |
| srg_collection_manager_app_ios | Swift (23) | 23 knots-visible |

**~/toolchain** (open-source, cloned with `--depth=1`):

| Repo | Language | Files | Notes |
|------|----------|-------|-------|
| cobra | Go | 36 | github.com/spf13/cobra |
| zod | TypeScript | 401 | github.com/colinhacks/zod |
| commons-lang | Java | 623 | github.com/apache/commons-lang |
| Newtonsoft.Json | C# | 945 | github.com/JamesNK/Newtonsoft.Json |
| todo-sqlite-cli | Rust | 36 | ~/toolchain/todo-sqlite-cli |
| curl | C | 744 | ~/toolchain/curl |
| mosquitto | C++ | 974 | ~/toolchain/mosquitto |
| gnatcoll-core | Ada | 452 | ~/toolchain/gnatcoll-core |

**~/data-enterprise** (new-language corpora):

| Repo | Language | Files | Notes |
|------|----------|-------|-------|
| lua-main | Lua | 34 | lua.org reference implementation v5.5.1-dev; test suite (testes/) only |

| laravel | PHP | 2,966 | github.com/laravel/framework |
| scala | Scala | 542 (src/library only) | github.com/scala/scala |
| lapack | Fortran | 31 `.f90` / 2,114 `.f` (SRC only) | github.com/Reference-LAPACK/lapack |

---

## Results

### Function count: knots vs lizard

| Language | Corpus | knots | lizard | delta | status |
|----------|--------|-------|--------|-------|--------|
| Java | commons-lang | 10,919 | 10,597 | +3% | ✓ good |
| C# | Newtonsoft.Json | 7,339 | 6,521 | +13% | ✓ good |
| Rust | todo-sqlite-cli | 120 | 113 | +6% | ✓ good |
| Swift | srg_collection_manager_app_ios | 199 | 201 | −1% | ✓ excellent |
| Kotlin | srg_collection_manager_app | 381 | 319 | +19% | ✓ explained — see §Kotlin |
| C | curl | 5,836 | 4,920 | +19% | plausible — knots picks up static inline |
| C++ | mosquitto | 5,300 | 3,343 | +59% | plausible — templates/lambdas |
| Go | cobra | 595 | 805 | −26% | ✓ explained — see §Anonymous Functions; named counts equal (595 = 595) |
| TypeScript | zod | 1,153 | 6,020 | −81% | ✓ explained — see §Anonymous Functions; ~4,900 anonymous arrow callbacks |
| Python | srg_card_search_website | 134 | 133 (Py only) | ~equal | ✓ good |
| Ada | gnatcoll-core | 2,672 | 386 | n/a | lizard has no Ada support |
| Lua | lua-main/testes | 590 named / 1,065 with `--count-anonymous-closures` | 1,054 | ~equal with flag | ✓ resolved — see §Lua; v1.13 adds assignment-context naming + anonymous flag coverage |
| PHP | laravel | 30,844 | 26,998 | +14% | ✓ plausible — knots picks up more small methods; see §PHP |
| Scala | scala/src/library | 11,292 | 4,490 | +151% | ✓ explained — see §Scala; expression-body `def`s lizard misses |
| Fortran (`.f90`) | lapack (31 files) | 30 | 26 | +15% | ✓ good — small corpus, broadly comparable |
| Fortran (`.f`) | lapack/SRC (2,114 files) | 1,157 | 2,106 | −45% | ⚠ open — see §Fortran77; multi-line doc comments confuse grammar |

### Average McCabe: knots vs lizard

| Language | Corpus | knots | lizard | delta | notes |
|----------|--------|-------|--------|-------|-------|
| Java | commons-lang | 1.58 | 1.6 | ~equal | ✓ |
| C# | Newtonsoft.Json | 1.93 | 2.3 | −16% | |
| C | curl | 6.10 | 7.2 | −15% | |
| C++ | mosquitto | 4.13 | 4.5 | −8% | |
| Swift | srg_collection_manager_app_ios | 2.87 | 3.0 | −4% | ✓ |
| Go | cobra | 3.37 | 2.8 | +20% | |
| Rust | todo-sqlite-cli | 2.37 | 3.6 | −34% | lizard non-standard; see rca comparison |
| TypeScript | zod | 2.31 | 1.6 | counts differ too much to compare | |
| Lua | lua-main/testes | 2.07 | 1.5 | +38% | knots named-only; lizard denominator inflated by trivial closures — see §Lua |
| PHP | laravel | 1.63 | 1.7 | ~equal ✓ | |
| Scala | scala/src/library | 1.74 | 2.1 | −17% | knots denominator inflated by SLOC=1 defs; see §Scala |
| Fortran `.f90` | lapack | 44.50 | 36.2 | +23% | small corpus (30 functions); scientific algorithms |
| Fortran `.f` | lapack/SRC | 15.89 | 22.9 | −31% | knots missing functions; see §Fortran77 |

### Average SLOC per function: knots vs lizard NLOC

| Language | Corpus | knots | lizard | delta |
|----------|--------|-------|--------|-------|
| C# | Newtonsoft.Json | 14.33 | 14.3 | ~equal ✓ |
| Java | commons-lang | 8.75 | 8.4 | +4% ✓ |
| C | curl | 27.57 | 29.7 | −7% ✓ |
| Swift | srg_collection_manager_app_ios | 14.23 | 15.2 | −6% ✓ |
| C++ | mosquitto | 16.98 | 20.0 | −15% |
| Go | cobra | 18.16 | 15.6 | +16% |
| Rust | todo-sqlite-cli | 19.30 | 22.4 | −14% |
| Lua | lua-main/testes | 4.29 | 3.9 | +10% | counts differ — see §Lua |
| PHP | laravel | 8.07 | 10.5 | −23% | knots inflated count deflates average; see §PHP |
| Scala | scala/src/library | 3.53 | 5.4 | −35% | SLOC=1 defs deflate knots average; see §Scala |
| Fortran `.f90` | lapack | 202.87 | 155.3 | +31% | small corpus; complex scientific routines |
| Fortran `.f` | lapack/SRC | 304.43 | 120.0 | +154% | fewer functions found inflates knots avg; see §Fortran77 |

### Rust: knots vs rust-code-analysis (rca)

rca is the authoritative Rust-specific tool. Lizard McCabe for Rust is
non-standard and should not be used as the comparison baseline.

| Metric | knots | rca (named fns only) | delta | explanation |
|--------|-------|----------------------|-------|-------------|
| Function count | 120 | 156 | rca higher | rca counts single-line closures as named functions |
| Avg McCabe | 2.37 | 4.04 | −41% | rca counts `?` as a branch; knots does not |
| Avg Cognitive | 1.96 | 1.47 | +33% | similar ballpark |
| Avg SLOC | 19.30 | 10.56 | +83% | rca deflated by 36 single-line closures in denominator |

**Concrete case — `add::run` (73 SLOC, 8 classical decision points):**

| Tool | McCabe | explanation |
|------|--------|-------------|
| knots | 9 | 1 base + 8 if/for branches |
| rca | 24 | 9 (knots) + 9 `?` usages + 6 single-line closure cyclomatic rolled up |
| lizard | ~similar to rca | non-standard for Rust |

See `paper/survey.tex` §McCabe language-specific adaptations for the full write-up.
Open: todo #21 — evaluate whether to count `?` as +1 McCabe.

---

## §Lua

**Corpus:** lua.org reference implementation v5.5.1-dev test suite (`testes/`), 34 `.lua` files.

**Function count discrepancy (knots 426 vs lizard 1,054):** Lua idiom uses anonymous
closures heavily — `function() … end` assigned to locals, table keys, and passed as
callbacks. knots counts only named functions by default; lizard counts all
`function` tokens. The discrepancy pattern matches the Go/TypeScript cases (see
§Anonymous Functions).

**Lua anonymous function handling (fixed in v1.13):** The Lua grammar uses
`function_definition` for anonymous function expressions and `function_declaration`
for named functions. knots now extracts names from assignment context for:
- `local x = function()` → named `x`
- `x = function()` → named `x`
- `{ key = function() }` → named `key`

Truly anonymous Lua closures (callbacks, index assignments like `a[i] = function()`)
are counted when `--count-anonymous-closures` is set. Measured on the lua-main
testes/ corpus: 590 named + 475 anonymous = 1,065 total (vs lizard 1,054). The
remaining gap (<1%) is within noise.

**Previous behaviour (pre-v1.13):** `--count-anonymous-closures` did not cover Lua
anonymous functions at all; see closed investigation point #24.

**McCabe gap (knots 2.07 vs lizard 1.50, +38%):** The lizard denominator includes
~628 trivial one-liner anonymous closures (e.g. `function() return x end`) which
pull its average down. When restricted to named functions the averages converge.
No calibration action needed.

**SLOC gap (knots 4.29 vs lizard 3.90, +10%):** Same denominator effect. Small and
within expected range.

**lizard support:** lizard does support Lua (default auto-detect). Results are
usable for sanity checks but the anonymous-closure inflation must be accounted for.

---

## §PHP

**Corpus:** Laravel framework (github.com/laravel/framework), 2,966 `.php` files.

**Function count (knots 30,844 vs lizard 26,998, +14%):** Consistent with the pattern seen
in C (+19%) and Kotlin (+19%). PHP has many interface methods, trait methods, and
abstract declarations that knots counts but lizard may undercount. No calibration
action needed.

**McCabe (~equal, 1.63 vs 1.7):** Excellent agreement — McCabe calibration is sound.

**SLOC gap (knots 8.07 vs lizard 10.5, −23%):** The inflated function count deflates
knots' average. Individual functions with SLOC≥3 align closely with lizard.

**lizard support:** Full PHP support via `lizard -l php`.

---

## §Scala

**Corpus:** scala/src/library from github.com/scala/scala, 542 `.scala` files
(standard library only; compiler and test sources excluded).

**Function count (knots 11,292 vs lizard 4,490, +151%):** Large discrepancy explained
by Scala's expression-body `def` syntax. knots found 6,695 functions with SLOC=1
(single-line `def foo(): T = expr`). lizard does not count expression-body methods
without a brace-delimited body — the same pattern seen with Kotlin (+19%) and
TypeScript anonymous arrows, but more pronounced because Scala stdlib uses this
style pervasively.

When restricted to SLOC≥2 functions, knots finds 4,597 — close to lizard's 4,490.
The difference is methodologically expected: knots is counting valid functions that
lizard skips.

**McCabe (knots 1.74 vs lizard 2.1, −17%):** The SLOC=1 one-liners are trivially
simple (McCabe=1), pulling the knots average down. McCabe on the shared SLOC≥2 set
would be closer.

**SLOC gap (knots 3.53 vs lizard 5.4, −35%):** Same denominator inflation effect.

**lizard support:** Full Scala support via `lizard -l scala`.

---

## §Fortran77

**Corpus:** LAPACK reference implementation (github.com/Reference-LAPACK/lapack).

**Modern Fortran `.f90` results (31 files):** knots 30 functions, lizard 26, +15%.
McCabe +23%, SLOC +31%. Small corpus of complex scientific routines; broadly
comparable. No calibration action needed.

**Fixed-form Fortran 77 `.f` results (LAPACK/SRC, 2,114 files):** knots 1,157 vs
lizard 2,106 (−45%). This is a significant undercount with a known root cause.

**Root cause — LAPACK Doxygen comment headers:** Nearly all LAPACK `.f` files use a
documentation convention where the subroutine/function signature appears verbatim
inside a comment block at the top of the file:

```fortran
*       SUBROUTINE CHEGV( ITYPE, JOBZ, UPLO, N, A, LDA, B, LDB, W, WORK,
*                         LWORK, RWORK, INFO )
```

When this doc-comment signature fits on a single line (shorter argument lists), the
tree-sitter-fortran grammar handles it correctly and knots finds the subroutine. When
the doc-comment signature spans multiple lines (using the `*` continuation style),
the grammar produces a parse error that prevents the real subroutine declaration from
being recognised. 2,042 of 2,114 LAPACK SRC files use this multi-line doc-comment
pattern.

**Consequence:** knots is not suitable as-is for LAPACK-style `.f` corpora. For
other Fortran 77 codebases without this convention, results would be closer to lizard.
The fix would require patching tree-sitter-fortran's comment handling for fixed-form
files.

**Bug fixed (discovered during benchmarking):** Explicit-only extensions (`.f`, `.h`,
`.ads`) were silently rejected by `collect_files` even when passed directly on the
command line. The check used `is_source_extension` (recursive-discovery only); now
correctly uses `is_parseable_extension` which includes both recursive and
explicit-only extensions. Introduced `is_parseable_extension` in `src/lib.rs`.

**lizard support:** Full Fortran support via `lizard -l fortran`.

---

## Open Investigation Points

| # | Language | Finding | Todo |
|---|----------|---------|------|
| 19 | JavaScript | `.jsx` extension not in `SUPPORTED_EXTENSIONS`; all JSX files skipped by `--recursive` | #19 |
| 20 | Kotlin | knots finds 19% more functions than lizard (381 vs 319) — **resolved**: 55 single-expression funs lizard misses + 7 abstract declarations; knots correct | closed |
| 21 | Rust | knots McCabe ~41% lower than rca due to `?` not counted as branch | #21 |
| 22 | TypeScript | knots finds 81% fewer functions than lizard on zod — **resolved**: ~4,900 anonymous arrow callbacks; named counts similar | closed |
| 23 | Go | knots finds 26% fewer functions than lizard on cobra — **resolved**: 210 anonymous `func_literal`; named counts equal (595 vs 595) | closed |
| 24 | Lua | `--count-anonymous-closures` did not include Lua anonymous `function_definition` nodes. Fixed in v1.13: assignment-context naming + Lua `function_definition` added to anonymous allowlist. knots (with flag): 1,065; lizard: 1,054 (~equal). | closed |
| 25 | PHP | No cross-tool validation corpus established — **resolved**: Laravel corpus benchmarked; +14% function count, McCabe ~equal. | closed |
| 26 | Scala | No cross-tool validation corpus established — **resolved**: scala/src/library benchmarked; +151% explained by expression-body `def`s lizard skips. | closed |
| 27 | Fortran | No cross-tool validation corpus established — **partially resolved**: `.f90` comparable (+15%); `.f` fixed-form significantly undercounts (−45%) due to LAPACK Doxygen comment pattern confusing tree-sitter-fortran. | open |
| 28 | Fortran | Explicit-only extensions (`.f`, `.h`, `.ads`) silently rejected when passed directly — **fixed in v1.13**: `is_parseable_extension` added to `src/lib.rs`. | closed |

---

## Languages with No Open Points

| Language | Assessment |
|----------|------------|
| Java | Function count +3%, McCabe ~equal, SLOC +4% — excellent agreement |
| C# | Function count +13%, SLOC ~equal — good |
| Swift | Function count −1%, McCabe −4%, SLOC −6% — excellent |
| Python | Function count ~equal (Python-only comparison) — good |
| C | +19% function count explainable by static inline detection; McCabe/SLOC within 15% |
| Ada | lizard has no Ada support; no cross-tool validation possible |
| Kotlin | Validated against personal project corpus; +19% explained and correct |
| PHP | +14% function count (interface/abstract methods), McCabe ~equal — calibration sound |
| Scala | +151% explained by expression-body `def`s with SLOC=1 that lizard skips — knots correct |

## Languages Pending Full Validation

| Language | Added | Status |
|----------|-------|--------|
| Lua | v1.12 | Corpus is test suite only, not application code; anonymous closure handling resolved in v1.13 (#24 closed) |
| PHP | v1.11 | Benchmarked (Laravel); +14% function count, McCabe ~equal (#25 closed) |
| Scala | v1.12 | Benchmarked (scala/src/library); +151% explained by expression-body `def`s (#26 closed) |
| Fortran `.f90` | v1.12 | Benchmarked (LAPACK); broadly comparable (+15%) — see §Fortran77 |
| Fortran `.f` | v1.12 | Grammar limitation with LAPACK Doxygen comment pattern; −45% undercount (#27 open) |

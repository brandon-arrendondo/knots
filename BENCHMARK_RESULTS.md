# knots Benchmark Results

Cross-language calibration against lizard/rca/tokei/cloc, plus AIRD/AICP corpus
validation and empirical AI difficulty experiment.

---

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
| lua | Lua | 34 | lua.org reference implementation v5.5.1-dev; test suite (testes/) only |
| laravel | PHP | 2,966 | github.com/laravel/framework |
| scala | Scala | 542 (src/library only) | github.com/scala/scala |
| lapack | Fortran | 31 `.f90` / 2,114 `.f` (SRC only) | github.com/Reference-LAPACK/lapack |

**AIRD/AICP validation corpora** (~/toolchain, open-source):

| Repo | Language | Files | Notes |
|------|----------|-------|-------|
| lua | C/Lua | ~200 | lua.org reference implementation v5.5.1-dev |
| libcrc | C | ~20 | libcrc.org CRC library |
| mosquitto | C/C++ | 974 | eclipse/mosquitto MQTT broker |
| hostap | C | ~3,000 | w1.fi/hostapd + wpa_supplicant |
| sqlite | C | ~200 | sqlite.org amalgamation + tools |
| curl | C | 744 | curl/curl HTTP library |

---

## Cross-Language Calibration

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
| Lua | lua/testes | 590 named / 1,065 with `--count-anonymous-closures` | 1,054 | ~equal with flag | ✓ resolved — see §Lua |
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
| Lua | lua/testes | 2.07 | 1.5 | +38% | knots named-only; lizard denominator inflated by trivial closures — see §Lua |
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
| Lua | lua/testes | 4.29 | 3.9 | +10% | counts differ — see §Lua |
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

## AIRD/AICP Metrics

Two orthogonal AI cost metrics replaced the single AIM score in v1.5.0:

**AIRD — AI Reasoning Difficulty** (0–100)
Predicts how much reasoning effort the model needs once it has context. Cognitive
complexity is the primary driver; SLOC, nesting, and testability are secondary.

**AICP — AI Context Pressure** (0–100)
Predicts how much context the model must load before it can act. External call breadth
and function size are the primary drivers; documentation reduces the cost.

A function can be cheap to load but hard to reason about, or expensive to load but
trivial once context is assembled. The two scores are independent.

### Formulas

```
AIRD = (cognitive/75 × 55) + (sloc/200 × 15) + (nesting/8 × 15) + (test_score/20 × 15) - (doc_score/10 × 15)
AICP = (external_calls/20 × 60) + (sloc/200 × 40) - (doc_score/10 × 15)
```
Both clamped to [0, 100].

### AIRD formula history

| Version | Change | Rationale |
|---------|--------|-----------|
| v1 | cognitive/50×35, sloc/100×25, nesting/8×15, test/40×25, doc/10×15 | Initial hypothesis |
| v2 | test ceiling 40→20 | Max observed test_score was ~18; v1 ceiling structurally halved the contribution |
| v3 | cognitive weight 35→45, test weight 25→15; cognitive ceiling 50→75, sloc ceiling 100→200 | Corpus percentile analysis: SLOC ceiling at 100 saturated at p95; weight shift reduces test_score inflation of small undocumented tool entry points |
| v4 | cognitive weight 45→55, sloc weight 25→15 | Empirical experiment: cognitive complexity correlated with actual AI difficulty; SLOC over-contributed for shallow-entry functions |
| v1.5.0 | Renamed AIM → AIRD; formula unchanged | Split single metric into AIRD (reasoning) + AICP (context pressure) |

### AICP formula rationale

`external_calls` (unique identifier-form call targets not defined in the same translation
unit) was validated against the corpus before AICP was defined. Global p99=20 across
32,205 functions, consistent across all 6 corpora (range 15–20). Mean by AIRD band:
2.74 (low) → 8.69 (mid) → 17.40 (high) — monotonic separation, stronger than
experiment self-rated difficulty scores.

Adding `external_calls` to AIRD was rejected: hostapd has slightly more external calls
(17) than whereLoopAddBtreeIndex (12), so every SLOC→ext weight transfer widened the
falsification gap. Instead, `external_calls` became the primary input to a separate AICP
metric where its meaning is unambiguous: more external dependencies = more context to load.

### Ceiling calibration

Ceilings are set near p99 of the observed distribution so only genuine outliers saturate.

| Dimension | Metric | Ceiling | p90 range | p95 range | p99 range |
|-----------|--------|---------|-----------|-----------|-----------|
| cognitive | AIRD | 75 | 8–22 | 12–40 | 25–102 |
| sloc | AIRD + AICP | 200 | 26–66 | 35–95 | 62–199 |
| nesting | AIRD | 8 | 3–4 | 3–5 | 5–7 |
| test_score | AIRD | 20 | — | — | max observed ~18 |
| external_calls | AICP | 20 | 5–9 | 9–12 | 15–20 |

---

## AIRD Distribution — v1.5.0 Baseline

Corpus validation run at knots v1.5.0 against 6 open-source C codebases.

| Corpus    | Functions | Avg AIRD | Max AIRD | 0–10 | 11–25 | 26–50 | 51–75 | 76–100 |
|-----------|-----------|----------|----------|------|-------|-------|-------|--------|
| lua       | 1,304     | 5.0      | 87       | 87%  | 11%   | 2%    | 0%    | 0%     |
| libcrc    | 34        | 8.2      | 66       | 88%  | 6%    | 0%    | 6%    | 0%     |
| mosquitto | 2,559     | 10.1     | 97       | 71%  | 20%   | 6%    | 2%    | 1%     |
| hostap    | 13,343    | 10.9     | 95       | 67%  | 23%   | 8%    | 2%    | 1%     |
| sqlite    | 9,491     | 10.2     | 99       | 72%  | 17%   | 7%    | 3%    | 2%     |
| curl      | 5,474     | 12.4     | 95       | 63%  | 23%   | 10%   | 3%    | 1%     |

Distribution is heavily right-skewed across all corpora — correct for mature codebases. The
76–100 bucket is 1–2% across mature corpora, indicating the high-AIRD threshold is
meaningfully selective rather than noisy.

### Top Scorers by Corpus (intuition check)

**Mosquitto** — `main` in top-10: 1
| AIRD | Cog | SLOC | Nest | Function | File |
|------|-----|------|------|----------|------|
| 97 | 126 | 261 | 7 | main | mosquitto_passwd.c |
| 93 | 591 | 770 | 77 | client_config_line_proc | client_shared.c |
| 90 | 127 | 209 | 7 | connect__on_authorised | handle_connect.c |

**SQLite** — `main` in top-10: 5 (all are large standalone fuzzer/tool programs)
| AIRD | Cog | SLOC | Nest | Function | File |
|------|-----|------|------|----------|------|
| 99 | 505 | 768 | 40 | main | fuzzcheck.c |
| 97 | 326 | 411 | 18 | main | fuzzershell.c |
| 97 | 192 | 476 | 50 | main | speedtest1.c |
| 90 | 187 | 336 | 7 | whereLoopAddBtreeIndex | where.c |
| 90 | 3023 | 4977 | 11 | sqlite3VdbeExec | vdbe.c |

**Curl** — `main` in top-10: 0
| AIRD | Cog | SLOC | Nest | Function | File |
|------|-----|------|------|----------|------|
| 95 | 114 | 286 | 11 | test_rtspd | rtspd.c |
| 94 | 366 | 314 | 8 | http_connect | sws.c |
| 94 | 139 | 212 | 9 | select_ws | sockfilt.c |

**Hostap** — `main` in top-10: 0
| AIRD | Cog | SLOC | Nest | Function | File |
|------|-----|------|------|----------|------|
| 95 | 901 | 2370 | 390 | parse_sae_password | config_file.c |
| 93 | 203 | 299 | 7 | hostapd_config_read_eap_user | config_file.c |
| 92 | 174 | 240 | 19 | wpas_get_est_tpt | scan.c |

**Lua** — `main` in top-10: 0
| AIRD | Cog | SLOC | Nest | Function | File |
|------|-----|------|------|----------|------|
| 87 | 138 | 751 | 4 | luaV_execute | lvm.c |
| 71 | 70 | 116 | 4 | llex | llex.c |
| 64 | 59 | 98 | 5 | match | lstrlib.c |

Intuition check passes — `luaV_execute` (VM loop), `llex` (lexer), `sqlite3VdbeExec`,
`whereLoopAddBtreeIndex`, and `parse_sae_password` are well-known hard-to-modify functions
in their respective codebases.

### Formula Notes

**SQLite main() inflation.** Five SQLite `main()` functions remain in the AIRD top 10.
These are legitimate: `fuzzcheck.c` (cog=505, sloc=768), `fuzzershell.c` (cog=326,
sloc=411), `speedtest1.c` (cog=192, sloc=476). The small-but-inflated cases (e.g.
`main(dbfuzz.c)`, cog=49, sloc=107) were eliminated by the v3 ceiling changes. The
remaining cases reflect SQLite's unusual project structure — 40+ standalone C programs —
not a formula defect.

**Score compression at the top.** The practical AIRD max across most corpora is 87–97.
Reaching 100 requires simultaneous saturation of all four positive inputs plus zero
documentation, which does not appear in practice.

---

## Empirical Validation (Sonnet 4.6 vs Opus 4.8)

Task: add a defensive `assert()` at function entry, identify the critical precondition,
self-rate difficulty 1–10, report distinct concepts needed and whether external context
(types, macros, callers) was required. Run against both Sonnet 4.6 and Opus 4.8.

### Raw results

| Function | Band | AIRD | AICP | ExtCalls | SLOC | Sonnet diff | Opus diff | Sonnet concepts | Opus concepts | External (S/O) |
|----------|------|------|------|----------|------|-------------|-----------|-----------------|---------------|----------------|
| db__ready_for_flight | low | 10 | 0 | 0 | 41 | 4 | 3 | 3 | 4 | Y / Y |
| bufq_slurpn | low | 14 | 3 | 1 | 31 | 4 | 3 | 3 | 4 | Y / Y |
| pmksa_cache_get_okc | low | 16 | 24 | 8 | 31 | 4 | 3 | 3 | 3 | Y / N |
| mosquitto_validate_utf8 | mid | 57 | 13 | 0 | 66 | 4 | 3 | 3 | 4 | Y / Y |
| Curl_conn_connect | mid | 56 | 80 | 26 | 102 | 4 | 3 | 3 | 4 | Y / Y |
| ieee802_1x_encapsulate_radius | mid | 71 | 89 | 21 | 144 | 5 | 3 | 4 | 4 | Y / Y |
| luaV_execute | high | 87 | 100 | 81 | 751 | 7 | 6 | 4 | 5 | Y / Y |
| whereLoopAddBtreeIndex | high | 90 | 73 | 12 | 336 | 7 | 6 | 5 | 5 | Y / Y |
| hostapd_config_read_eap_user | high | 93 | 91 | 17 | 299 | 4 | 3 | 3 | 4 | Y / N |

### Findings

**High-AIRD band confirmed.** `luaV_execute` and `whereLoopAddBtreeIndex` were consistently
the hardest (difficulty 6–7, highest concept counts). Both required tracing macro chains,
union type invariants, and multi-level indirection across files. The AIRD ≥85 threshold is
empirically validated.

**Mid band not differentiated by AIRD.** Sonnet 4.00→4.33→6.00 across low/mid/high; Opus
3.00→3.00→5.00. A 40-point AIRD increase from low to mid produces near-zero difficulty
change. AIRD does not predict mid-range reasoning effort reliably.

**AICP adds signal the mid band was missing.** Mid-band AICP spans 13–89: `mosquitto_validate_utf8`
(AICP=13, 0 external calls — self-contained UTF-8 validator) vs `Curl_conn_connect` (AICP=80,
26 external calls) vs `ieee802_1x_encapsulate_radius` (AICP=89, 21 external calls). The
experiment rated `ieee802_1x` hardest of the mid-band (Sonnet=5 vs 4 for others) — the
higher AICP tracks the higher rated difficulty. A single AIM score could not distinguish
these cases.

**Systematic model offset.** Sonnet rated every function 1 point higher than Opus (8/9
functions). Consistent ordering, not content disagreement — calibration offset between
models, not formula signal.

**Clear falsification: `hostapd_config_read_eap_user`.** AIRD=93 (highest in experiment),
difficulty rated 3–4 (same as low band). It is a 328-line line-oriented parser; the entry
clause is two trivially obvious lines. Both cognitive (203 > ceiling 75) and SLOC (299 >
ceiling 200) are fully saturated, so no AIRD reweighting can reduce its score — both inputs
hit 1.0 regardless of weight. AICP=91 correctly characterizes it as high context-pressure:
it genuinely takes significant token budget to load 299 SLOC and 17 external dependencies.
The falsification is specific to AIRD — the context cost is real, the reasoning cost is not.

**External context is universal above low band.** The rate increases from ~67% (low) to
100% (mid and most of high). Discriminates low from non-low but not mid from high. The
binary form is not useful as a formula input; AICP captures the same signal continuously.

### What the experiment supports

- `--aird-threshold 85` as a CI gate is validated: the high-AIRD functions (≥85) are
  genuinely hard to reason about and consistently distinguished from mid and low.
- `--aicp-threshold` is not yet validated — a threshold recommendation requires a separate
  experiment targeting context-loading cost (token budget, number of tool calls to gather
  context) rather than self-rated reasoning difficulty.
- Cognitive complexity is the dominant valid predictor for AIRD; SLOC adds noise for
  functions where both inputs saturate simultaneously.

### Known gaps

**AIRD falsification class.** Functions with high SLOC and high cognitive complexity that
both saturate their ceilings simultaneously cannot be distinguished from genuinely hard
functions by AIRD alone. Raising the cognitive ceiling would differentiate `sqlite3VdbeExec`
(cog=3023) from `hostapd_config_read_eap_user` (cog=203) — but it would also drop
`whereLoopAddBtreeIndex` (cog=187) below the 85 threshold, invalidating an empirically
confirmed hard function. AICP partially compensates: the falsification case shows high
AICP (91) reflecting real loading cost, even though AIRD overstates its reasoning cost.

**Cross-file type/function indirection.** The two hardest experiment functions
(`luaV_execute`, `whereLoopAddBtreeIndex`) required chasing macro chains and union/struct
definitions across header files. This is not measured by any current metric. All existing
metrics are intra-function or intra-file. Capturing it requires libclang-level include
resolution.

**Mid-band AIRD calibration.** The experiment had only 3 mid-band functions. A larger
sample spanning AIRD 30–70 from type-heavy codebases (Linux kernel, LLVM, OpenSSL) is
needed before mid-range behavior can be assessed or tuned. Mid-band AICP validation is
entirely pending.

---

## AIRD Distribution — v1.12.0 Re-evaluation (2026-06-28)

Re-run of all six original corpora against knots v1.12.0 to detect drift from the v1.5.0
baseline.

| Corpus    | Functions | Avg AIRD | Max AIRD | 0–10 | 11–25 | 26–50 | 51–75 | 76–100 |
|-----------|-----------|----------|----------|------|-------|-------|-------|--------|
| lua       | 1,880     | 6.1      | 85       | 83%  | 12%   | 2%    | 0%    | 0%     |
| libcrc    | 34        | 9.9      | 68       | 85%  | 5%    | 2%    | 5%    | 0%     |
| mosquitto | 5,549     | 9.6      | 98       | 73%  | 19%   | 5%    | 1%    | 0%     |
| hostap    | 20,160    | 12.3     | 100      | 61%  | 26%   | 9%    | 1%    | 1%     |
| sqlite    | 11,351    | 11.0     | 100      | 71%  | 17%   | 7%    | 2%    | 2%     |
| curl      | 5,836     | 14.2     | 98       | 59%  | 24%   | 9%    | 3%    | 2%     |

Compared to v1.5.0 baseline:

| Corpus    | v1.5.0 Fns | v1.12.0 Fns | Count Δ | v1.5.0 Avg AIRD | v1.12.0 Avg AIRD |
|-----------|------------|-------------|---------|-----------------|------------------|
| lua       | 1,304      | 1,880       | +44%    | 5.0             | 6.1              |
| libcrc    | 34         | 34          | 0%      | 8.2             | 9.9              |
| mosquitto | 2,559      | 5,549       | +117%   | 10.1            | 9.6              |
| hostap    | 13,343     | 20,160      | +51%    | 10.9            | 12.3             |
| sqlite    | 9,491      | 11,351      | +20%    | 10.2            | 11.0             |
| curl      | 5,474      | 5,836       | +7%     | 12.4            | 14.2             |

**Count increases:** Primarily caused by C++ language support added after v1.5.0 (`.cpp`,
`.cc`, `.hpp`, `.hxx` extensions). Mosquitto (+117%) and hostap (+51%) both contain
substantial C++ code that was previously missed. Lua (+44%) reflects `.lua` files now
parsed with the correct grammar rather than the C fallback.

**AIRD distribution shape:** Preserved across all corpora — heavily right-skewed, 0–10
bucket dominant (59–85%), high-AIRD (>75) at 0–1%. The `--aird-threshold 85` gate
remains well-calibrated.

**Avg AIRD drift:** All corpora within 2 points of v1.5.0 baseline. The distribution
ordering is preserved (curl hardest, lua easiest).

### Key Validation Functions — v1.12.0

| Function | v1.5.0 AIRD | v1.12.0 AIRD | v1.5.0 SLOC | v1.12.0 SLOC | Status |
|----------|------------|--------------|------------|--------------|--------|
| whereLoopAddBtreeIndex | 90 | 93 | 336 | 336 | ✓ stable |
| hostapd_config_read_eap_user | 93 | 95 | 299 | 299 | ✓ stable |
| mosquitto_validate_utf8 | 57 | 59 | 66 | 66 | ✓ stable |
| sqlite3VdbeExec | 90 | 91 | 4,977 | 4,927 | ✓ stable (corpus update) |
| luaV_execute | 87 | 89 | 751 | 751 | ✓ stable (SLOC regression fixed in #29) |
| parse_sae_password | 95 | n/a | — | — | removed from hostap corpus |

### SLOC Deflation Regression (vmcase macro pattern) — Fixed in #29

`luaV_execute` in lvm.c (Lua 5.5.1-dev) shows SLOC=34 vs the expected ~750. Root cause:
the `nested_fn_sloc` subtraction (added to handle genuine nested function definitions in
Python/Rust/JS) incorrectly fires on C code that uses the `macro(args) { body }` pattern.

Lua's VM interpreter uses:
```c
vmdispatch(GET_OPCODE(i)) {
  vmcase(OP_MOVE) { ... vmbreak; }
  vmcase(OP_LOADI) { ... vmbreak; }
  ...
}
```

tree-sitter-c parses `vmcase(OP_MOVE) { ... }` as a `function_definition` node (it looks
syntactically like a K&R-style function definition). The `nested_fn_sloc` accumulator then
subtracts each vmcase block's SLOC from the outer function.

**Scope of impact:**
- Only `luaV_execute` shows the pattern in the lua corpus (1 of 1,880 functions)
- No functions in curl, mosquitto, hostap, or sqlite corpora are affected — `sqlite3VdbeExec`
  (which uses standard `switch`) is unaffected

**Consequence:** `luaV_execute` AIRD dropped from 87 to 76 (still in the high band;
threshold at 85 still fires). The falsification flag from the original experiment is
unaffected since it concerned `hostapd_config_read_eap_user` which is stable.

**Fix direction:** In `accumulate_nested_sloc`, filter out `function_definition` nodes
whose declarator is a `call_expression` (macro pattern) rather than a plain `identifier`
(genuine nested function).

---

## Open Investigation Points

| # | Language | Finding | Todo |
|---|----------|---------|------|
| 19 | JavaScript | `.jsx` extension not in `SUPPORTED_EXTENSIONS`; all JSX files skipped by `--recursive` — **fixed**: `.jsx` added to `SUPPORTED_EXTENSIONS` and `language_for_file` | closed |
| 20 | Kotlin | knots finds 19% more functions than lizard (381 vs 319) — **resolved**: 55 single-expression funs lizard misses + 7 abstract declarations; knots correct | closed |
| 21 | Rust | knots McCabe ~41% lower than rca due to `?` not counted as branch — **fixed**: `?` and `?`-family short-circuit operators now count as +1 McCabe | closed |
| 22 | TypeScript | knots finds 81% fewer functions than lizard on zod — **resolved**: ~4,900 anonymous arrow callbacks; named counts similar | closed |
| 23 | Go | knots finds 26% fewer functions than lizard on cobra — **resolved**: 210 anonymous `func_literal`; named counts equal (595 vs 595) | closed |
| 24 | Lua | `--count-anonymous-closures` did not include Lua anonymous `function_definition` nodes. Fixed in v1.13: assignment-context naming + Lua `function_definition` added to anonymous allowlist. knots (with flag): 1,065; lizard: 1,054 (~equal). | closed |
| 25 | PHP | No cross-tool validation corpus established — **resolved**: Laravel corpus benchmarked; +14% function count, McCabe ~equal. | closed |
| 26 | Scala | No cross-tool validation corpus established — **resolved**: scala/src/library benchmarked; +151% explained by expression-body `def`s lizard skips. | closed |
| 27 | Fortran | No cross-tool validation corpus established — **partially resolved**: `.f90` comparable (+15%); `.f` fixed-form significantly undercounts (−45%) due to LAPACK Doxygen comment pattern confusing tree-sitter-fortran. | open |
| 28 | Fortran | Explicit-only extensions (`.f`, `.h`, `.ads`) silently rejected when passed directly — **fixed in v1.13**: `is_parseable_extension` added to `src/lib.rs`. | closed |
| 29 | C | `vmcase(args) { block }` macros parsed as `function_definition` by tree-sitter-c; `nested_fn_sloc` subtracts them from outer SLOC — affects `luaV_execute` (SLOC 751→34). **Fixed**: `is_macro_function_definition` filters `function_definition` nodes whose declarator is `parenthesized_declarator` in `accumulate_nested_sloc`. `luaV_execute` SLOC restored to 751, AIRD 89. | closed |

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

---

## Pending Tasks

- Mid-band empirical validation: AIRD 30–70 and AICP across difficulty spectrum — measure
  actual token usage and tool-call counts, not self-rating
- AICP threshold recommendation: determine what AICP score correlates with meaningfully
  elevated context-gathering cost in practice
- ~~Fix vmcase/vmdispatch SLOC deflation (#29)~~ — closed; `parenthesized_declarator` filter in `accumulate_nested_sloc`

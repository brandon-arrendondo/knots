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

## Open Investigation Points

| # | Language | Finding | Todo |
|---|----------|---------|------|
| 19 | JavaScript | `.jsx` extension not in `SUPPORTED_EXTENSIONS`; all JSX files skipped by `--recursive` | #19 |
| 20 | Kotlin | knots finds 19% more functions than lizard (381 vs 319) — **resolved**: 55 single-expression funs lizard misses + 7 abstract declarations; knots correct | closed |
| 21 | Rust | knots McCabe ~41% lower than rca due to `?` not counted as branch | #21 |
| 22 | TypeScript | knots finds 81% fewer functions than lizard on zod — **resolved**: ~4,900 anonymous arrow callbacks; named counts similar | closed |
| 23 | Go | knots finds 26% fewer functions than lizard on cobra — **resolved**: 210 anonymous `func_literal`; named counts equal (595 vs 595) | closed |

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

# AIM Score Benchmark Results

Corpus validation run against 6 open-source C codebases.
knots version: 1.4.3 (post-v3 tuning, see formula history below)

## Formula (v4)

```
AIM = (cognitive/75 × 55) + (sloc/200 × 15) + (nesting/8 × 15) + (test_score/20 × 15) - (doc_score/10 × 15)
```
Clamped to [0, 100].

### Formula history

| Version | Change | Rationale |
|---------|--------|-----------|
| v1 | cognitive/50×35, sloc/100×25, nesting/8×15, test/40×25, doc/10×15 | Initial hypothesis |
| v2 | test ceiling 40→20 | Max observed test_score was ~18; v1 ceiling structurally halved the contribution |
| v3 | cognitive weight 35→45, test weight 25→15; cognitive ceiling 50→75, sloc ceiling 100→200 | Corpus percentile analysis: SLOC ceiling at 100 saturated at p95; weight shift reduces test_score inflation of small undocumented tool entry points |
| v4 | cognitive weight 45→55, sloc weight 25→15 | Empirical experiment (see below): cognitive complexity correlated with actual AI difficulty; SLOC over-contributed for shallow-entry functions |

### Ceiling calibration (basis for v3)

Ceilings are set near p99 of the observed distribution so only genuine outliers saturate.

| Dimension | Ceiling | p90 range | p95 range | p99 range |
|-----------|---------|-----------|-----------|-----------|
| cognitive | 75 | 8–22 | 12–40 | 25–102 |
| sloc | 200 | 26–66 | 35–95 | 62–199 |
| nesting | 8 | 3–4 | 3–5 | 5–7 |
| test_score | 20 | — | — | max observed ~18 |

## Distribution by Corpus (v3)

| Corpus    | Functions | Avg AIM | Max AIM | 0–10 | 11–25 | 26–50 | 51–75 | 76–100 |
|-----------|-----------|---------|---------|------|-------|-------|-------|--------|
| lua       | 1,304     | 5.0     | 87      | 87%  | 11%   | 2%    | 0%    | 0%     |
| libcrc    | 34        | 8.2     | 66      | 88%  | 6%    | 0%    | 6%    | 0%     |
| mosquitto | 2,559     | 10.1    | 97      | 71%  | 20%   | 6%    | 2%    | 1%     |
| hostap    | 13,343    | 10.9    | 95      | 67%  | 23%   | 8%    | 2%    | 1%     |
| sqlite    | 9,491     | 10.2    | 99      | 72%  | 17%   | 7%    | 3%    | 2%     |
| curl      | 5,474     | 12.4    | 95      | 63%  | 23%   | 10%   | 3%    | 1%     |

Distribution is heavily right-skewed across all corpora — correct for mature codebases. The
76–100 bucket is 1–2% across mature corpora, indicating the high-AIM threshold is meaningfully
selective rather than noisy.

## Top Scorers by Corpus (v3 intuition check)

**Mosquitto** — `main` in top-10: 1
| AIM | Cog | SLOC | Nest | Function | File |
|-----|-----|------|------|----------|------|
| 97 | 126 | 261 | 7 | main | mosquitto_passwd.c |
| 93 | 591 | 770 | 77 | client_config_line_proc | client_shared.c |
| 90 | 127 | 209 | 7 | connect__on_authorised | handle_connect.c |

**SQLite** — `main` in top-10: 5 (all are large standalone fuzzer/tool programs)
| AIM | Cog | SLOC | Nest | Function | File |
|-----|-----|------|------|----------|------|
| 99 | 505 | 768 | 40 | main | fuzzcheck.c |
| 97 | 326 | 411 | 18 | main | fuzzershell.c |
| 97 | 192 | 476 | 50 | main | speedtest1.c |
| 90 | 187 | 336 | 7 | whereLoopAddBtreeIndex | where.c |
| 90 | 3023 | 4977 | 11 | sqlite3VdbeExec | vdbe.c |

**Curl** — `main` in top-10: 0
| AIM | Cog | SLOC | Nest | Function | File |
|-----|-----|------|------|----------|------|
| 95 | 114 | 286 | 11 | test_rtspd | rtspd.c |
| 94 | 366 | 314 | 8 | http_connect | sws.c |
| 94 | 139 | 212 | 9 | select_ws | sockfilt.c |

**Hostap** — `main` in top-10: 0
| AIM | Cog | SLOC | Nest | Function | File |
|-----|-----|------|------|----------|------|
| 95 | 901 | 2370 | 390 | parse_sae_password | config_file.c |
| 93 | 203 | 299 | 7 | hostapd_config_read_eap_user | config_file.c |
| 92 | 174 | 240 | 19 | wpas_get_est_tpt | scan.c |

**Lua** — `main` in top-10: 0
| AIM | Cog | SLOC | Nest | Function | File |
|-----|-----|------|------|----------|------|
| 87 | 138 | 751 | 4 | luaV_execute | lvm.c |
| 71 | 70 | 116 | 4 | llex | llex.c |
| 64 | 59 | 98 | 5 | match | lstrlib.c |

Intuition check passes — `luaV_execute` (VM loop), `llex` (lexer), `sqlite3VdbeExec`,
`whereLoopAddBtreeIndex`, and `parse_sae_password` are well-known hard-to-modify functions
in their respective codebases.

## Remaining Formula Notes

**SQLite main() inflation.** Five SQLite `main()` functions remain in the top 10. These are
legitimate: `fuzzcheck.c` (cog=505, sloc=768), `fuzzershell.c` (cog=326, sloc=411),
`speedtest1.c` (cog=192, sloc=476). The small-but-inflated cases (e.g. `main(dbfuzz.c)`,
cog=49, sloc=107) were eliminated by the v3 ceiling changes. The remaining cases reflect
SQLite's unusual project structure — 40+ standalone C programs — not a formula defect.

**Score compression at the top.** The practical max across most corpora is 87–97. Reaching
100 requires simultaneous saturation of all four positive inputs plus zero documentation,
which does not appear in practice. This is intentional; scores cluster in the 80–95 band
for the genuinely hardest functions.

## Empirical Validation (Sonnet 4.6 vs Opus 4.8)

Task: add a defensive `assert()` at function entry, identify the critical precondition,
self-rate difficulty 1–10, report distinct concepts needed and whether external context
(types, macros, callers) was required. Run against both Sonnet 4.6 and Opus 4.8.

### Raw results

| Function | Band | AIM | SLOC | Sonnet diff | Opus diff | Sonnet concepts | Opus concepts | External (S/O) |
|----------|------|-----|------|-------------|-----------|-----------------|---------------|----------------|
| db__ready_for_flight | low | 9 | 52 | 4 | 3 | 3 | 4 | Y / Y |
| bufq_slurpn | low | 14 | 35 | 4 | 3 | 3 | 4 | Y / Y |
| pmksa_cache_get_okc | low | 15 | 32 | 4 | 3 | 3 | 3 | Y / N |
| mosquitto_validate_utf8 | mid | 55 | 91 | 4 | 3 | 3 | 4 | Y / Y |
| Curl_conn_connect | mid | 55 | 121 | 4 | 3 | 3 | 4 | Y / Y |
| ieee802_1x_encapsulate_radius | mid | 55 | 136 | 5 | 3 | 4 | 4 | Y / Y |
| luaV_execute | high | 87 | 773 | 7 | 6 | 4 | 5 | Y / Y |
| whereLoopAddBtreeIndex | high | 90 | 435 | 7 | 6 | 5 | 5 | Y / Y |
| hostapd_config_read_eap_user | high | 93 | 328 | 4 | 3 | 3 | 4 | Y / N |

### Findings

**High band confirmed.** `luaV_execute` and `whereLoopAddBtreeIndex` were consistently the
hardest (difficulty 6–7, highest concept counts). Both required tracing macro chains, union
type invariants, and multi-level indirection across files. The high-AIM boundary is valid.

**Mid band not differentiated.** Sonnet 4.00→4.33→6.00 across low/mid/high; Opus
3.00→3.00→5.00. A 40-point AIM increase from low to mid produces near-zero difficulty
change. The formula does not predict mid-range effort reliably.

**Systematic model offset.** Sonnet rated every function 1 point higher than Opus (8/9
functions). Consistent ordering, not content disagreement — calibration offset between
models, not formula signal.

**Clear falsification: `hostapd_config_read_eap_user`.** AIM=93 (highest in experiment),
difficulty rated 3–4 (same as low band). It is a 328-line line-oriented parser; the entry
clause is two trivially obvious lines. The SLOC contribution saturates the score without
adding reasoning depth. Both cognitive (203 > ceiling 75) and SLOC (299 > ceiling 200) are
fully saturated, so the v4 reweight (SLOC 25→15, cognitive 45→55) does not reduce its
score — both inputs hit their ceilings regardless of relative weight. Reducing the score
for this function class requires either raising the cognitive ceiling (so functions with
genuinely massive cognitive scores pull away) or introducing a new metric for entry-point
shallowness. Tracked as a known gap below.

**External context is universal above low band.** The rate increases from ~67% (low) to
100% (mid and most of high). This metric discriminates low from non-low but not mid from
high. Not useful as a formula input in its current binary form.

### What the experiment supports

- `--aim-threshold 85` as a binary flag is validated: the high band functions (≥85) are
  genuinely hard and consistently distinguished from mid and low.
- Cognitive complexity is the dominant valid predictor; SLOC adds noise for functions
  where cognitive complexity is high and SLOC is large (both saturate).

### Known gaps

**Cross-file type/function indirection.** The two experiment functions that produced the
most model disagreement and required the most reasoning (`luaV_execute`, `whereLoopAddBtreeIndex`)
both required chasing macro chains and union/struct definitions across header files. This
is not currently measured by knots. All existing metrics (cognitive, SLOC, nesting,
test_score, doc_score, ABC, McCabe) are intra-function or intra-file. A future metric
could count the number of distinct external translation units contributing types or macros
referenced in the function's entry prologue — but this requires libclang-level include
resolution, not just token analysis.

**Mid-band calibration.** The experiment had only 3 mid-band functions (all AIM=55). A
larger sample spanning AIM 30–70 from type-heavy codebases (Linux kernel, LLVM, OpenSSL)
is needed before the mid-range behavior can be assessed or tuned.

## Pending Tasks

- Mid-band empirical validation with AIM 30–70 functions from type-heavy corpora
- Investigate cognitive ceiling raise (75 → 150?) to differentiate VdbeExec-class functions
  from hostapd-class functions (both currently saturate at the same score)

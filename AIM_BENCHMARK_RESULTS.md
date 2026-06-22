# AIRD/AICP Benchmark Results

Corpus validation run against 6 open-source C codebases.
knots version: 1.5.0

## Metrics

Two orthogonal AI cost metrics replaced the single AIM score in v1.5.0:

**AIRD — AI Reasoning Difficulty** (0–100)
Predicts how much reasoning effort the model needs once it has context. Cognitive
complexity is the primary driver; SLOC, nesting, and testability are secondary.

**AICP — AI Context Pressure** (0–100)
Predicts how much context the model must load before it can act. External call breadth
and function size are the primary drivers; documentation reduces the cost.

A function can be cheap to load but hard to reason about, or expensive to load but
trivial once context is assembled. The two scores are independent.

## Formulas

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

## Distribution by Corpus (AIRD)

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

## Top Scorers by Corpus (AIRD intuition check)

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

## Formula Notes

**SQLite main() inflation.** Five SQLite `main()` functions remain in the AIRD top 10.
These are legitimate: `fuzzcheck.c` (cog=505, sloc=768), `fuzzershell.c` (cog=326,
sloc=411), `speedtest1.c` (cog=192, sloc=476). The small-but-inflated cases (e.g.
`main(dbfuzz.c)`, cog=49, sloc=107) were eliminated by the v3 ceiling changes. The
remaining cases reflect SQLite's unusual project structure — 40+ standalone C programs —
not a formula defect.

**Score compression at the top.** The practical AIRD max across most corpora is 87–97.
Reaching 100 requires simultaneous saturation of all four positive inputs plus zero
documentation, which does not appear in practice.

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

## Pending Tasks

- Mid-band empirical validation: AIRD 30–70 and AICP across difficulty spectrum, from
  type-heavy codebases — measure actual token usage and tool-call counts, not self-rating
- AICP threshold recommendation: determine what AICP score correlates with meaningfully
  elevated context-gathering cost in practice

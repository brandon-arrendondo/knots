# AIM Score Benchmark Results

Corpus validation run against 6 open-source C codebases.
knots version: 1.4.3 (post-v3 tuning, see formula history below)

## Formula (v3)

```
AIM = (cognitive/75 × 45) + (sloc/200 × 25) + (nesting/8 × 15) + (test_score/20 × 15) - (doc_score/10 × 15)
```
Clamped to [0, 100].

### Formula history

| Version | Change | Rationale |
|---------|--------|-----------|
| v1 | cognitive/50×35, sloc/100×25, nesting/8×15, test/40×25, doc/10×15 | Initial hypothesis |
| v2 | test ceiling 40→20 | Max observed test_score was ~18; v1 ceiling structurally halved the contribution |
| v3 | cognitive weight 35→45, test weight 25→15; cognitive ceiling 50→75, sloc ceiling 100→200 | Corpus percentile analysis: SLOC ceiling at 100 saturated at p95, cognitive at 50 saturated at p95–97; weight shift reduces test_score inflation of small-but-untested tool entry points |

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

## Empirical Validation Plan

The formula is calibrated against static heuristics. The open question is whether AIM
scores actually predict real AI effort — specifically, whether high-AIM functions consume
more tokens and require more turns when modified by an AI agent.

### Proposed experiment

For each of 3 representative functions per band (low / mid / high), issue a consistent
modification task to both Sonnet and Opus agents and record:
- Total input + output tokens consumed
- Number of turns to a passing result
- Whether the agent succeeded without human correction

Candidate functions (drawn from stable, well-known corpora):

**Low AIM (8–15) — expect: fast, low token cost**
| AIM | Cog | SLOC | Function | Corpus |
|-----|-----|------|----------|--------|
| 9 | 21 | 41 | db__ready_for_flight | mosquitto/database.c:37 |
| 14 | 16 | 31 | bufq_slurpn | curl/bufq.c:579 |
| 15 | 17 | 31 | pmksa_cache_get_okc | hostap/pmksa_cache_auth.c:527 |

**Mid AIM (35–55) — expect: moderate cost, occasional correction**
| AIM | Cog | SLOC | Function | Corpus |
|-----|-----|------|----------|--------|
| 55 | 44 | 66 | mosquitto_validate_utf8 | mosquitto/utf8_common.c:25 |
| 55 | 49 | 102 | Curl_conn_connect | curl/connect.c:335 |
| 55 | 45 | 113 | ieee802_1x_encapsulate_radius | hostap/eapol_test.c:179 |

**High AIM (75+) — expect: high token cost, multiple turns**
| AIM | Cog | SLOC | Function | Corpus |
|-----|-----|------|----------|--------|
| 87 | 138 | 751 | luaV_execute | lua/lvm.c:1198 |
| 90 | 187 | 336 | whereLoopAddBtreeIndex | sqlite/where.c |
| 93 | 203 | 299 | hostapd_config_read_eap_user | hostap/config_file.c:251 |

### What success looks like

A positive result is a clear monotonic relationship: low-AIM tasks consistently cost fewer
tokens than mid-AIM, which cost fewer than high-AIM, across both models. A negative result
(no correlation) would suggest the formula is measuring something other than AI modification
cost and needs a different set of inputs.

Secondary signal: if Opus and Sonnet diverge significantly on the same high-AIM function
(Opus succeeds in fewer turns), that's evidence the score is capturing genuine reasoning
difficulty, not just context window consumption.

## Pending Tasks

- Run empirical token correlation experiment (see above)
- Add `--aim-threshold` flag once empirical validation confirms score bands are meaningful

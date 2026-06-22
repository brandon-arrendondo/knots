# AIM Score Benchmark Results

Initial corpus validation run against 6 open-source C codebases.
knots version: 1.4.3 (commit 8102bc8)

## Formula (v1)

```
AIM = (cognitive/50 × 35) + (sloc/100 × 25) + (nesting/8 × 15) + (test_score/40 × 25) - (doc_score/10 × 15)
```
Clamped to [0, 100].

## Distribution by Corpus

| Corpus   | Functions | Avg AIM | Max AIM | 0–10 | 11–25 | 26–50 | 51–75 | 76–100 |
|----------|-----------|---------|---------|------|-------|-------|-------|--------|
| lua      | 1,304     | 6.7     | 76      | 81%  | 16%   | 3%    | 0%    | 0%     |
| libcrc   | 34        | 10.4    | 73      | 68%  | 24%   | 3%    | 6%    | 0%     |
| mosquitto| 3,324     | 10.8    | 84      | 69%  | 20%   | 7%    | 3%    | 1%     |
| sqlite   | 9,491     | 12.9    | 86      | 64%  | 20%   | 9%    | 5%    | 1%     |
| hostap   | 13,525    | 14.1    | 83      | 57%  | 27%   | 11%   | 5%    | 0%     |
| curl     | 5,486     | 15.7    | 83      | 53%  | 27%   | 13%   | 5%    | 1%     |

## Top AIM Scorers (SQLite, intuition check)

| Function | File | McCabe | Cognitive | SLOC | Nesting | TestScore | AIM |
|----------|------|--------|-----------|------|---------|-----------|-----|
| main (fuzzcheck) | test/fuzzcheck.c | 202 | 505 | 768 | 40 | 18 | 86 |
| sqlite3VdbeExec | src/vdbe.c | 1278 | 3023 | 4977 | 11 | 11 | 79 |
| DbObjCmd | src/tclsqlite.c | 274 | 631 | 1391 | 8 | 10 | 78 |
| sqlite3Pragma | src/pragma.c | 372 | 1014 | 1643 | 8 | 7 | 76 |
| sqlite3WhereCodeOneLoopStart | src/wherecode.c | 235 | 490 | 964 | 8 | 7 | 76 |
| sqlite3_str_vappendf | src/printf.c | 246 | 730 | 848 | 8 | 7 | 76 |

Intuition check passes — these are well-known hard-to-modify functions.

## Findings & Tuning Notes

### What's working
- Distribution is heavily right-skewed (most functions score low) — correct for healthy codebases
- Top 1% scorers are genuinely the hardest functions by human intuition
- Scale differentiates well between trivial (0–10), moderate (11–25), and complex (26+)

### Identified tuning issue
**test_score saturation ceiling too high.** Max observed test_score across all corpora
is ~18, but the ceiling is 40. This means `test_norm` never exceeds ~0.45 in practice,
structurally cutting the test_score contribution in half.

Recommendation: lower ceiling from 40 → 20 to use the full range of the weight.

### Max AIM cap
Practical max observed is 86 (not 100). To reach 100 requires all four inputs to
saturate AND zero documentation. Reaching 86 requires nesting=40 (extreme outlier),
with cognitive/sloc saturated and modest test_score. The cap behavior is intentional
but worth revisiting after the test_score ceiling fix.

## Pending Tasks
- Lower test_score saturation ceiling: 40 → 20 (see tuning note above)
- Add --format json / --format csv for easier future corpus analysis
- Add --aim-threshold flag once ceiling/weight tuning is validated

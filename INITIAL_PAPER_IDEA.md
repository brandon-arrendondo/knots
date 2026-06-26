# AI Code Complexity Metrics: Empirical Design Notes

**Status**: Research notes — seed for a future paper. Migrate to LaTeX when ready to submit.

---

## 1. Motivation

Traditional complexity metrics (McCabe cyclomatic complexity, Cognitive Complexity) were designed to model **human** cognitive load. They answer: *how hard is it for a programmer to read this function?*

LLM-assisted development introduces a different cost model. When a developer asks an AI assistant to modify a function, the cost has two orthogonal components:

1. **Reasoning depth** — how much cognitive effort the model needs to analyze the function, predict side effects, and generate a correct edit. This is roughly analogous to human cognitive load, but the penalty curve is different: models do not fatigue, but they lose coherence under token pressure.

2. **Context pressure** — how many tokens the model must gather from *outside* the function before it can act. A one-line function that calls 15 external APIs is cheap to reason about but expensive to contextualize.

Human cognitive load metrics conflate these two dimensions. A 300-line function with a simple linear structure is cognitively cheap to a human (just read top to bottom) but has high context pressure for a model (300 lines of token budget consumed before any external calls are loaded). Conversely, a 20-line function with 12 external calls is cognitively light but forces the model to chase 12 dependency chains.

The hypothesis: separating these dimensions into two independent metrics — **AIRD** (reasoning difficulty) and **AICP** (context pressure) — gives practitioners more actionable signals than any single composite score.

---

## 2. The Metrics

### 2.1 AIRD — AI Reasoning Difficulty

Normalized 0–100 score. Predicts how much *reasoning effort* an AI model needs to safely modify a function.

```
AIRD = (cognitive/75 × 55)
     + (sloc/200    × 15)
     + (nesting/8   × 15)
     + (test/20     × 15)
     - (doc/10      × 15)
     + (coupling/12 × 10)
```

**Term rationale:**

| Term | Weight | Rationale |
|---|---|---|
| Cognitive complexity | 55 | Dominant driver. Structural branching and nesting are the primary sources of reasoning difficulty for both humans and models. |
| SLOC | 15 | Length is a secondary signal. Long functions require more tokens; increases the chance of context truncation. |
| Nesting depth | 15 | Deeply nested code requires the model to maintain more stack state while generating edits. |
| Test score | 15 | Untestable functions are harder to modify safely — no automated verification available to the model. |
| Documentation | −15 | Well-documented functions give the model accurate priors; reduces guessing at intent. |
| State coupling | +10 | Functions that touch many distinct state variables (parameters + `self.fields`) require more context to modify without breaking invariants. Dampens over-credit from mechanical splits. |

**Ceiling values** (p99 of 32,205-function corpus: mosquitto, SQLite, curl, hostap, Lua, libcrc):

| Input | p99 ceiling |
|---|---|
| Cognitive complexity | 75 |
| SLOC | 200 |
| Nesting depth | 8 |

**Recommended CI threshold**: 85. Validated empirically against Sonnet 4.6 and Opus 4.8.

**Distribution**: heavily right-skewed. In mature codebases 67–88% of functions score ≤10. The ≥76 band accounts for 1–2% of all functions.

### 2.2 AICP — AI Context Pressure

Normalized 0–100 score. Predicts how many tokens the model must load from *outside* the function before it can act.

```
AICP = (external_calls/20 × 60)
     + (sloc/200          × 40)
     - (doc/10            × 15)
```

External call breadth is the dominant driver (60%). A function that calls 20 distinct external APIs forces the model to resolve 20 dependency chains. SLOC contributes as a secondary pressure (40%) — longer functions consume more of the available context window.

**p99 ceiling for external calls**: 20 (consistent across all 6 corpora).

---

## 3. Corpus and Validation

### 3.1 Calibration corpus

Six production C/C++ codebases, 32,205 functions total:

- **mosquitto** — MQTT broker
- **SQLite** — embedded relational database
- **curl** — HTTP/transfer library
- **hostap** — WPA supplicant / AP
- **Lua** — scripting language runtime
- **libcrc** — CRC algorithm collection

Selected to span a range of complexity profiles: systems code (SQLite, curl), protocol code (hostap, mosquitto), scripting runtime (Lua), and simple utility (libcrc).

### 3.2 McCabe validation

100% match with `pmccabe` output across the full 32,205-function corpus.

### 3.3 Cognitive complexity validation

Validated against Mozilla's `rust-code-analysis` at 1.004× mean ratio across 11,365 Rust functions (285k lines).

### 3.4 AIRD threshold validation

Functions scoring ≥85 were consistently rated significantly harder to modify by Sonnet 4.6 and Opus 4.8 in structured evaluations. No formal benchmark yet — this is an empirical observation from real refactoring sessions.

---

## 4. State Coupling: Design History

### 4.1 The original gap (v1.9.0 observation)

When knots was applied to `tools_sqc` (a 10,000+ line Rust static analysis tool), a pattern emerged: functions with many explicit parameters scored high on raw-arity metrics, but struct-wrapping the parameters into a context object trivially lowered the score without reducing the actual state touched.

Example: `fn analyze_if(ctx: &Context)` touches the same state as `fn report_conditional_leaks(a: T1, b: T2, c: T3, d: T4, e: T5, f: T6)` — it's just less honest about it. The god-struct anti-pattern can hide coupling, not remove it.

The target metric:

```
distinct_state = explicit_params + self_fields_accessed
```

### 4.2 Design constraint: dampen, don't invert (v1.10.0)

The coupling term must **dampen over-credit from mechanical splits**, not invert genuine wins. A function split from AIRD 92 to 12 via real deduplication should still score 12. Only mechanical relocation (splitting a flat N-arm match into N named helpers, touching the same state in aggregate) should receive reduced credit.

Calibration anchor: Clippy's `too_many_arguments` fires at 7 parameters, so 11-parameter functions should land at ~0.9 coupling_norm — meaningful dampening without hard reversal. The 12-normalization point satisfies this. Weight 10 (vs 55 for cognitive) ensures the coupling term influences but does not dominate.

**Test case group 1 (must be dampened):** `analyze_one_file` (11 params), `scan_call_expression` (8 + `&self`), `load_project_context` (8), `report_conditional_leaks` (6 + `&self`).

**Test case group 2 (genuine wins, must stay low after split):** `str31_violation` (4 params, killed ABC 152→79), `generate_subdir_tests` (−35 LOC via dedup), `test_func_name`/`rule_status` (dedup'd logic).

The formula separates these two groups correctly.

### 4.3 Validation oracle

`tools_sqc` commit range `52f66252..6356ccfa`: a 10-function refactor with behavior preservation verified by exact-match Juliet 0.4.69→0.4.70 benchmark (TP 22107, FP 4262, zero delta across all CWE categories).

---

## 5. Observed Failure Modes

### 5.1 State-coupling extraction paradox

*Observed during `funky` v1.10.0 refactor, 2026-06-26.*

The first three method extractions from a function at AIRD 90 caused AIRD to **increase** to 94. Root cause: the original function accessed many `self.fields` directly. After extraction, those accesses moved into helper methods, reducing the caller's `state_coupling` count and therefore reducing the coupling contribution to AIRD. But the cognitive complexity was still above the 75 cap at this stage (not enough extracted yet), so the cognitive term was unchanged. Net effect: coupling contribution fell, cognitive stayed at ceiling → AIRD rose.

The paradox resolves once enough is extracted to push cognitive below 75, at which point the dominant term drops sharply and AIRD falls.

**Implication for UX:** violation messages now include per-term breakdown and a tip when state_coupling > 0.

**Open question:** should the coupling term be subtractive (as a true dampener) rather than additive? A subtractive term would prevent this paradox, but the calibration tests (`test_high_arity_dampens_aird`) require additive behavior to penalize genuinely high-arity functions. This is a fundamental tension in the design.

### 5.2 Dual-cap stagnation plateau

When both `cognitive ≥ 75` AND `sloc ≥ 200` simultaneously:

```
cognitive floor: 55.0  (capped)
sloc floor:      15.0  (capped)
nesting (typical): ~7.5
test (typical):     ~5.0
──────────────────────
floor before coupling: ~82.5
```

Any nonzero coupling pushes this past 85. No incremental extraction reduces either capped term until the function breaks through the cap boundary. The developer must commit to one large extraction push, not a series of small ones.

**Implication:** violation message tips now detect this condition and explicitly say "push for a larger extraction."

### 5.3 Normalization cap mismatch (signal loss at the top)

The 200-line SLOC cap and the 75-cognitive cap mean AIRD cannot distinguish between a function at cognitive=80 and one at cognitive=400. Both score 55 on that term. This is correct for the primary use case (CI gate) — both need refactoring. But for tracking *progress* through a refactor, the signal is lost once you're above both caps. The `[capped]` readout in violation messages partially addresses this.

**Open question:** a two-pass formula — one for CI gating, one for progress tracking — might serve both use cases better. Or a separate "refactor progress score" that lifts the caps.

### 5.4 Inter-function coupling blind spot

AIRD scores functions in isolation. A split that lowers per-function AIRD can raise the cost of cross-function edits (e.g., a flat N-arm match refactored into N named helpers — understanding the full flow now requires reading N+1 functions instead of one).

This is a deliberate design choice (optimize the common case: understand/modify one function), but it should be stated explicitly in the paper. The metric answers "how hard is this function to modify?" not "how hard is this module to maintain?"

**Signal for over-splitting:** `#[allow(clippy::too_many_arguments)]` appearing during a knots-driven split is a stop signal — the coupling was real and has been hidden, not removed.

---

## 6. What Worked Well

- **Threshold at 85 is well-calibrated.** In the `tools_sqc` integration, all 10 flagged functions clustered at AIRD 86–92. Nothing in the 60–84 band was flagged. The threshold has not produced false positives in observed use.

- **Targeting accuracy.** All flagged functions were independently the hardest to read (`process_call` 199 SLOC, `analyze_node` 258 SLOC, `check_node` 16-arm match). Knots surfaces the right functions.

- **Behavior preservation.** All knots-driven refactors verified by Juliet benchmark with exact zero delta. The tool is a smoke detector, not a design guide — and it works as one.

- **State coupling dampening is net positive.** In practice the `format()` dispatch loop in `funky`, even after heavy extraction, retained enough `self.field` delegation calls to earn meaningful dampening. The 10-point max cap prevents pathological gaming (a function that does nothing but access 100 fields).

- **AIRD correctly identified `next_token` and `Fmt::format` as genuine extraction candidates.** The resulting code is more maintainable regardless of what the metric says post-refactor.

---

## 7. Open Questions for the Paper

1. **Formula sign of coupling:** should state coupling be additive (penalizes high-arity functions) or subtractive (dampens mechanical splits)? Currently additive, but this creates the extraction paradox. A bifurcated formula (additive for explicit params, subtractive for `self.fields`) may resolve the tension.

2. **Empirical LLM benchmark:** the AIRD/AICP thresholds are empirically informed but not formally benchmarked against LLM edit quality. A structured study (same edit tasks across low/mid/high AIRD functions, measured by correctness and first-pass success rate) would strengthen the paper significantly.

3. **Cross-language calibration:** ceiling values (cognitive=75, sloc=200, nesting=8) were derived from a C/C++ corpus. Are they stable across Rust, Python, JavaScript? Early indicators suggest yes, but this needs measurement.

4. **AICP validation:** less empirically grounded than AIRD. The external-calls ceiling of 20 is consistent across all 6 corpora, but the 60/40 weighting between external calls and SLOC was not formally calibrated.

5. **Progress tracking vs. CI gating:** should there be two modes — a hard gate for CI and a softer "progress score" that lifts caps and shows directional movement?

6. **Match-dispatch false positives:** flat N-arm match/switch statements score high on McCabe and AIRD but are often easier to read than their AIRD suggests. A structural discount for flat switch/match (no nesting, no cross-arm state) might reduce refactor churn on this pattern.

---

## 8. Related Work

*(To be expanded — placeholders)*

- McCabe (1976): cyclomatic complexity
- G. Ann Campbell / SonarSource: Cognitive Complexity specification
- Mozilla rust-code-analysis: validated comparison target
- Halstead complexity measures
- ABC complexity (Fitzpatrick, 1997)
- Literature on LLM context window effects on code generation quality (TBD)
- Literature on AI-assisted refactoring tooling (TBD)

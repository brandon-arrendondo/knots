# ADR-0003: Every gate default has a recorded source or calibration

**Status:** Accepted, 2026-10-05

## Context

knots ships numbers that decide whether a build passes: the thresholds in the
`knots-strict` hooks, the thresholds knots gates its own code with, the AIRD
gate and AIRD's own weights and ceilings, and the advice in the documentation.
Several had no recorded reason. A default with no stated basis can't be
defended, can't be revisited when the counts change (ADR-0001 changed them),
and invites a reader to treat it as a published standard.

This ADR records, for each default, where it comes from. A source is a primary
document that states the number. A calibration is a measurement of how often
the default fires on code it was not fitted to. Where there is neither, the
default is marked **no basis** and the measurement says what it does.

## Decision

1. **A default is a source or a calibration, stated here.** A new default, or a
   changed one, adds its row to the table below in the same change.
2. **Calibrate on held-out code.** The AIRD ceilings were set on lua, libcrc,
   mosquitto, hostap, sqlite and curl, so those six cannot confirm them. The
   measurements below use corpora the ceilings never saw: mbedtls, valkey,
   raylib, sel4, pureftpd and ventoy (C), twelve pinned crates (Rust) and
   eleven pinned npm packages (JS/TS). The paper's six are reported beside
   them to show the drift since the ceilings were set.
3. **The method is a script, not a table.** `validation/default_calibration.py`
   produces every figure here. Re-run it when a counting rule changes.
4. **A change to the AIRD weights or ceilings changes the formula**, so the
   paper's pilots must be re-scored. It needs the maintainer's decision, not a
   calibration run's.

## Method

knots 1.18.0 at `d6037fc`, `--format ndjson --score-components`. Each C corpus
is scanned over the files its scope selects (`scope_include`/`scope_exclude`
in aurora-lint's `data/benchmark_repos.json`, the shipped product and not its
tests or vendored code), at the pinned commits in `validation/README.md` and
aurora-lint's `corpus-check`. The paper's six are also scanned unscoped, which
reproduces the 32,999 functions the paper reports. The Rust and JS/TS sets are
the pinned releases in `validation/README.md`, scanned at their authored source
(`src/` or `lib/`, no declaration files or bundles), so their function counts
differ from that README's whole-package counts.

| Set | Functions |
|-----|----------:|
| C, paper's six (unscoped) | 32,999 |
| C, held out (scoped) | 12,248 |
| Rust, twelve crates (`src/`) | 12,273 |
| JS/TS, eleven packages | 18,120 |

**Prior work.** Deriving a threshold from the distribution of a metric over a
set of benchmark systems is an established approach: Alves, Ypma and Visser,
"Deriving metric thresholds from benchmark data" (ICSM 2010,
doi:10.1109/ICSM.2010.5609747), and Oliveira, Valente and Lima, "Extracting
relative thresholds for source code metrics" (CSMR-WCRE 2014,
doi:10.1109/CSMR-WCRE.2014.6747177). The references and DOIs were checked in
Crossref; the papers themselves were not available to this work, so they are
cited for the approach only, not for any detail of their procedures. This
ADR's method is pooled, unweighted per-function percentiles over the corpora,
with no weighting by function size or per-system aggregation, and it reports
how often an existing default fires. It derives no new threshold except ABC
20.0, which is a percentile choice and is marked as such below.

Figures below are the share of functions over a threshold, pooled over the set,
with the range across its corpora. They describe how often a default fires on
these corpora. They are not a claim that the corpora are well written.

## The defaults

### Cyclomatic complexity (McCabe)

| Default | Where | Source or calibration |
|---------|-------|-----------------------|
| 10, "good" band | `metrics-reference.rst`, `knots-strict` | **Source.** McCabe 1976, IEEE TSE SE-2(4), section on use: "the particular upper bound that has been used for cyclomatic complexity is 10 which seems like a reasonable, but not magical, upper limit". |
| 11-20 moderate, 21+ high | `metrics-reference.rst` | **Source.** SEI, C4 Software Technology Reference Guide, CMU/SEI-97-HB-001 (Jan 1997), p. 146: "One such threshold set is as follows": 1-10 simple, 11-20 moderate risk, 21-50 high risk, over 50 untestable. SEI offers it as one set, not a derived result. knots' docs merge 21-50 and over 50. |
| 15, self-check | `.pre-commit-config.yaml` | **Source, conditional.** NIST SP 500-235 (Watson and McCabe 1996) section 2.5: limits "as high as 15 have been used successfully", but "limits over 10 should be reserved for projects that have several operational advantages", such as experienced staff, formal design and a comprehensive test plan. **Calibration:** over 15 on 1.2% of Rust, the language knots is written in (0.0-3.0% across crates). |
| 20, CI examples | `ci-integration.rst` | **Calibration.** Over 20 on 0.7% of Rust, 2.6% of JS/TS, 4.0% of held-out C. No source for 20 itself. |

**The self-check keeps 15 under NIST's condition, by the maintainer's ruling.**
NIST allows a limit above 10 for projects with operational advantages, such as
experienced staff. The reasoning for knots: inexperienced teams are unlikely to
be using knots to begin with, and more experienced teams, or ones using LLMs,
can manage the added complexity. That is a judgment about who knots' users are,
not a measurement, and it is recorded as one.

Both McCabe and NIST exempt a module that is one multiway `switch` from the
limit. knots counts each non-default `case` (ADR-0001, McCabe's definition) and
applies no exemption, so a McCabe gate flags dispatch tables. (McCabe 1976
and NIST SP 500-235 both exempt a module that is a single multiway decision.) McCabe's count
is over 10 on 11.6% of held-out C functions and 2.1% of Rust; part of that gap
may be large `switch` bodies, which this measurement does not separate. See the
Ada paragraph in `metrics-reference.rst`.

### Cognitive Complexity

| Default | Where | Source or calibration |
|---------|-------|-----------------------|
| 15, self-check and CI examples | `.pre-commit-config.yaml`, `ci-integration.rst` | **Source.** SonarSource's rule S3776 ships a default threshold of 15 (`DEFAULT_MAX = 15` in sonar-java's `CognitiveComplexityMethodCheck`). The default belongs to that rule, not to the Cognitive Complexity specification. **Calibration:** over 15 on 0.9% of Rust, 6.3% of JS/TS, 9.7% of held-out C. |
| 25, React advice | `metrics-reference.rst`, `ci-integration.rst` | **Calibration.** Over 25 on 3.7% of JS/TS (0.6-6.3% across packages). No source. The set has no JSX or TSX corpus, so the claim that React components need a different gate is not measured here. |
| 10, `knots-strict` | `.pre-commit-hooks.yaml` | **Calibration.** Over 10 on 17.7% of paper-six C, 14.7% of held-out C, 1.7% of Rust, 9.2% of JS/TS. |

### The other self-check and strict values

None has a source that this project has found. The measurements:

| Default | Over it: C held-out | Rust | JS/TS | Reading |
|---------|-------------------:|-----:|------:|---------|
| nesting 5 (self-check) | 1.5% | 0.2% | 1.4% | top ~1-2% |
| SLOC 50 (self-check) | 9.6% | 2.2% | 2.2% | top ~2% outside C |
| returns 3 (self-check) | 12.2% | 0.6% | 8.0% | top ~1% of Rust |
| ABC 10.0 (self-check, was) | 27.6% | 7.7% | 13.5% | **outlier**: ~8% of Rust; now 20.0 |
| McCabe 10 (strict) | 11.6% | 2.1% | 6.5% | |
| nesting 3 (strict) | 6.0% | 1.3% | 5.4% | |
| SLOC 30 (strict) | 19.6% | 5.0% | 5.6% | |
| ABC 5.0 (strict, was) | 48.7% | 19.3% | 28.0% | **failed half of C**; now 20.0 |
| **ABC 20.0** (self-check and strict, now) | 11.9% | 2.4% | 5.8% | 88th percentile of held-out C, 97.6th of Rust |
| returns 3 (strict) | 12.2% | 0.6% | 8.0% | |

On Rust, knots' own language, every self-check value except ABC fires on about
1-2% of functions. ABC 10.0 fires on 7.7%, so it is tighter than its
neighbours by a wide margin (it sits near the 92nd percentile of Rust where
the others sit near the 98th-99th). ABC magnitude grows with function size, so
one number cannot fit C (p90 = 22.2) and Rust (p90 = 8.2) alike.

The `knots-strict` hooks list C, C++, Rust, Python, JavaScript and TypeScript.
ABC 5.0 (the value before the ruling below) failed 48.7% of held-out C functions and 54.1% of the paper's six. A
gate that fails most functions in a language it claims to support is not
tight, it is off.

### The AIRD gate

| Default | Source or calibration |
|---------|-----------------------|
| 85 | **Calibration, original.** Chosen at the formula's fourth version because the 76-100 bucket held 1-2% of functions across the six calibration corpora. The nine-function pilot that followed did not validate it: one function falsified it, and the paper says so. **Calibration, now:** over 85 on 1.08% of paper-six C and 0.74% of held-out C, so it still selects the top ~1% of C. On Rust it selects 0.04% (5 of 12,273) and on JS/TS 0.15%. |

The gate means "the top percent of C functions". Outside C it almost never
fires, which `metrics-reference.rst` already says for JS/TS. Rust is the same
and the note should say so.

### AIRD weights and ceilings

Weights are 55 (Cognitive), 15 (SLOC), 15 (nesting), 15 (test difficulty), -15
(documentation) and 10 (state coupling). They have a history, not a source:
the formula went through four versions, each made after a calibration failure
(`knots_aird_paper`, section "Calibration"). The 55/15/15/15 split dates from
the fourth version. State coupling's weight of 10 is explained in
`complexity.rs`: it "dampens mechanical splits without inverting genuine
wins". The weights are a design choice and nothing measured here can confirm
or refute them. **No change proposed.**

Ceilings (the value at which a term saturates), as the percentile of
functions at or over each:

| Input | Ceiling | Paper-six C | Held-out C | Rust | JS/TS |
|-------|--------:|------------:|-----------:|-----:|------:|
| Cognitive | 75 | 1.21% | 1.02% | 0.02% | 0.84% |
| SLOC | 200 | 0.82% | 0.60% | 0.18% | 0.14% |
| Nesting | 8 | 0.53% | 0.59% | 0.06% | 0.41% |
| Test difficulty | 20 | 0.10% | 0.28% | 0.00% | 0.03% |
| State coupling | 12 | 0.20% | 0.11% | 0.18% | 0.53% |

Cognitive 75 and SLOC 200 are close to the 99th percentile of C, as the paper
and `metrics-reference.rst` say (held-out p99: Cognitive 75, SLOC 161). Two
corrections to what the documentation says:

- **Nesting 8 is not a p99.** p99 of nesting is 6 in every C set. 8 is
  roughly the 99.5th percentile. The paper says nesting "seldom approaches"
  its ceiling, which is the accurate statement.
- **The documented figures are stale.** `metrics-reference.rst` says the
  ceilings are the p99 over 32,205 functions. At this version the same six
  corpora have 32,999 functions, with p99 Cognitive 81, SLOC 180 and nesting 6.
  The counting fixes (ADR-0001) moved them. The paper acknowledges that its
  calibration has drifted.

**The ceilings are C-calibrated.** On Rust the 99th percentile of Cognitive is
14 and of SLOC 86, against ceilings of 75 and 200; on JS/TS it is 67 and 74.
AIRD therefore compresses Rust code into the bottom of its range (p99 AIRD is
29 on Rust, 65 on JS/TS, 82 on held-out C). Whether to give each language its
own ceilings is a change to the formula and is for the maintainer to decide.

## Rulings and open items

The maintainer ruled on these defaults on 2026-10-05.

1. **ABC is 20.0 in the self-check and in `knots-strict`** (was 10.0 and 5.0).
   **The ABC threshold has no published basis, and 20.0 is not a validated
   value.** It is a calibration: the 97.6th percentile of the pinned Rust set,
   and the 88th of the held-out C set. ABC magnitude grows with function size,
   so one number cannot suit every language (p90 is 22.2 in held-out C and 8.2
   in Rust). **Follow-up research is needed** to find a basis for any ABC
   threshold, or to give ABC no default. Until then 20.0 is a placeholder with
   a measured effect: over it on 11.9% of held-out C, 2.4% of Rust and 5.8% of
   JS/TS.
2. **McCabe 15 in the self-check stands**, under NIST's condition, as above.
3. **The AIRD weights and ceilings do not change.** The ceilings are
   calibrated on C. **They need tuning on non-C languages**: on Rust and JS/TS
   AIRD compresses into the bottom of its range, and the 85 gate almost never
   fires (above). Whether that means per-language ceilings or a within-language
   gate is a change to the formula, and the paper's pilots would need
   re-scoring.
4. **Documentation.** The stale "p99 over 32,205 functions" paragraph in
   `metrics-reference.rst` now gives the percentiles at this version and the
   nesting caveat, and its JS/TS calibration note covers Rust.

## Not covered

- `knots-test-complexity` defaults (`--threshold`, `--boundary-threshold`) have
  no recorded basis and are not measured here.
- AICP's weights and ceilings, and the file-level Ce multiplier.
- Ada. The old advice to gate Ada McCabe at a fixed number is no longer in the
  docs; the page tells the reader to derive the number from the project's own
  distribution, which needs no default. No Ada corpus was run here.
- A JSX/TSX corpus, so the React advice is unmeasured.
- clippy's default of 25 (`alternatives.rst`) was not checked against clippy's
  own documentation.

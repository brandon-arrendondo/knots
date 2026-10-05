# ADR-0001: A metric's published definition is the authority, not another tool

**Status:** Accepted, 2026-10-05

## Context

knots validated its McCabe count against pmccabe and its Cognitive
Complexity against rust-code-analysis, and published the agreement as
"matches exactly" and "1.004×". Both claims rested on the reference tool
being right. Each reference tool has its own departures from its metric's
definition: rust-code-analysis omits the Cognitive Complexity recursion
increment, and pmccabe keeps only the first `#if` arm. Agreeing with one
inherits its departures. Meanwhile knots added `goto` and `throw`/`raise` to
McCabe, which neither the definition nor pmccabe counts, and the "exact
match" stopped being true without any check noticing: 2,176 of 30,461 C
functions differed.

## Decision

1. **Each metric's published definition decides what knots counts.**
   McCabe 1976 for cyclomatic complexity, Campbell's SonarSource whitepaper
   for Cognitive Complexity, Fitzpatrick 1997 for ABC, Nejmeh 1988 for NPath.
   A reference implementation is evidence about the definition, not a
   substitute for it.
2. **Every departure from a definition is deliberate, named and documented**
   in `docs/metrics-reference.rst`, with its reason. A difference nobody
   chose is a bug.
3. **A published agreement figure names its corpus pins, the knots commit
   and the script that produced it** (`validation/`), and its sample size
   is the number of functions actually compared.
4. **Changing a counting rule re-runs the validations it touches**
   (`validation/compare_versions.py` against the previous release) before
   the figure stays in the docs. The changelog lists it under "Changed",
   because existing scores change meaning.

## Consequences

- **McCabe no longer counts `goto`, `throw` or `raise`.** None is a decision.
  Agreement with pmccabe on the six pinned C corpora rose from 92.9% to 97.7%.
  The remaining differences are documented (preprocessor arms, and statements
  inside macro arguments).
- **Two Cognitive Complexity departures are known and still open:** the
  missing recursion increment, and the flat +1 for `throw`/`raise`, which the
  specification does not give. Both feed AIRD's dominant term, so they are
  decided deliberately (fix or document), not as a side effect.
- This doesn't make agreement with other tools unimportant. It makes it
  evidence: a disagreement is investigated against the definition, and
  either side can turn out to be wrong.

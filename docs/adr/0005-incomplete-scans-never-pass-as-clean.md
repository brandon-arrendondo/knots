# ADR-0005: An incomplete scan is reported and never passes as clean

**Status:** Proposed, 2026-10-08. Target policy; implementation is separate.

## Context

Knots currently warns and skips unreadable or unparseable files in multi-file
collection. The run may still exit 0. A zero threshold result then describes
only the inputs that happened to finish, not the entire selected corpus.

Parsing, traversal and cross-file aggregation can also fail or reach work
limits. Losing one unit's result should not discard unrelated completed work,
but a partial score is not a measured zero. Neither passing a gate nor writing
a baseline should conceal that gap.

This ADR defines the target. It does not claim that knots already contains
panics, enforces budgets or exports incomplete-scan metadata. The current CLI
and exit statuses remain documented in the CLI reference until code lands.

## Decision

1. **Contain work at the smallest trustworthy unit.** Isolate file reading
   and parsing, and function/metric analysis where those results can be
   separated safely. A failure discards the affected result, not unrelated
   completed results. Report which stage, file and function or metric failed,
   with a source location when available.
2. **Cross-file gaps are explicit.** A failed file may remove imports or
   recursion facts used elsewhere. Identify the missing contribution and
   invalidate or mark dependent scores accordingly. Do not silently produce
   a complete-looking Ce multiplier or other derived score from partial facts.
3. **Input-dependent work is bounded.** Prefer deterministic step, iteration
   or depth budgets to timing alone. A reached limit that truncates required
   analysis is incomplete; it must not return a fabricated zero. Time limits
   or watchdogs may backstop work that checkpoints cannot contain. Budgets
   and defaults require evidence from knots inputs, not copied constants.
4. **Incomplete is never clean.** Selected inputs lost to reading, parsing,
   crashes or truncation are named on stderr and summarized. Machine-readable
   consumers receive an explicit completion signal; SARIF records an
   unsuccessful invocation and execution notification. Completed trustworthy
   measurements remain available, distinguishable from missing results.
5. **The exit status reports incompleteness without a threshold flag.** It is
   nonzero and distinguishable from an ordinary threshold violation, and
   takes precedence over gate results. The implementation must specify the
   status value and output contract before the CLI documentation changes.
6. **Snapshots cannot hide gaps.** Do not silently write or replace a baseline
   from an incomplete scan, or present a partial validation run as evidence
   of agreement. Incomplete reusable cross-file facts must not become a
   complete cache for a later run.

Files intentionally outside the selected language/file scope are not failures.
An empty source file or a valid file without functions is not incomplete just
because it yields no function records. Tree-sitter recovery nodes alone do
not prove failure: preserve the recorded recovery and configuration rules in
[ADR-0002](0002-a-name-is-not-a-function.md), and report actual lost analysis
rather than suppressing whole regions indiscriminately.

Documented approximations are not automatically runtime incompleteness. Keep
known departures recorded under [ADR-0001](0001-the-definition-is-the-authority.md).
Distinguish a deliberate defined calculation from an execution that did not
finish that calculation; any unsound or truncated fallback must be visible.

## Consequences

- The separate implementation needs failure injection, bounded-work tests and
  deterministic results across sequential and parallel runs. Containment does
  not excuse leaving the underlying defect unfixed.
- Consumer scripts must recognize incomplete scans as failure. Existing scripts
  checking every nonzero status need no clean-result exception.
- Define partial-record serialization, aggregation dependencies and baseline
  refusal before shipping the new status. Preserve existing formats where
  possible; do not silently replace integer scores with zero for failed work.
- No aurora rule-abandonment count, step budget or time limit is adopted here.
  Knots' units are metric/function/file analysis, not CERT rules.

Origin: adaptation of aurora-lint ADR-0017, “A crash or a runaway costs one rule
on one file, is always reported, and never passes as clean”.

Source record: [aurora-lint ADR](https://github.com/brandon-arrendondo/aurora-lint/blob/8ad31a842e9d02f97a6d4b3a8cbeef5a9264cf0b/docs/adr/0017-graceful-failure.md).

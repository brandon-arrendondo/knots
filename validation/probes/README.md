# Definition probes

Each file here is a tiny program with a known answer. Its header states the
value a metric's **published definition** gives, and cites the source:

```
// source: <definition and section>
// expect <function> <metric>=<value> [<metric>=<value> ...]
```

(`--` instead of `//` for Ada.) A dotted key such as `mccabe.modified=2`
records the value of a variant other tools use. It's informational and not
checked against knots.

`tests/probes.rs` runs knots on every probe. A probe passes when knots gives
the definition's value, or when the difference is a **recorded departure**
in `conformance.toml`, with its definition value, knots' value, a status and
a reason. The test also fails when a recorded departure no longer occurs, so
the list can't go stale.

The expected values are derived by hand from the definitions, never from
knots' output. Where possible a probe is a definition's own worked example
(`sum_of_primes.c` and `get_words.c` are the SonarSource whitepaper's).

Sources: McCabe, "A Complexity Measure", IEEE TSE SE-2(4), 1976; Myers, "An
Extension to the Cyclomatic Measure of Program Complexity", SIGPLAN Notices
12(10), 1977; G. A. Campbell, *Cognitive Complexity*, SonarSource
whitepaper, version 1.7, 29 August 2023; Park, *Software Size Measurement*,
CMU/SEI-92-TR-20, 1992.

Coverage today: C (14 probes) and Ada (5). Other languages follow the same
pattern.

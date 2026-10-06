#!/usr/bin/env python3
"""How much the pooled percentile behind a gate default depends on corpus size.

    abc_basis.py [--ext=.rs] [--metric=abc] KNOTS_BIN OUT.json NAME=DIR[:SCOPE] ...

Takes the same corpus arguments as `default_calibration.py` and, for one metric
(ABC magnitude by default), reports the value at each percentile under four
ways of combining the corpora:

- `pooled`: every function counts once. This is what
  `default_calibration.py` and the ABC 20.0 figure in
  `docs/adr/0003-gate-defaults-have-a-recorded-basis.md` use.
- `equal_corpus`: every corpus counts equally, however many functions it has
  (each function weighs 1/n of its corpus). The per-system normalization of
  Alves, Ypma and Visser (ICSM 2010, section VI-C) with unweighted functions.
- `alves_sloc`: Alves et al.'s own weighting: each function weighs its SLOC,
  normalized within its corpus so every corpus sums to 1 (VI-B, VI-C). A
  percentile here is a share of the code, not of the functions.
- `median_of_corpus`: the median of the per-corpus percentiles. Alves et al.
  argue against it (VII-B: at the top quantiles it rests on few points and
  moves as data is added); it is a labelled check, in the manner of Oliveira,
  Valente and Lima's median of per-system tail percentiles (CSMR-WCRE 2014).

It also reports each corpus' own percentile and the share of functions over
each candidate threshold under each weighting, so a number can be compared
with the default it would replace.
"""

import json
import statistics
import sys
from pathlib import Path

from default_calibration import C_EXT, FIELD, percentile, run_knots, select

QUANTILES = (50, 70, 80, 88, 90, 95)
CANDIDATES = (10.0, 20.0)


def weighted_quantile(pairs, q):
    """Smallest value whose cumulative weight reaches q% of the total."""
    pairs = sorted(pairs)
    total = sum(w for _, w in pairs)
    need, run = total * q / 100, 0.0
    for v, w in pairs:
        run += w
        if run >= need - 1e-12:
            return v
    return pairs[-1][0]


def weighted_share_over(pairs, t):
    total = sum(w for _, w in pairs)
    return sum(w for v, w in pairs if v > t) / total


def weights(corpora, how):
    """(value, weight) per function over every corpus."""
    out = []
    for fns in corpora.values():
        n = len(fns)
        sloc_total = sum(s for _, s in fns) or 1
        for v, s in fns:
            if how == "pooled":
                out.append((v, 1.0))
            elif how == "equal_corpus":
                out.append((v, 1.0 / n))
            elif how == "alves_sloc":
                out.append((v, s / sloc_total))
    return out


def main(argv):
    exts, metric, args = C_EXT, "abc", []
    for a in argv:
        if a.startswith("--ext="):
            exts = tuple(a[6:].split(","))
        elif a.startswith("--metric="):
            metric = a[9:]
        else:
            args.append(a)
    if len(args) < 3:
        sys.exit(__doc__)
    binary, out_path, specs = args[0], args[1], args[2:]
    key = FIELD.get(metric, metric)

    corpora, files = {}, {}
    for spec in specs:
        name, _, rest = spec.partition("=")
        directory, _, scope = rest.partition(":")
        fl = select(Path(directory), scope, exts)
        recs = run_knots(binary, fl)
        corpora[name] = [(r[key], r.get("sloc") or 0) for r in recs if r.get(key) is not None]
        files[name] = len(fl)
        print(f"{name}: {len(fl)} files, {len(corpora[name])} functions", file=sys.stderr)

    result = {"metric": metric, "quantiles": QUANTILES, "corpora": {}, "combined": {}}
    for name, fns in corpora.items():
        vals = sorted(v for v, _ in fns)
        result["corpora"][name] = {
            "files": files[name], "functions": len(fns),
            "sloc": sum(s for _, s in fns),
            **{f"p{q}": percentile(vals, q) for q in QUANTILES},
            **{f"share_over_{t}": sum(v > t for v in vals) / len(vals) for t in CANDIDATES},
        }
    for how in ("pooled", "equal_corpus", "alves_sloc"):
        pairs = weights(corpora, how)
        result["combined"][how] = {
            **{f"p{q}": weighted_quantile(pairs, q) for q in QUANTILES},
            **{f"share_over_{t}": weighted_share_over(pairs, t) for t in CANDIDATES},
        }
    per = {q: [c[f"p{q}"] for c in result["corpora"].values()] for q in QUANTILES}
    result["combined"]["median_of_corpus"] = {f"p{q}": statistics.median(v) for q, v in per.items()}
    result["corpus_range"] = {f"p{q}": [min(v), max(v)] for q, v in per.items()}
    Path(out_path).write_text(json.dumps(result, indent=1))


if __name__ == "__main__":
    main(sys.argv[1:])

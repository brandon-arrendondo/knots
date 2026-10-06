#!/usr/bin/env python3
"""Where each gate default sits on code it was not fitted to.

    default_calibration.py KNOTS_BIN OUT.json NAME=DIR[:GLOB,GLOB!EXCLUDE,...] ...

Runs KNOTS_BIN over each corpus (`--format ndjson --score-components`), keeping
the files each corpus' scope selects, and writes per-corpus distributions of
every metric that has a default: percentiles, and the share of functions over
each default threshold. Re-run it after a counting rule changes; the figures in
`docs/adr/0003-gate-defaults-have-a-recorded-basis.md` come from it.

A scope is `GLOB,GLOB!EXCLUDE,EXCLUDE` relative to DIR; `**` matches any depth.
No scope keeps every file the extension filter finds. Pass `--ext=.rs` (before
the corpora) for another language.
"""

import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path

# The defaults under test, as knots ships them (src/complexity.rs,
# .pre-commit-config.yaml, docs/metrics-reference.rst).
THRESHOLDS = {
    # Each value is a default somewhere in knots: the hooks (`knots-strict`:
    # 10/10/3/30/5.0/3), the self-check (15/15/5/50/10.0/3), and the advice in
    # docs/ (McCabe 10/20, cognitive 25 for React).
    "mccabe": (10, 15, 20),
    "cognitive": (10, 15, 25),
    "nesting": (3, 5),
    "sloc": (30, 50),
    "abc": (5.0, 10.0),
    "return_count": (3,),
}
# AIRD's per-input ceilings (the value at which a term saturates).
CEILINGS = {"cognitive": 75, "sloc": 200, "nesting": 8, "test_score": 20,
            "doc_score": 10, "state_coupling": 12}
# knots' JSON names the ABC magnitude `abc_magnitude`; the rest match.
FIELD = {"abc": "abc_magnitude"}
PERCENTILES = (50, 75, 90, 95, 98, 99, 99.9)
C_EXT = (".c", ".h", ".cc", ".cpp", ".cxx", ".hpp", ".hh")


def glob_re(pattern):
    """fnmatch with `**` crossing directories and `*` not."""
    out, i = [], 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif pattern[i] == "*":
            out.append("[^/]*")
            i += 1
        elif pattern[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(pattern[i]))
            i += 1
    return re.compile("".join(out) + r"\Z")


def select(root, scope, exts):
    include, _, exclude = scope.partition("!")
    inc = [glob_re(g) for g in include.split(",") if g]
    exc = [glob_re(g) for g in exclude.split(",") if g]
    files = []
    for p in sorted(root.rglob("*")):
        if not p.is_file() or p.suffix not in exts or ".git" in p.parts:
            continue
        rel = p.relative_to(root).as_posix()
        if (not inc or any(r.match(rel) for r in inc)) and not any(r.match(rel) for r in exc):
            files.append(str(p))
    return files


def run_knots(binary, files):
    """One record per function. Batches the file list to stay under ARG_MAX."""
    records = []
    for i in range(0, len(files), 400):
        proc = subprocess.run([binary, "--format", "ndjson", "--score-components",
                               *files[i:i + 400]],
                              capture_output=True, text=True)
        if proc.returncode not in (0, 1):  # 1 = a threshold failed; none is passed
            sys.exit(f"knots failed: {proc.stderr[-800:]}")
        records += [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]
    return records


def percentile(sorted_vals, q):
    if not sorted_vals:
        return None
    k = (len(sorted_vals) - 1) * q / 100
    lo, hi = int(k), min(int(k) + 1, len(sorted_vals) - 1)
    return sorted_vals[lo] + (sorted_vals[hi] - sorted_vals[lo]) * (k - lo)


def summarize(records):
    out = {"functions": len(records), "metrics": {}}
    fields = set(THRESHOLDS) | set(CEILINGS) | {"aird"}
    for f in sorted(fields):
        key = FIELD.get(f, f)
        vals = sorted(r[key] for r in records if r.get(key) is not None)
        if not vals:
            continue
        m = {"p%s" % q: percentile(vals, q) for q in PERCENTILES}
        m["max"] = vals[-1]
        for t in THRESHOLDS.get(f, ()):
            m[f"share_over_{t}"] = sum(v > t for v in vals) / len(vals)
        if f in CEILINGS:
            m["share_at_or_over_ceiling"] = sum(v >= CEILINGS[f] for v in vals) / len(vals)
        if f == "aird":
            m["share_over_85"] = sum(v > 85 for v in vals) / len(vals)
        out["metrics"][f] = m
    return out


def main(argv):
    exts = C_EXT
    args = []
    for a in argv:
        if a.startswith("--ext="):
            exts = tuple(a[6:].split(","))
        else:
            args.append(a)
    if len(args) < 3:
        sys.exit(__doc__)
    binary, out_path, corpora = args[0], args[1], args[2:]
    result, pooled = {}, []
    for spec in corpora:
        name, _, rest = spec.partition("=")
        directory, _, scope = rest.partition(":")
        files = select(Path(directory), scope, exts)
        records = run_knots(binary, files)
        result[name] = {"files": len(files), **summarize(records)}
        pooled += records
        print(f"{name}: {len(files)} files, {len(records)} functions", file=sys.stderr)
    result["POOLED"] = summarize(pooled)
    Path(out_path).write_text(json.dumps(result, indent=1))


if __name__ == "__main__":
    main(sys.argv[1:])

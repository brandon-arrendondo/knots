"""Per-function before/after comparison of two knots binaries on pinned corpora.

Usage: compare_versions.py [--ext=.rs,...] OLD_BIN NEW_BIN OUT.json CORPUS_DIR [CORPUS_DIR ...]

--ext picks the file extensions scanned (default: C and C++). A corpus that is
not a git checkout, such as an unpacked crates.io crate, is recorded by its
directory name, which carries the crate's version.

Use before changing a counting rule that feeds AIRD/AICP or a published figure:
it shows which metrics moved, by how much, and which functions changed band
or crossed the 85 gate.
"""

import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

EXTS = (".c", ".h", ".cc", ".cpp", ".cxx", ".hpp", ".hh")
METRICS = ("mccabe", "cognitive", "nesting", "sloc", "test_score", "aird", "aicp")


def band(a):
    return "0-10" if a <= 10 else "11-25" if a <= 25 else "26-50" if a <= 50 else "51-75" if a <= 75 else "76-100"


def run(binary, root, exts=EXTS):
    files = sorted(str(p.relative_to(root)) for p in root.rglob("*")
                   if p.is_file() and p.suffix in exts and ".git" not in p.parts)
    out = {}
    for i in range(0, len(files), 500):
        r = subprocess.run([binary, "--format", "ndjson", *files[i:i + 500]], cwd=root,
                           capture_output=True, text=True)
        for line in r.stdout.splitlines():
            if line.startswith("{"):
                d = json.loads(line)
                out[(d["file"], d["start_line"], d["function"])] = d
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--ext=")]
    exts = next((tuple(a[6:].split(",")) for a in sys.argv[1:] if a.startswith("--ext=")), EXTS)
    # Absolute, because each corpus is scanned with cwd set to its root.
    old, new = (str(Path(b).resolve()) if "/" in b else b for b in args[:2])
    out_path, roots = Path(args[2]), [Path(p) for p in args[3:]]
    ver = lambda b: subprocess.run([b, "--version"], capture_output=True, text=True).stdout.strip()
    report = {"old": ver(old), "new": ver(new), "corpora": {}}
    for root in roots:
        a, b = run(old, root, exts), run(new, root, exts)
        keys = sorted(set(a) & set(b))
        changed = {m: sum(1 for k in keys if a[k][m] != b[k][m]) for m in METRICS}
        aird_delta = Counter(b[k]["aird"] - a[k]["aird"] for k in keys if a[k]["aird"] != b[k]["aird"])
        bands = Counter(f"{band(a[k]['aird'])}->{band(b[k]['aird'])}" for k in keys
                        if band(a[k]["aird"]) != band(b[k]["aird"]))
        gate = [{"file": k[0], "line": k[1], "function": k[2], "old": a[k]["aird"], "new": b[k]["aird"]}
                for k in keys if (a[k]["aird"] >= 85) != (b[k]["aird"] >= 85)]
        commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True,
                                text=True).stdout.strip() if (root / ".git").exists() else root.name
        report["corpora"][root.name] = {"commit": commit, "functions": len(keys), "changed": changed,
                                        "aird_delta": dict(sorted(aird_delta.items())),
                                        "band_changes": dict(bands), "gate_85_crossings": gate}
        print(f"{root.name:10} n={len(keys):6} changed={changed} aird_delta={dict(sorted(aird_delta.items()))} "
              f"bands={dict(bands)} gate={len(gate)}")
    out_path.write_text(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()

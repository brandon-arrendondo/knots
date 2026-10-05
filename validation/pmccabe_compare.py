"""Compare knots' McCabe with pmccabe's, function by function, on pinned C corpora.

Usage: pmccabe_compare.py KNOTS_BIN OUT.json CORPUS_DIR [CORPUS_DIR ...]

pmccabe column 1 ("modified": a switch counts once) is the variant knots
claims to match. Functions are matched by (file, start line). Each
mismatching function is classified by the constructs it contains, so a
claimed "exact match" can be checked against what actually differs.
"""

import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

EXTS = {".c", ".h"}
PM = re.compile(r"^(\d+)\t(\d+)\t(\d+)\t(\d+)\t(\d+)\t(.+)\((\d+)\): (.+)$")


def files_of(root: Path) -> list[str]:
    return sorted(str(p.relative_to(root)) for p in root.rglob("*")
                  if p.is_file() and p.suffix in EXTS and ".git" not in p.parts)


def knots_rows(knots: str, root: Path, files: list[str]) -> dict:
    out = {}
    for i in range(0, len(files), 500):
        r = subprocess.run([knots, "--format", "ndjson", *files[i:i + 500]], cwd=root,
                           capture_output=True, text=True)
        for line in r.stdout.splitlines():
            if line.startswith("{"):
                d = json.loads(line)
                out[(str((root / d["file"]).resolve().relative_to(root.resolve())), d["start_line"])] = d
    return out


def pmccabe_rows(root: Path, files: list[str]) -> dict:
    out = {}
    for i in range(0, len(files), 500):
        r = subprocess.run(["pmccabe", *files[i:i + 500]], cwd=root, capture_output=True, text=True)
        for line in r.stdout.splitlines():
            m = PM.match(line)
            if m:
                out[(m.group(6), int(m.group(7)))] = {"modified": int(m.group(1)),
                                                      "traditional": int(m.group(2)),
                                                      "name": m.group(8)}
    return out


def constructs(src: str) -> list[str]:
    tags = []
    for name, pat in (("goto", r"\bgoto\b"), ("throw", r"\bthrow\b"), ("catch", r"\bcatch\b"),
                      ("preproc-if", r"^\s*#\s*(if|ifdef|ifndef|elif)\b"),
                      ("ternary", r"\?[^:]*:")):
        if re.search(pat, src, re.M):
            tags.append(name)
    return tags


def main():
    knots, out_path, roots = sys.argv[1], Path(sys.argv[2]), [Path(p).expanduser() for p in sys.argv[3:]]
    report = {"knots": subprocess.run([knots, "--version"], capture_output=True, text=True).stdout.strip(),
              "pmccabe": "pmccabe (Debian package; no --version flag)", "corpora": {}}
    for root in roots:
        commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True,
                                text=True).stdout.strip()
        files = files_of(root)
        k, p = knots_rows(knots, root, files), pmccabe_rows(root, files)
        matched = sorted(set(k) & set(p))
        diffs, by_tag = [], Counter()
        for key in matched:
            kv, pv = k[key]["mccabe"], p[key]["modified"]
            if kv != pv:
                lines = (root / key[0]).read_text(errors="ignore").splitlines()
                body = "\n".join(lines[key[1] - 1:k[key].get("end_line", key[1])])
                tags = constructs(body) or ["none-found"]
                by_tag[" + ".join(tags)] += 1
                diffs.append({"file": key[0], "line": key[1], "function": k[key]["function"],
                              "knots": kv, "pmccabe": pv, "delta": kv - pv, "constructs": tags})
        report["corpora"][root.name] = {
            "commit": commit, "knots_functions": len(k), "pmccabe_functions": len(p),
            "matched": len(matched), "equal": len(matched) - len(diffs), "differ": len(diffs),
            "differ_by_constructs": dict(by_tag.most_common()),
            "delta_histogram": dict(Counter(d["delta"] for d in diffs).most_common()),
            "diffs": diffs}
        c = report["corpora"][root.name]
        print(f"{root.name:10} matched={c['matched']:6} equal={c['equal']:6} differ={c['differ']:5} "
              f"top={list(c['differ_by_constructs'].items())[:4]}")
    out_path.write_text(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()

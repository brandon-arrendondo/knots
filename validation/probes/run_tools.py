"""Run every C probe through knots and the other tools that compute the same metrics.

Usage: run_tools.py KNOTS_BIN OUT.md [OUT.json]

Each probe's header gives the value the published definition assigns; this
puts each tool's value beside it, so the table shows where tools agree with
the definition, with each other, and with knots. Tools that aren't installed
are reported as absent, not skipped silently.
"""

import json
import re
import shutil
import subprocess
import sys
from datetime import date
from pathlib import Path

HERE = Path(__file__).resolve().parent
VENV_BIN = Path(sys.executable).parent


def tool(name):
    found = shutil.which(name) or (VENV_BIN / name if (VENV_BIN / name).exists() else None)
    return str(found) if found else None


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True).stdout


def expectations(path):
    out = []
    for line in path.read_text().splitlines():
        m = re.match(r"\s*(?://|--)\s*expect\s+(\S+)\s+(.*)", line)
        if m:
            for pair in m.group(2).split():
                metric, value = pair.split("=")
                out.append((m.group(1), metric, int(value)))
    return out


def knots(binary, path):
    rows = [json.loads(l) for l in run([binary, str(path), "--format", "ndjson"]).splitlines() if l.startswith("{")]
    return {r["function"]: {"mccabe": r["mccabe"], "cognitive": r["cognitive"], "sloc": r["sloc"]} for r in rows}


def pmccabe(path):
    out = {}
    for line in run(["pmccabe", str(path)]).splitlines():
        m = re.match(r"^(\d+)\t(\d+)\t\d+\t\d+\t\d+\t.+\(\d+\): (.+)$", line)
        if m:
            out[m.group(3)] = {"modified": int(m.group(1)), "traditional": int(m.group(2))}
    return out


def lizard(binary, path, modified):
    cmd = [binary, "--csv", str(path)] + (["-m"] if modified else [])
    out = {}
    for line in run(cmd).splitlines():
        cols = next(iter(__import__("csv").reader([line])))
        if len(cols) > 7 and cols[1].isdigit():
            out[cols[7]] = int(cols[1])
    return out


def clang_tidy(binary, path):
    config = "{CheckOptions: [{key: readability-function-cognitive-complexity.Threshold, value: 0}, " \
             "{key: readability-function-cognitive-complexity.DescribeBasicIncrements, value: false}]}"
    text = run([binary, str(path), "-checks=-*,readability-function-cognitive-complexity",
                f"-config={config}", "--", "-std=c11", "-DFEATURE_UNSET"])
    found = {m.group(1): int(m.group(2))
             for m in re.finditer(r"function '([^']+)' has cognitive complexity of (\d+)", text)}
    return found


def rust_code_analysis(binary, path):
    text = run([binary, "-m", "-p", str(path), "-O", "json"])
    out = {}

    def walk(space):
        if space.get("kind") == "function":
            m = space["metrics"]
            out[space["name"]] = {"cyclomatic": int(m["cyclomatic"]["sum"]), "cognitive": int(m["cognitive"]["sum"])}
        for child in space.get("spaces", []):
            walk(child)

    for line in text.splitlines():
        if line.strip().startswith("{"):
            walk(json.loads(line))
    return out


def gnatmetric(binary, path):
    """McCabe per subprogram from gnatmetric's XML, run on a copy in a temp dir."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        copy = Path(tmp) / path.name
        copy.write_text(path.read_text())
        xml = Path(tmp) / "out.xml"
        subprocess.run([binary, "--complexity-cyclomatic", "--generate-xml-output",
                        f"--xml-file-name={xml}", str(copy)], cwd=tmp, capture_output=True, text=True)
        text = xml.read_text() if xml.exists() else ""
    out = {}
    for unit in re.finditer(r'<unit name="([^"]+)"[^>]*>(.*?)</unit>', text, re.S):
        m = re.search(r'<metric name="cyclomatic_complexity">(\d+)<', unit.group(2))
        if m:
            out[unit.group(1)] = int(m.group(1))
    return out


def version(cmd):
    return (run(cmd).strip().splitlines() or ["?"])[0]


def main():
    knots_bin, out_md = sys.argv[1], Path(sys.argv[2])
    out_json = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    tools = {"gnatmetric": tool("gnatmetric") or (str(Path.home() / ".alire/bin/gnatmetric")
                                                  if (Path.home() / ".alire/bin/gnatmetric").exists() else None),
             "lizard": tool("lizard"), "clang-tidy": tool("clang-tidy"),
             "rust-code-analysis-cli": tool("rust-code-analysis-cli"), "pmccabe": tool("pmccabe")}
    commit = run(["git", "-C", str(HERE), "rev-parse", "--short", "HEAD"]).strip()
    versions = {"knots": f"{version([knots_bin, '--version'])} (built from {commit})",
                "lizard": tools["lizard"] and version([tools["lizard"], "--version"]),
                "clang-tidy": tools["clang-tidy"] and version([tools["clang-tidy"], "--version"]),
                "rust-code-analysis-cli": tools["rust-code-analysis-cli"]
                and version([tools["rust-code-analysis-cli"], "--version"]),
                "pmccabe": tools["pmccabe"] and "Debian package (no --version)",
                "gnatmetric": tools["gnatmetric"] and "gnatmetric (libadalang_tools 25.0.0, GNAT 14.2.1 via Alire)"}
    rows = []
    for path in sorted((HERE / "c").glob("*.c")):
        k = knots(knots_bin, path)
        p = pmccabe(path) if tools["pmccabe"] else {}
        lz = lizard(tools["lizard"], path, False) if tools["lizard"] else {}
        lzm = lizard(tools["lizard"], path, True) if tools["lizard"] else {}
        ct = clang_tidy(tools["clang-tidy"], path) if tools["clang-tidy"] else None
        rca = rust_code_analysis(tools["rust-code-analysis-cli"], path) if tools["rust-code-analysis-cli"] else {}
        for function, metric, want in expectations(path):
            if "." in metric:
                continue
            row = {"probe": path.name, "function": function, "metric": metric, "definition": want,
                   "knots": k.get(function, {}).get(metric)}
            if metric == "mccabe":
                row["pmccabe (modified)"] = p.get(function, {}).get("modified")
                row["pmccabe (traditional)"] = p.get(function, {}).get("traditional")
                row["lizard"] = lz.get(function)
                row["lizard -m"] = lzm.get(function)
                row["rust-code-analysis"] = rca.get(function, {}).get("cyclomatic")
            elif metric == "cognitive":
                row["clang-tidy"] = None if ct is None else ct.get(function, 0)
                row["rust-code-analysis"] = rca.get(function, {}).get("cognitive")
            rows.append(row)
    for path in sorted((HERE / "ada").glob("*.adb")):
        k = knots(knots_bin, path)
        g = gnatmetric(tools["gnatmetric"], path) if tools["gnatmetric"] else {}
        for function, metric, want in expectations(path):
            row = {"probe": path.name, "function": function, "metric": metric, "definition": want,
                   "knots": k.get(function, {}).get(metric), "language": "ada"}
            if metric == "mccabe":
                row["gnatmetric"] = g.get(function)
            rows.append(row)
    write_markdown(out_md, rows, versions)
    if out_json:
        out_json.write_text(json.dumps({"date": str(date.today()), "versions": versions, "rows": rows}, indent=1))


def cell(value, want):
    if value is None:
        return "–"
    return f"{value}" if value == want else f"**{value}**"


def write_markdown(path, rows, versions):
    lines = [f"# Probe results across tools ({date.today()})", "",
             "Generated by `run_tools.py`; do not edit by hand. A **bold** value differs from the",
             "definition. – means the tool doesn't report that metric for that function, or isn't installed.", "",
             "Tool versions: " + "; ".join(v if v.lower().startswith(k.split("-")[0]) else f"{k} {v}"
                                for k, v in versions.items() if v) + ".", ""]
    for lang, metric, cols in (("c", "mccabe", ["knots", "pmccabe (modified)", "pmccabe (traditional)", "lizard",
                                                "lizard -m", "rust-code-analysis"]),
                               ("c", "cognitive", ["knots", "clang-tidy", "rust-code-analysis"]),
                               ("ada", "mccabe", ["knots", "gnatmetric"]),
                               ("ada", "cognitive", ["knots"])):
        lines += [f"## {lang.upper() if lang == 'c' else 'Ada'}: {metric}", "",
                  "| probe | function | definition | " + " | ".join(cols) + " |",
                  "|---|---|---|" + "---|" * len(cols)]
        for r in (r for r in rows if r["metric"] == metric and r.get("language", "c") == lang):
            lines.append(f"| {r['probe']} | {r['function']} | {r['definition']} | "
                         + " | ".join(cell(r.get(c), r["definition"]) for c in cols) + " |")
        lines.append("")
    path.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()

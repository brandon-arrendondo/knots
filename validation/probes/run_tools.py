"""Run every probe through knots and the other tools that compute the same metrics.

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
import tempfile
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
        m = re.match(r"\s*(?://|--|#)\s*expect\s+(\S+)\s+(.*)", line)
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


def clippy(binary, path):
    """clippy's cognitive_complexity lint, threshold 0 so it reports every function.

    Informative only: the lint scores each function from 1 and isn't Campbell's
    algorithm (clippy documents it as a different, older measure).
    """
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "clippy.toml").write_text("cognitive-complexity-threshold = 0\n")
        r = subprocess.run([binary, "--edition", "2021", "--crate-type", "lib", "-A", "dead_code",
                            "-W", "clippy::cognitive_complexity", "--error-format=json", "--emit=metadata",
                            "-o", "/dev/null", str(path)], capture_output=True, text=True,
                           env={**__import__("os").environ, "CLIPPY_CONF_DIR": tmp})
    out = {}
    for line in r.stderr.splitlines():
        d = json.loads(line) if line.startswith("{") else {}
        m = re.search(r"cognitive complexity of \((\d+)/", d.get("message", ""))
        if m and d["spans"]:
            t = d["spans"][0]["text"][0]
            out[t["text"][t["highlight_start"] - 1:t["highlight_end"] - 1]] = int(m.group(1))
    return out


def rust_rows(tools, knots_bin):
    rows = []
    for path in sorted((HERE / "rust").glob("*.rs")):
        k = knots(knots_bin, path)
        lz = lizard(tools["lizard"], path, False) if tools["lizard"] else {}
        rca = rust_code_analysis(tools["rust-code-analysis-cli"], path) if tools["rust-code-analysis-cli"] else {}
        cl = clippy(tools["clippy-driver"], path) if tools["clippy-driver"] else {}
        for function, metric, want in expectations(path):
            row = {"probe": path.name, "function": function, "metric": metric, "definition": want,
                   "knots": k.get(function, {}).get(metric), "language": "rust"}
            if metric == "mccabe":
                row["lizard"] = lz.get(function)
                row["rust-code-analysis"] = rca.get(function, {}).get("cyclomatic")
            elif metric == "cognitive":
                row["rust-code-analysis"] = rca.get(function, {}).get("cognitive")
                row["clippy (not Campbell)"] = cl.get(function)
            rows.append(row)
    return rows


ESLINT_TOOLCHAIN = Path.home() / "toolchain/jsmetrics"
SLOC_TOOLCHAIN = Path.home() / "toolchain/sloctools"


def file_sloc(tools, path):
    """Code lines per whole-file counter. A SLOC probe holds one function and comments, so the file's count
    is the function's. sloccount reads a directory and skips duplicate files, so each probe gets its own."""
    out = {}
    if tools["sloccount"]:
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copy(path, tmp)
            for line in run([tools["sloccount"], "--details", tmp]).splitlines():
                parts = line.split()
                if parts and parts[0].isdigit() and parts[-1].endswith(path.name):
                    out["sloccount"] = int(parts[0])
    if tools["cloc"]:
        rows = run([tools["cloc"], "--quiet", "--csv", str(path)]).strip().splitlines()
        out["cloc"] = int(rows[-1].split(",")[4]) if len(rows) > 1 else None
    if tools["tokei"]:
        out["tokei"] = json.loads(run([tools["tokei"], "-o", "json", str(path)]))["Total"]["code"]
    if tools["scc"]:
        out["scc"] = sum(f["Code"] for f in json.loads(run([tools["scc"], "-f", "json", str(path)])))
    return out


def sloc_rows(tools, knots_bin):
    rows = []
    for path in sorted(HERE.glob("*/sloc*")):
        k = knots(knots_bin, path)
        counts = file_sloc(tools, path)
        for function, metric, want in expectations(path):
            rows.append({"probe": f"{path.parent.name}/{path.name}", "function": function, "metric": metric,
                         "definition": want, "knots": k.get(function, {}).get(metric), "language": "any",
                         **counts})
    return rows


def eslint(path, binary_dir, knots_bin):
    """ESLint `complexity` (classic) and sonarjs cognitive, keyed by function name via knots' start_line.
    sonarjs reports only functions scoring above 0, so a function knots finds and sonarjs omits scored 0."""
    rows = [json.loads(l) for l in run([knots_bin, str(path), "--format", "ndjson"]).splitlines()
            if l.startswith("{")]
    names = {r["start_line"]: r["function"] for r in rows}
    out = {r["function"]: {"cognitive": 0} for r in rows}
    for line in run(["node", str(HERE / "eslint_probe.cjs"), str(binary_dir), str(path)]).splitlines():
        found = json.loads(line)
        name = names.get(found["line"])
        if name:
            key = "mccabe" if found["rule"] == "complexity" else "cognitive"
            out.setdefault(name, {})[key] = found["value"]
    return out


def js_rows(tools, knots_bin):
    rows = []
    paths = sorted((HERE / "js").glob("*.js")) + sorted((HERE / "ts").glob("*.ts*"))
    for path in paths:
        k = knots(knots_bin, path)
        es = eslint(path, tools["eslint"], knots_bin) if tools["eslint"] else {}
        lz = lizard(tools["lizard"], path, False) if tools["lizard"] else {}
        rca = rust_code_analysis(tools["rust-code-analysis-cli"], path) if tools["rust-code-analysis-cli"] else {}
        for function, metric, want in expectations(path):
            row = {"probe": f"{path.parent.name}/{path.name}", "function": function, "metric": metric,
                   "definition": want, "knots": k.get(function, {}).get(metric), "language": "js"}
            if metric == "mccabe":
                row["eslint complexity"] = es.get(function, {}).get("mccabe")
                row["lizard"] = lz.get(function)
                row["rust-code-analysis"] = rca.get(function, {}).get("cyclomatic")
            elif metric == "cognitive":
                row["sonarjs"] = es.get(function, {}).get("cognitive")
                row["rust-code-analysis"] = rca.get(function, {}).get("cognitive")
            rows.append(row)
    return rows


def js_versions(toolchain):
    def of(name):
        return json.loads((toolchain / "node_modules" / name / "package.json").read_text())["version"]
    return (f"eslint {of('eslint')}, eslint-plugin-sonarjs {of('eslint-plugin-sonarjs')}, "
            f"typescript-eslint {of('typescript-eslint')}")


def version(cmd):
    return (run(cmd).strip().splitlines() or ["?"])[0]


def main():
    knots_bin, out_md = sys.argv[1], Path(sys.argv[2])
    out_json = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    tools = {"gnatmetric": tool("gnatmetric") or (str(Path.home() / ".alire/bin/gnatmetric")
                                                  if (Path.home() / ".alire/bin/gnatmetric").exists() else None),
             "lizard": tool("lizard"), "clang-tidy": tool("clang-tidy"),
             "rust-code-analysis-cli": tool("rust-code-analysis-cli"), "pmccabe": tool("pmccabe"),
             "clippy-driver": tool("clippy-driver"),
             "sloccount": tool("sloccount"), "cloc": tool("cloc"),
             "tokei": tool("tokei") or (str(SLOC_TOOLCHAIN / "tokei") if (SLOC_TOOLCHAIN / "tokei").exists() else None),
             "scc": tool("scc") or (str(SLOC_TOOLCHAIN / "scc") if (SLOC_TOOLCHAIN / "scc").exists() else None),
             "eslint": ESLINT_TOOLCHAIN if (ESLINT_TOOLCHAIN / "node_modules/eslint").exists() else None}
    commit = run(["git", "-C", str(HERE), "rev-parse", "--short", "HEAD"]).strip()
    versions = {"knots": f"{version([knots_bin, '--version'])} (built from {commit})",
                "lizard": tools["lizard"] and version([tools["lizard"], "--version"]),
                "clang-tidy": tools["clang-tidy"] and version([tools["clang-tidy"], "--version"]),
                "rust-code-analysis-cli": tools["rust-code-analysis-cli"]
                and version([tools["rust-code-analysis-cli"], "--version"]),
                "pmccabe": tools["pmccabe"] and "Debian package (no --version)",
                "gnatmetric": tools["gnatmetric"] and "gnatmetric (libadalang_tools 25.0.0, GNAT 14.2.1 via Alire)",
                "clippy-driver": tools["clippy-driver"] and version([tools["clippy-driver"], "--version"]),
                "eslint": tools["eslint"] and js_versions(tools["eslint"]),
                "sloccount": tools["sloccount"] and "sloccount 2.26 (Debian package)",
                "cloc": tools["cloc"] and f"cloc {version([tools['cloc'], '--version'])}",
                "tokei": tools["tokei"] and version([tools["tokei"], "--version"]).split(" compiled")[0],
                "scc": tools["scc"] and version([tools["scc"], "--version"])}
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
    rows += rust_rows(tools, knots_bin)
    rows += js_rows(tools, knots_bin)
    rows = [r for r in rows if r["metric"] != "sloc"] + sloc_rows(tools, knots_bin)
    write_markdown(out_md, rows, versions)
    if out_json:
        out_json.write_text(json.dumps({"date": str(date.today()), "versions": versions, "rows": rows}, indent=1))


LANGUAGE_NAMES = {"c": "C", "ada": "Ada", "rust": "Rust", "js": "JavaScript and TypeScript",
                  "any": "Every language"}


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
                               ("ada", "cognitive", ["knots"]),
                               ("rust", "mccabe", ["knots", "lizard", "rust-code-analysis"]),
                               ("rust", "cognitive", ["knots", "rust-code-analysis", "clippy (not Campbell)"]),

                               ("js", "mccabe", ["knots", "eslint complexity", "lizard", "rust-code-analysis"]),
                               ("js", "cognitive", ["knots", "sonarjs", "rust-code-analysis"]),
                               ("any", "sloc", ["knots", "sloccount", "cloc", "tokei", "scc"])):
        lines += [f"## {LANGUAGE_NAMES[lang]}: {metric}", "",
                  "| probe | function | definition | " + " | ".join(cols) + " |",
                  "|---|---|---|" + "---|" * len(cols)]
        for r in (r for r in rows if r["metric"] == metric and r.get("language", "c") == lang):
            lines.append(f"| {r['probe']} | {r['function']} | {r['definition']} | "
                         + " | ".join(cell(r.get(c), r["definition"]) for c in cols) + " |")
        lines.append("")
    path.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()

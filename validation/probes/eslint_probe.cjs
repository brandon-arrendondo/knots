// Run ESLint's `complexity` rule and eslint-plugin-sonarjs's cognitive-complexity
// (S3776) on one JavaScript or TypeScript file, with every threshold at 0, and
// print one JSON line per finding: {rule, line, value}. The line is the
// function's first line, which run_tools.py matches to knots' start_line.
//
// usage: node eslint_probe.cjs TOOLCHAIN_DIR FILE
// TOOLCHAIN_DIR holds node_modules with eslint, eslint-plugin-sonarjs and
// typescript-eslint (validation/probes/README.md says how to install them).

const path = require("path");
const [toolchain, file] = process.argv.slice(2);
const load = (name) => require(require.resolve(name, { paths: [toolchain] }));
const { Linter } = load("eslint");
const sonarjs = load("eslint-plugin-sonarjs");
const tseslint = load("typescript-eslint");

const typescript = /\.tsx?$/.test(file);
const config = {
    files: ["**/*.js", "**/*.ts", "**/*.tsx"],
    plugins: { sonarjs },
    languageOptions: {
        ecmaVersion: "latest",
        sourceType: "script",
        parserOptions: { ecmaFeatures: { jsx: true } },
        ...(typescript ? { parser: tseslint.parser } : {}),
    },
    rules: {
        complexity: ["error", { max: 0, variant: "classic" }],
        "sonarjs/cognitive-complexity": ["error", 0],
    },
};

const source = require("fs").readFileSync(file, "utf8");
const messages = new Linter().verify(source, config, { filename: path.basename(file) });
for (const m of messages) {
    if (m.fatal) {
        console.error(m.message);
        process.exitCode = 1;
        continue;
    }
    const value = m.message.match(/complexity of (\d+)|Complexity from (\d+)/i);
    if (value) {
        console.log(JSON.stringify({ rule: m.ruleId, line: m.line, value: Number(value[1] || value[2]) }));
    }
}

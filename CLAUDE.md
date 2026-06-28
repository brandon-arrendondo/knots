# knots — developer guide for Claude

knots is a multi-language complexity analyzer (McCabe, Cognitive, SLOC, ABC, AIRD, AICP, etc.)
built on tree-sitter. All metrics are language-neutral; only the node-kind names differ per grammar.

## Repository layout

| Path | Purpose |
|------|---------|
| `src/lib.rs` | Extension list, `language_for_file()`, re-exports — **single source of truth for language registration** |
| `src/complexity.rs` | All 13 metric calculations (pure tree-sitter traversal, no I/O) |
| `src/main.rs` | CLI, file discovery, `visit_functions()`, `get_function_name()`, output formats, threshold enforcement |
| `Cargo.toml` | Workspace; tree-sitter language crates live in `[workspace.dependencies]` and are forwarded via `[dependencies]` |

---

## Adding a new language — checklist

Work through these five files in order. Each step is small; the pattern is identical for every language.

### 1. `Cargo.toml` — add the crate

```toml
# [workspace.dependencies]
tree-sitter-go = "0.21"   # match the version pattern of existing crates

# [dependencies] (knots package)
tree-sitter-go.workspace = true
```

Check crates.io for the exact crate name (`tree-sitter-go`, `tree-sitter-java`, etc.).
Some crates export **two** language functions (e.g. `tree-sitter-typescript` exports
`language_typescript()` and `language_tsx()`); check the crate docs before writing `language_for_file`.

### 2. `src/lib.rs` — register the language

```rust
// LANGUAGES — the single source of truth (drives SUPPORTED_EXTENSIONS,
// `knots --supported-languages`, and every doc via `invoke sync-languages`)
LanguageInfo { name: "Go", extensions: &["go"], explicit_only: &[] },

// SUPPORTED_EXTENSIONS — add the same recursive extensions
// (a test enforces that this stays equal to LANGUAGES.extensions)
"go",

// language_for_file — add a match arm
Some("go") => tree_sitter_go::LANGUAGE.into(),

// re-export so tests can reach the grammar
pub use tree_sitter_go;
```

Then run `invoke sync-languages --write` to propagate the new language into the
README, docs, and packaging descriptions (a single command, no hand-editing).

### 3. `src/complexity.rs` — map node kinds to metrics

Each function below needs a new `match` arm (or additions to an existing one) for the new grammar's node names.
Find the correct names by running `knots --debug` on a sample file, or by reading the grammar's
`grammar.js` / `node-types.json` in the crate source.

| Function | What to add |
|----------|-------------|
| `visit_node_mccabe` | `if_statement`, loops, logical operators, `switch`/`match` equivalents |
| `visit_node_cognitive` | Same structures + closures/lambdas (increment nesting, no base cost) |
| `visit_node_nesting` | Same control-flow nodes |
| `visit_node_abc` | Assignments, call expressions, conditions |
| `count_explicit_params` | Parameter node kinds for the function nodes you add in step 4. **Important:** if the language's `function_declaration` has a direct `parameters` named field (JS, TS, Go…), try that first; fall back to `count_c_params_in_subtree` only for C/C++. |
| `collect_self_fields_recursive` | How the language spells `self.field` / `this.field`. Rust = `field_expression`, Python = `attribute`, JS/TS = `member_expression` with `object == "this"`. |
| `calculate_sloc_*` | Python (`#` comments) has a separate path; add a new one only if the language uses a comment style not covered by `calculate_sloc` (`//` and `/* */`). |

### 4. `src/main.rs` — wire up function discovery

Three places:

**`visit_functions`** — add the grammar's function node kinds:
```rust
| "func_literal"         // Go example
| "method_declaration"   // Go example
```

**`get_function_name`** — add a branch that extracts the function name.
Most languages have a direct `name` field (like Rust's `function_item`).
C/C++ is the exception that uses a declarator chain.

**`collect_local_names_recursive`** — add the same node kinds so locally-defined
functions are excluded from external-call counts.

**`collect_function_metrics`** — the Python SLOC branch (`is_python`) is the only
language-specific path here. Add a similar guard only if the new language needs a
different SLOC mode. Otherwise nothing to change.

### 5. `src/main.rs` — add discovery tests

Mirror the existing `discover_js_functions` / `discover_ts_functions` pattern:

```rust
fn discover_go_functions(code: &str) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&knots::tree_sitter_go::language()).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let mut cursor = tree.root_node().walk();
    let mut names = Vec::new();
    visit_functions(&mut cursor, code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            names.push(name);
        }
    });
    names
}
```

Cover: plain function, method on a type, multiple functions, anonymous/closure (if applicable).

---

## Key invariants

- **Metrics are language-neutral** — the formulas in `complexity.rs` never change; only node-kind strings differ.
- **`LANGUAGES` is the single source of truth for language support** — `SUPPORTED_EXTENSIONS`, `knots --supported-languages`, and every doc that lists languages derive from it. After editing it, run `invoke sync-languages` (a test guards that `SUPPORTED_EXTENSIONS` still matches).
- **`SUPPORTED_EXTENSIONS` is the recursive-discovery gate** — if an extension isn't listed there, `--recursive` silently skips those files.
- **`.h` files are intentionally excluded** from `SUPPORTED_EXTENSIONS` even though `language_for_file` maps them to C. Headers are parsed when passed explicitly.
- **SLOC mode** — only Python uses `calculate_sloc_python` (skips `#` lines). Everything else uses `calculate_sloc` (`//` and `/* */`). Add a new mode only if necessary.
- **External calls** — `collect_local_names_recursive` must mirror `visit_functions`; any function node kind in one should be in the other, or locally-defined functions will be misclassified as external calls.

---

## Language-specific calibration notes

### Ada — McCabe vs Cognitive for case/dispatch patterns

Ada's `case_statement` counts each `when` alternative as +1 to McCabe (correct per the McCabe definition). A dispatch table with 20 `when` arms contributes 20 to McCabe even if each arm is a single assignment. The same construct contributes only `1 + nesting` to Cognitive complexity.

**Consequence:** McCabe thresholds calibrated against C/Rust code (e.g. the default threshold of 10–15) will fire on routine Ada dispatch tables that are not genuinely complex.

**Recommendation when analysing Ada code:**
- Use **Cognitive complexity** as the primary gate; McCabe as secondary.
- If using McCabe thresholds, raise them for Ada (20–25 is a reasonable starting point for code that uses large case statements).
- `select_alternative` in task bodies has the same per-branch counting, so selective_accept with many alternatives inflates McCabe the same way.

---

## Languages currently supported

This table is generated from the `LANGUAGES` table in `src/lib.rs` — the single
source of truth. Run `invoke sync-languages` after editing it to refresh every
doc that lists languages (and `knots --supported-languages` to print it). Grammar
crates are the `tree_sitter_*` re-exports at the top of `src/lib.rs`.

<!-- BEGIN:supported-languages (generated by `invoke sync-languages`) -->
| Language | Extensions | Explicit-only |
|----------|------------|---------------|
| C | `.c` | `.h` |
| C++ | `.cpp` `.cc` `.cxx` `.hpp` `.hxx` | — |
| Rust | `.rs` | — |
| Python | `.py` | — |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx` | — |
| TypeScript | `.ts` `.tsx` | — |
| Ada | `.adb` `.ada` | `.ads` |
| Go | `.go` | — |
| Java | `.java` | — |
| C# | `.cs` | — |
| Kotlin | `.kt` `.kts` | — |
| Swift | `.swift` | — |
| PHP | `.php` | — |
<!-- END:supported-languages -->

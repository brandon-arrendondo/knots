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

### 2. `src/lib.rs` — register extensions

```rust
// SUPPORTED_EXTENSIONS — add the new extensions
"go",

// language_for_file — add a match arm
Some("go") => tree_sitter_go::language(),

// re-export so tests can reach the grammar
pub use tree_sitter_go;
```

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
- **`SUPPORTED_EXTENSIONS` is the recursive-discovery gate** — if an extension isn't listed there, `--recursive` silently skips those files.
- **`.h` files are intentionally excluded** from `SUPPORTED_EXTENSIONS` even though `language_for_file` maps them to C. Headers are parsed when passed explicitly.
- **SLOC mode** — only Python uses `calculate_sloc_python` (skips `#` lines). Everything else uses `calculate_sloc` (`//` and `/* */`). Add a new mode only if necessary.
- **External calls** — `collect_local_names_recursive` must mirror `visit_functions`; any function node kind in one should be in the other, or locally-defined functions will be misclassified as external calls.

---

## Languages currently supported

| Language | Extensions | Grammar crate |
|----------|-----------|---------------|
| C | `.c`, `.h` (explicit only) | `tree-sitter-c` |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx` | `tree-sitter-cpp` |
| Rust | `.rs` | `tree-sitter-rust` |
| Python | `.py` | `tree-sitter-python` |
| JavaScript | `.js`, `.mjs`, `.cjs` | `tree-sitter-javascript` |
| TypeScript | `.ts` | `tree-sitter-typescript` (`language_typescript()`) |
| TSX | `.tsx` | `tree-sitter-typescript` (`language_tsx()`) |

# knots — developer guide for Claude

knots is a multi-language complexity analyzer (McCabe, Cognitive, SLOC, ABC, AIRD, AICP, etc.)
built on tree-sitter. All metrics are language-neutral; only the node-kind names differ per grammar.

## Repository layout

| Path | Purpose |
|------|---------|
| `src/lib.rs` | Substrate registry/grammar re-exports, `language_for_file()`, `FunctionMetrics`, `collect_function_metrics()`, `visit_functions()`, `get_function_name()`, `FilterRules`, and all function-analysis helpers — the public library API |
| `src/complexity.rs` | All 13 metric calculations (pure tree-sitter traversal, no I/O) |
| `src/main.rs` | CLI, file discovery, output formats, threshold enforcement |
| `Cargo.toml` | Workspace; the knots package depends on `lang-parsing-substrate` for language detection and grammars |

---

## Adding a new language — checklist

Language registration and grammar dependencies belong to
[`lang-parsing-substrate`](https://github.com/brandon-arrendondo/lang_parsing_substrate),
not a local knots registry. Keep metric and function-discovery changes here.

### 1. Add parsing support in the substrate

Follow the substrate's developer guide: add the optional grammar dependency
and feature, registry metadata and extension dispatch in `src/registry.rs`,
and the cfg-gated grammar re-export in `src/lib.rs`. Test parsing and registry
behavior there, and release that change before updating this consumer.

### 2. Adopt the substrate release in knots

Update the `lang-parsing-substrate` workspace dependency in `Cargo.toml`.
Add any new compatibility grammar re-export to `src/lib.rs`; its language
lookup and registry functions are already re-exported from the substrate.
Check grammar constants for variants such as TypeScript and TSX rather than
assuming an older grammar API.

Use `knots --supported-languages` to inspect the compiled registry. Review
the language tables against that output. `invoke sync-languages` currently
expects the removed local `LANGUAGES` constant; it needs a separate update
before it can regenerate those tables. Do not recreate that constant here.

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

### 4. `src/lib.rs` — wire up function discovery

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
    parser.set_language(&knots::tree_sitter_go::LANGUAGE.into()).unwrap();
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
- **The substrate registry is the source of truth for language support** — knots re-exports `languages()`, extension lookup, and grammar dispatch. `knots --supported-languages` reports that compiled registry.
- **Recursive discovery uses the substrate's extension predicates** — add extension metadata there, not a separate knots list.
- **`.h` (and other `explicit_only` extensions like Ada's `.ads`) are included in `--recursive` discovery** via `is_parseable_extension`, alongside each language's primary `extensions` — see `docs/architecture.rst` for why. `.h` content is sniffed for unambiguous C++-only syntax (`language_for_header_content`) before falling back to the C grammar.
- **SLOC mode** — only Python uses `calculate_sloc_python` (skips `#` lines). Everything else uses `calculate_sloc` (`//` and `/* */`). Add a new mode only if necessary.
- **External calls** — `collect_local_names_recursive` must mirror `visit_functions`; any function node kind in one should be in the other, or locally-defined functions will be misclassified as external calls.
- **Preprocessor dead-code blanking happens once, in `parse_file`, before anything parses the source** — for C, C++, Swift, and C#, `knots::blank_dead_code(source_code, language_key)` replaces every character (not the newline) on a preprocessor-dead line (`#if 0`, `#ifdef __cplusplus`, locally-provable `#ifdef MACRO`, Swift/C# constant-false `#if`) with a space, and `parse_file` returns the resulting `(Tree, String)` pair instead of just a `Tree`. Every caller must destructure both and use the returned source for everything downstream (metrics, import extraction, fingerprinting) — reusing the original, unblanked source alongside the returned tree will desync line/byte offsets from what the tree actually contains. Other languages are an unconditional passthrough. See `docs/architecture.rst` ("Preprocessor dead-code awareness") and `docs/metrics-reference.rst` ("Preprocessor Dead-Code Exclusion") for the full rationale and per-language rules.

---

## Departures from a metric's definition

A metric's published definition is the authority (`docs/adr/0001-the-definition-is-the-authority.md`).
**A departure from it is either recorded with its reason in `validation/probes/conformance.toml`, or it is a bug.**
`tests/probes.rs` enforces this: it fails on an unrecorded difference and on a recorded one that no longer occurs.
Before changing a counting rule, add or adjust a probe in `validation/probes/`, with the definition's value and its
source, and run `validation/compare_versions.py` against the previous release.

**Never resolve a name by its spelling** (`docs/adr/0002-a-name-is-not-a-function.md`): count a call, type or
declaration only where the syntax fixes its target; otherwise count nothing and record the undercount. Probe every
language a resolution change touches, not just C.

## Language-specific calibration notes

### Ada — McCabe vs Cognitive for case/dispatch patterns

Ada's `case_statement` counts each `when` alternative as +1 to McCabe, except `when others` (the default), as in the McCabe definition. A dispatch table with 20 `when` arms contributes 20 to McCabe even if each arm is a single assignment. The same construct contributes only `1 + nesting` to Cognitive complexity.

**Consequence:** McCabe thresholds calibrated against C/Rust code (e.g. the default threshold of 10–15) will fire on routine Ada dispatch tables that are not genuinely complex.

**Recommendation when analysing Ada code:**
- Use **Cognitive complexity** as the primary gate; McCabe as secondary.
- If using McCabe thresholds, raise them for Ada (20–25 is a reasonable starting point for code that uses large case statements).
- `select_alternative` in task bodies has the same per-branch counting, so selective_accept with many alternatives inflates McCabe the same way.

---

## Languages currently supported

Language metadata comes from the substrate's `languages()` registry,
re-exported by `src/lib.rs`. Use `knots --supported-languages` to inspect the
current compiled set. The table below retains the old generator markers;
`invoke sync-languages` must be updated to read the substrate before use.
Grammar re-exports in `src/lib.rs` preserve the `knots::tree_sitter_*` API.

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
| Fortran | `.f90` `.f95` `.f03` `.f08` `.F90` `.F95` `.F03` `.F08` | `.f` `.for` `.f77` `.F` `.FOR` `.F77` |
| Scala | `.scala` `.sc` | — |
| Lua | `.lua` | — |
<!-- END:supported-languages -->

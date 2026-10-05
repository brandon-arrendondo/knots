# ADR-0002: A name is not a function; count only what the syntax resolves

**Status:** Accepted, 2026-10-05

## Context

knots computes every metric from a tree-sitter parse, through
`lang-parsing-substrate`. A parse is syntax only. It doesn't say which
declaration a name refers to, what type an expression has, or which
preprocessor arm a build compiles. Any metric step that needs one of those
answers either gets it from the syntax or guesses.

The Rust definition probes found such a guess in the Cognitive Complexity
recursion increment. Cycles were found by matching callee names, so a call
reached any function with that spelling:

- `self.0.fmt(f)` inside `fmt` counted as `fmt` calling itself. On twelve
  crates.io crates, 4,554 of 13,882 functions (33%) scored a false +1,
  nearly all delegations (`clone`, `hash`, `fmt`, `next`).
- Functions that share a name were merged into one node: `From<A>::from` and
  `From<B>::from`; `mod small`'s `iadd` and `mod large`'s; a nested `fn imp`
  in each of a dozen functions; an `#else` stub (sqlite `sqlite3SelectDup`,
  curl `curl_formfree`) and the real definition in the other arm, so the stub
  borrowed the real one's recursion.

The figure looked plausible, and it surfaced only by reading individual
results. aurora-lint, a C-specialist built on the same substrate, recorded
the same defect in rules three times (its ADR-0006, "a name is not a
variable"), the same mistake one level up through name-keyed maps ("a tag is
not a type"), and how parser recovery from preprocessor text gets misread
(its ADR-0008). Its resolvers are C-specific; the principle is not.

## Decision

1. **Never decide what a name refers to by its spelling alone.** A call, a
   type or a declaration counts toward a metric only where the syntax fixes
   its target. For recursion, that means:
   - a bare call, resolved through the language's lexical scopes where knots
     models them (Rust `fn` and `mod`); elsewhere, file-wide;
   - a call through `self`/`this`;
   - Rust `Self::f` or `Type::f` inside `Type`'s impl.

   See `src/recursion.rs`.
2. **When identity can't be resolved, don't count, and record it.** A metric
   has no equivalent of aurora-lint's "report what you can't prove safe".
   An unresolved name adds nothing, and the resulting undercount is a
   recorded departure under ADR-0001, with its measured size (for example
   `child.depth()` in `validation/probes/conformance.toml`).
3. **Alternatives are not combined.** Two definitions in mutually exclusive
   preprocessor arms, or `#[cfg]` variants, are different functions in
   different configurations, never one node. The substrate's dead-code
   detection (`blank_dead_code`: C/C++ `#if 0`, `__cplusplus` and locally
   proven macros, C# and Swift `#if`) removes arms no build compiles before
   parsing. What remains are live alternatives, and how a metric scores them
   is a separate decision.
4. **When parser recovery produces structure, diagnose what was misread
   before trusting it.** For example, a C function that appears nested in
   another is nearly always error recovery around a macro. Don't build
   semantics on that structure (aurora-lint ADR-0008).
5. **Resolution code lives in knots until it is proven useful across
   tools.** It moves to `lang-parsing-substrate` only when another consumer
   (aurora-lint, moldy, clew) needs the same capability, per aurora-lint
   ADR-0003's utility-layer rule.

## Consequences

- Each language knots supports needs probes for the resolution rules it
  relies on. A fix validated on one grammar's probes can overfit that
  grammar, as the recursion increment did on C.
- Places that still decide by spelling are audit items, not settled
  behavior:
  - the external-call count that feeds AICP (any name not defined in the file);
  - file-wide bare-call recursion for C++ and Java overloads and namespaces;
  - stem-matched `#include` resolution in coupling (C family).
- Undercounting where identity is unknown is the accepted cost, and it is
  measured and recorded, not left implicit.

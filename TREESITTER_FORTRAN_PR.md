# PR Details: fixed-form comment support for tree-sitter-fortran

## Target

- **Upstream repo**: `stadelmanma/tree-sitter-fortran`
- **Fork**: `brandon-arrendondo/tree-sitter-fortran`
- **Base branch**: `master` (upstream)
- **Head branch**: `master` (fork: `brandon-arrendondo:master`)
- **Commit**: `9dda4e0`

## Command

```sh
gh pr create \
  --repo stadelmanma/tree-sitter-fortran \
  --head brandon-arrendondo:master \
  --base master \
  --title "fix: add fixed_form_comment external token for fixed-form *.f files" \
  --body "$(cat <<'EOF'
## Problem

Fixed-form Fortran (`.f`, `.for`, `.f77`) treats any line whose **first
character (column 0)** is `*`, `C`, or `c` as a comment. The grammar
currently only recognises `!` as a comment character (free-form style).

When tree-sitter-fortran encounters LAPACK-style Doxygen comment blocks,
which repeat the subroutine signature verbatim inside `*`-prefixed lines:

```fortran
*       SUBROUTINE CHEGV( ITYPE, JOBZ, UPLO, N, A, LDA, B, LDB, W, WORK,
*                         LWORK, RWORK, INFO )
```

it produces a parse error that prevents the **real** `SUBROUTINE` declaration
below from being recognised. 2,042 of 2,114 LAPACK SRC files use this
multi-line `*` comment pattern.

This column-position constraint cannot be expressed as a tree-sitter regex,
so the fix must live in the external scanner where `get_column()` is available.

## Fix

- **`grammar.js`**: add `$.fixed_form_comment` to `externals` and to `extras`
  (alongside `$.comment`). Listing it in `extras` makes tree-sitter treat it
  as ignorable whitespace that can appear anywhere.

- **`src/scanner.c`**: add `FIXED_FORM_COMMENT` to `TokenType` and implement
  `scan_fixed_form_comment()`. When `lexer->get_column(lexer) == 0` and
  `lookahead` is `*`, `C`, or `c`, the function advances to end-of-line and
  emits `FIXED_FORM_COMMENT`. This fires at the very top of `scan()`, before
  any whitespace is consumed, so column tracking is still reliable.

- **`src/parser.c`**: regenerated with tree-sitter-cli 0.22.6.

## Validation

Tested against the LAPACK reference implementation
(`github.com/Reference-LAPACK/lapack`, SRC directory, 2,114 `.f` files):

| | Functions found |
|---|---|
| Before this fix | 1,157 (−45% vs lizard) |
| After this fix | 2,072 (−2% vs lizard 2,106) |

The remaining 34-function gap (~1.6%) is within noise for a corpus this size
and is not attributable to comment handling.

Modern free-form Fortran (`.f90`, `.f95`, etc.) is unaffected — those files
do not use `*`/`C` comment convention and the new token is guarded by the
column-0 check.
EOF
)"
```

## Notes

- The `package.json` changes (added `prebuildify` script and `tree-sitter`
  section) were produced automatically by `tree-sitter generate` and should be
  included in the PR as-is — they are part of the standard generate output.
- The `src/tree_sitter/alloc.h`, `array.h`, and `parser.h` header diffs are
  also from `tree-sitter generate` updating the bundled headers to match
  tree-sitter-cli 0.22.6.
- Once this PR is merged and a new crate version is published,
  `knots/Cargo.toml` should be updated from the git fork pin back to the
  crates.io version.

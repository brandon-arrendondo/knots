# Validation

Scripts that produce the agreement figures knots publishes, so each figure
can be reproduced and re-checked whenever a counting rule changes
(`docs/adr/0001-the-definition-is-the-authority.md`).

## Corpora (pinned)

The six C calibration corpora, at the commits the AIRD paper and the
aurora-lint oracle use:

| Corpus | Commit |
|--------|--------|
| lua | `40b76de2d77e` |
| libcrc | `7719e2112a9a` |
| mosquitto | `d3ee5c5ca62c` |
| hostap | `dcee60436390` |
| sqlite | `b1a73ba34d05` |
| curl | `3e198f75861c` |

Check out each one detached at its commit before running. A drifted tree
silently changes every figure.

The Rust corpus is twelve crates.io releases, pinned by version (a published
crate version never changes): regex 1.13.1, regex-syntax 0.8.11, serde_json
1.0.151, syn 2.0.119, clap_builder 4.6.7, hashbrown 0.17.1, aho-corasick
1.1.5, memchr 2.8.3, winnow 0.7.15, toml_edit 0.22.27, rayon-core 1.13.0 and
itertools 0.13.0, 13,882 functions. Use the copies under
`~/.cargo/registry/src/` once a build has fetched them (or unpack each
`.crate` from crates.io with `tar`), and pass the directories with
`--ext=.rs`.

## Scripts

- `pmccabe_compare.py KNOTS_BIN OUT.json CORPUS...` compares knots' McCabe
  with pmccabe's "modified" count, function by function. It tags each
  mismatch with the constructs present (`goto`, `throw`, `#if`, ternary).
- `compare_versions.py [--ext=.rs] OLD_BIN NEW_BIN OUT.json CORPUS...` shows
  which metrics moved between two knots builds, AIRD deltas, band changes,
  and crossings of the 85 gate. `--ext` picks the file extensions (default C
  and C++).

## Results recorded 2026-10-05

**pmccabe agreement, C files (30,461 matched functions):**

| knots | Equal | Differ | Agreement |
|-------|-------|--------|-----------|
| 1.18.0 (`de0c562`, counts `goto`) | 28,285 | 2,176 | 92.9% |
| branch, no `goto`/`throw`/`raise`, vs. the modified column | 29,753 | 708 | 97.7% |
| branch, each non-default `case` counted, vs. the **traditional** column (McCabe's definition) | 29,744 | 717 | 97.65% |

In 858 of the 861 functions whose only extra construct was `goto`, knots'
excess equaled the function's `goto` count exactly. After the change, 702 of
the 708 differing functions contain `#if` arms. knots counts decisions in
every arm and pmccabe keeps the first.

**Effect of the change on 33,000 functions (1.18.0 → branch):**
- Cognitive Complexity: unchanged.
- McCabe: 1,711 functions changed.
- Test score: 932 changed.
- AIRD: 819 changed, each by −1 to −3. 58 functions moved down one band; none
  crossed the 85 gate.

**Cognitive Complexity brought to the specification (branch, second change):**
+1 for each function in a recursion cycle, and no increment for
`throw`/`raise` (whitepaper 1.7, Appendix B1). Against 1.18.0 on the same
33,000 functions, combined with the McCabe change:
- Cognitive Complexity: 603 functions changed. All C changes are the +1 for
  recursion; spot-checked cycles in sqlite are real, e.g. `btreeNext` →
  `sqlite3BtreeNext` → `btreeNext`.
- AIRD: up 1 in 395 functions (recursion) and down 1–3 in about 813 (McCabe
  via the test score). About 75 functions changed band in either direction;
  none crossed the 85 gate.

**AIRD validation study micro-pilot (32 tasks, `aird_validation_study`
9118b81):** 30 keep the same AIRD and 2 move by 1 point. None changes
Cognitive Complexity. McCabe drops sharply for `goto`-heavy tasks (e.g.
mosquitto `handle__connect` from 68 to 44).

**Definition probes (`validation/probes/`, third change).** The probes
found, and this branch fixed, four more departures with no recorded reason:
- Cognitive Complexity now counts the conditional operator (+1 plus
  nesting).
- A negation now starts a new boolean sequence.
- Ada McCabe no longer counts `when others`.
- Ada McCabe no longer counts a bare `loop`.

**Cumulative effect against 1.18.0, the same 32,999 functions, all fixes on
the branch:**
- Cognitive Complexity: 3,580 functions changed (10.8%), mostly from the
  ternary.
- McCabe: 1,711 changed.
- AIRD: 3,499 changed; 2,783 rose (by up to +48 for ternary-heavy functions)
  and 716 fell by 1–3.
- 376 band changes; **20 functions now cross the 85 gate upward** (for
  example curl `client_cert` 77 → 91, hostap `bss_update` 67 → 90).
- The AIRD paper's three exemplars (`hostapd_config_read_eap_user`,
  `whereLoopAddBtreeIndex`, `luaV_execute`) keep their AIRD (98, 98, 89).
  Their Cognitive Complexity is already above the ceiling.
- Micro-pilot (32 tasks): 7 change AIRD, all within their band. The one
  exception is `http_output_bearer` at 3 → 2, under the L band's listed
  floor of 3.

**switch counted per case (fourth change, Brandon's call on the evidence in
`validation/probes/TOOLS.md`):** C/C++/PHP `switch` is now one decision per
non-default `case`, the definition. pmccabe's traditional column is the
reference: 29,744 of 30,461 agree (97.65%), against 28,113 for its modified
column. Labels that sqlite's statement-like `deliberate_fall_through` macro
makes the parser misread are recovered. Labels generated by macros (Lua's
`vmcase`) stay invisible to both tools.

**Cumulative effect against 1.18.0, all changes on the branch (32,999
functions):**
- McCabe: 3,277 changed.
- Test score: 2,092 changed.
- Cognitive Complexity: 3,580 changed (unaffected by the switch change).
- AIRD: 4,448 changed (3,759 up, 689 down); 510 band changes; 20 functions
  cross the 85 gate, the same 20 as before the switch change, since McCabe
  reaches AIRD only through the capped test score.
- The micro-pilot and the paper's exemplars are as above.

**Rust probes (fifth change).** Fourteen Rust probes found five departures
with no recorded reason, all fixed:
- McCabe counted a `match` once; it is now one decision per arm but the last,
  plus guards and `|` alternatives.
- McCabe counted a bare `loop` as a decision, and didn't count `let ... else`.
- Cognitive Complexity didn't count `break`/`continue` to a label.
- Recursion through `Self::f` wasn't seen.

The probes also exposed a fault in the second change. Cycles were found by
matching callee names, so a method called on any receiver matched any
function of that name. On the Rust corpus that scored 4,554 of 13,882
functions as recursive, nearly all delegations (`clone`, `hash`, `fmt`,
`next`). The graph now follows only calls whose target the syntax fixes (see
`src/recursion.rs`). Recursion only through another receiver
(`child.depth()`) is a recorded departure in `probes/conformance.toml`.
Against the previous branch commit on the C corpora, this changes Cognitive
Complexity in 172 functions, all down by 1: 170 C++ gmock delegations in
mosquitto's tests, and two `#else` stubs (sqlite `sqlite3SelectDup`, curl
`curl_formfree`) that had borrowed the recursion of the other definition of
the same name.

**Cumulative effect against 1.18.0, all changes on the branch:**
- C (32,999 functions): McCabe 3,438 changed; Cognitive Complexity 3,584
  (both including the sixth change below); test score 2,092; AIRD 4,335
  (3,646 up, 689 down); 510 band changes; the same 20 functions cross the 85
  gate.
- Rust (13,882 functions): McCabe 923 changed; Cognitive Complexity 110;
  test score 766; AIRD 626 (612 up, 14 down); 82 band changes; none crosses
  the 85 gate.

**Preprocessor configurations (sixth change).** McCabe and Cognitive
Complexity now score a C/C++ function as its worst real configuration (ADR-0002
§3). Each `#if` chain is one choice, chains on the same condition are one
choice, and arms are never summed. Four probes cover it (`c/preproc_*.c`), and
the two recorded `preproc_arms.c` departures are gone. AIRD, AICP and the test
score read the source as written, so they do not move.

Against the previous branch commit (32,999 functions): McCabe changed in 239
functions and Cognitive Complexity in 249, all down. Test score, AIRD and AICP
are unchanged. For example, mosquitto `mosquitto_fopen` goes from 34 to 21:
the `WIN32` arm (13 decisions) and the POSIX arm (20) had been added together.
One sqlite function exceeds the 64-combination limit and keeps every arm
counted.

pmccabe agreement, traditional column: 29,852 of 30,461 (98.00%), up from
29,744. 597 of the 609 remaining differences are in functions with `#if` arms:
pmccabe keeps the first arm, knots the worst.


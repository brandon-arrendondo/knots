====================
Benchmark Reference
====================

Cross-language calibration results for knots against lizard, radon,
rust-code-analysis (rca), tokei, and cloc.  These results informed the
AIRD/AICP formula calibration and established per-language validation
status.

Comparison Tools
================

.. list-table::
   :header-rows: 1
   :widths: 20 15 45 20

   * - Tool
     - Version
     - Location
     - Metrics
   * - lizard
     - 1.23.0
     - ``~/data-enterprise/venv/bin/lizard``
     - McCabe, NLOC, token count
   * - radon
     - 6.0.1
     - ``~/data-enterprise/venv/bin/radon``
     - McCabe, Halstead (Python only)
   * - rust-code-analysis-cli (rca)
     - 0.0.25 (git HEAD)
     - ``~/.cargo/bin/rust-code-analysis-cli``
     - Cyclomatic, Cognitive, Halstead, SLOC, ABC
   * - tokei
     - (cargo install)
     - ``~/.cargo/bin/tokei``
     - SLOC by language
   * - cloc
     - (apt)
     - ``/usr/bin/cloc``
     - SLOC by language

.. note::

   rca 0.0.25 from crates.io fails to compile on rustc 1.95.  Install
   from git HEAD::

     cargo install --git https://github.com/mozilla/rust-code-analysis \
       rust-code-analysis-cli

Benchmark Corpora
=================

Cross-language calibration
--------------------------

.. list-table::
   :header-rows: 1
   :widths: 25 20 10 45

   * - Repo
     - Language
     - Files
     - Notes
   * - cobra
     - Go
     - 36
     - github.com/spf13/cobra
   * - zod
     - TypeScript
     - 401
     - github.com/colinhacks/zod
   * - commons-lang
     - Java
     - 623
     - github.com/apache/commons-lang
   * - Newtonsoft.Json
     - C#
     - 945
     - github.com/JamesNK/Newtonsoft.Json
   * - todo-sqlite-cli
     - Rust
     - 36
     - ~/toolchain/todo-sqlite-cli
   * - curl
     - C
     - 744
     - ~/toolchain/curl
   * - mosquitto
     - C++
     - 974
     - ~/toolchain/mosquitto
   * - gnatcoll-core
     - Ada
     - 452
     - ~/toolchain/gnatcoll-core
   * - lua/testes
     - Lua
     - 34
     - lua.org reference implementation v5.5.1-dev; test suite only
   * - laravel
     - PHP
     - 2,966
     - github.com/laravel/framework
   * - scala/src/library
     - Scala
     - 542
     - github.com/scala/scala (standard library only)
   * - lapack (SRC)
     - Fortran
     - 31 ``.f90`` / 2,114 ``.f``
     - github.com/Reference-LAPACK/lapack

AIRD/AICP calibration corpora
------------------------------

.. list-table::
   :header-rows: 1
   :widths: 20 15 10 55

   * - Repo
     - Language
     - Functions
     - Notes
   * - lua
     - C/Lua
     - 1,304
     - lua.org reference implementation v5.5.1-dev
   * - libcrc
     - C
     - 34
     - libcrc.org CRC library
   * - mosquitto
     - C/C++
     - 2,559
     - eclipse/mosquitto MQTT broker
   * - hostap
     - C
     - 13,343
     - w1.fi/hostapd + wpa_supplicant
   * - sqlite
     - C
     - 9,491
     - sqlite.org amalgamation + tools
   * - curl
     - C
     - 5,474
     - curl/curl HTTP library

Cross-Language Calibration Summary
===================================

.. note::

   Numbers below reflect the corpus state as of 2026-06-28 (knots v1.13.0,
   lizard 1.23.0).  Corpora are live git clones and drift over time; the
   deltas and status notes are the durable signal.  Re-run when a corpus
   is refreshed or a new knots version ships.

Function Count: knots vs. lizard
---------------------------------

.. list-table::
   :header-rows: 1
   :widths: 15 25 10 10 10 40

   * - Language
     - Corpus
     - knots
     - lizard
     - Delta
     - Status
   * - Java
     - commons-lang
     - 10,919
     - 10,597
     - +3%
     - Good agreement
   * - C#
     - Newtonsoft.Json
     - 7,339
     - 6,521
     - +13%
     - Good; knots picks up more interface/abstract methods
   * - Rust
     - todo-sqlite-cli
     - 120
     - 113
     - +6%
     - Good agreement
   * - Swift
     - srg_collection_manager_app_ios
     - 199
     - 201
     - −1%
     - Excellent
   * - Kotlin
     - srg_collection_manager_app
     - 381
     - 319
     - +19%
     - Explained: 55 single-expression funs + 7 abstract decls lizard skips
   * - C
     - curl
     - 5,836
     - 4,920
     - +19%
     - Plausible; knots picks up static inline functions
   * - C++
     - mosquitto
     - 5,549
     - 3,305
     - +68%
     - Plausible; templates and lambdas
   * - Go
     - cobra
     - 595
     - 805
     - −26%
     - Explained: 210 anonymous ``func_literal`` closures; named counts equal
   * - TypeScript
     - zod
     - 1,696
     - 6,081
     - −72%
     - Explained: ~4,400 anonymous arrow callbacks; named counts similar
   * - Python
     - srg_card_search_website
     - 134
     - 133
     - ~equal
     - Good
   * - Lua
     - lua/testes
     - 590 named / 1,065 with flag
     - 1,054
     - ~equal with ``--count-anonymous-closures``
     - Explained: 475 anonymous closures
   * - PHP
     - laravel
     - 30,844
     - 26,998
     - +14%
     - Plausible; interface/trait methods
   * - Scala
     - scala/src/library
     - 11,292
     - 4,490
     - +151%
     - Explained: 6,695 SLOC=1 expression-body defs lizard skips
   * - Fortran (.f90)
     - lapack SRC
     - 16
     - 16
     - 0%
     - Excellent; corpus reorganized (14 files left SRC/), both tools agree
   * - Fortran (.f)
     - lapack SRC (2,114 files)
     - 2,072
     - 2,106
     - −2%
     - Fixed; was −45% before tree-sitter-fortran fork fixed ``*``-continuation comment handling

Rust: knots vs. rust-code-analysis (rca)
-----------------------------------------

rca is the authoritative Rust-specific tool.  Lizard McCabe for Rust is
non-standard and should not be used as the comparison baseline.

.. list-table::
   :header-rows: 1
   :widths: 20 15 25 10 30

   * - Metric
     - knots
     - rca (named fns only)
     - Delta
     - Explanation
   * - Function count
     - 120
     - 156
     - rca higher
     - rca counts single-line closures as named functions
   * - Avg McCabe
     - 2.37
     - 4.04
     - −41%
     - rca counts ``?`` as a branch; knots does too (see below)
   * - Avg Cognitive
     - 1.96
     - 1.47
     - +33%
     - Similar ballpark
   * - Avg SLOC
     - 19.30
     - 10.56
     - +83%
     - rca deflated by 36 single-line closures in denominator

Performance
===========

Measured on the project benchmark machine (24-core, 2026-06-28, knots v1.13.0,
hyperfine 1.15.0, lizard 1.23.0).  **Absolute times are machine-specific; the
speedup ratios are the portable signal** and should be used for comparisons
across versions.

``--jobs`` scaling
------------------

knots processes files in parallel via rayon (``--jobs``/``-j``).  Both corpora
show the same scaling shape: near-linear to ``-j4``, good gains to ``-j8``,
diminishing returns beyond that, with regression at ``-j24`` from thread
oversubscription on this 24-core machine.

.. list-table::
   :header-rows: 1
   :widths: 10 15 10 15 10

   * - Jobs
     - hostap (C, 504 files)
     - Speedup
     - laravel (PHP, 2,970 files)
     - Speedup
   * - j1
     - 22.6s ± 0.3s
     - 1×
     - 18.1s ± 0.4s
     - 1×
   * - j2
     - 11.8s ± 0.4s
     - 1.9×
     - 12.3s ± 3.2s
     - 1.5×
   * - j4
     - 5.9s ± 0.1s
     - 3.8×
     - 4.8s ± 0.2s
     - 3.8×
   * - j8
     - 3.2s ± 0.1s
     - 7.0×
     - 2.8s ± 0.1s
     - 6.4×
   * - j16
     - 2.45s ± 0.05s
     - 9.2×
     - 2.0s ± 0.2s
     - 9.1×
   * - j24
     - 3.7s ± 0.9s
     - 6.1× (regresses)
     - 2.4s ± 0.3s
     - 7.4× (regresses)

The j2 laravel variance (σ=3.2s) reflects PHP file size non-uniformity; the
work-stealing pool evens out by j4.

knots vs. lizard vs. rca throughput
-------------------------------------

All three tools support parallelism: knots ``-j``, lizard ``-t``
(``--working_threads``, default 1), rca ``-j`` (``--num-jobs``).  Each was
benchmarked single-threaded and at 16 threads.

rca writes one JSON file per source file to a mirrored directory tree; this
I/O overhead (visible as high sys time) is unavoidable in normal use and is
included in the numbers below.  rca does not support PHP, so the laravel
corpus is knots/lizard only.

.. list-table::
   :header-rows: 1
   :widths: 22 10 10 10 10 10 10

   * - Corpus
     - knots j1
     - knots j16
     - lizard t1
     - lizard t16
     - rca j1
     - rca j16
   * - hostap (C, 504 files)
     - 22.3s
     - **2.4s**
     - 25.1s
     - 3.2s
     - 32.2s
     - 3.4s
   * - laravel (PHP, 2,970 files)
     - 17.7s
     - **2.0s**
     - 13.8s
     - 2.9s
     - —
     - —
   * - commons-lang (Java, 623 files)
     - 5.7s
     - **0.87s**
     - 10.2s
     - 1.6s
     - 11.0s
     - 1.2s

At full parallelism knots leads across the board: 1.3–1.4× faster than lizard
and rca on C, 1.4× faster than rca and 1.9× faster than lizard on Java.  rca
beats lizard on Java (Rust parser vs Python) but trails on C due to its
per-file output overhead.

Known Implementation Notes
===========================

C macro pattern: vmcase SLOC deflation
---------------------------------------

Lua's VM interpreter uses a macro dispatch pattern that tree-sitter-c
parses as nested ``function_definition`` nodes::

  vmdispatch(GET_OPCODE(i)) {
    vmcase(OP_MOVE)  { ... vmbreak; }
    vmcase(OP_LOADI) { ... vmbreak; }
  }

Before knots v1.12.0, the ``nested_fn_sloc`` subtraction incorrectly
fired on these macro blocks, reducing ``luaV_execute`` SLOC from 751 to
34 and AIRD from 87 to 76.  Fixed in v1.12.0: ``function_definition``
nodes whose declarator is a ``parenthesized_declarator`` (macro call
pattern) are filtered from the nested-SLOC subtraction.  Only
``identifier`` declarators (genuine nested functions) are subtracted.

Fortran explicit-only extension rejection
------------------------------------------

Before v1.13, explicit-only extensions (``.f``, ``.h``, ``.ads``) were
silently rejected even when passed directly on the command line.  Fixed
by introducing ``is_parseable_extension`` in ``src/lib.rs`` — the
command-line path now uses this (includes both recursive and
explicit-only extensions) rather than ``is_source_extension`` (recursive
discovery only).

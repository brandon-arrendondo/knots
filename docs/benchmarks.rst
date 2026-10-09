====================
Benchmark Reference
====================

This page preserves the June 2026 comparison and performance snapshot,
including knots v1.13.0 measurements. It is historical, not a claim of
conformance for the current release. Counting rules and parser support have
changed since these runs. A reference tool is comparison evidence, not the
authority for a metric's definition.

For pinned inputs, tool commits and reproducible current checks, use the
`validation record
<https://github.com/brandon-arrendondo/knots/blob/main/validation/README.md>`_,
`definition probes
<https://github.com/brandon-arrendondo/knots/blob/main/validation/probes/TOOLS.md>`_
and `ADR-0003
<https://github.com/brandon-arrendondo/knots/blob/main/docs/adr/0003-gate-defaults-have-a-recorded-basis.md>`_.
Current support is listed by ``knots --supported-languages``. In particular,
the current substrate supports free-form Fortran, not the fixed-form inputs
in the archived tables below.

Comparison Tools
================

.. list-table::
   :header-rows: 1
   :widths: 20 15 45 20

   * - Tool
     - Version
     - Installation used in the archived run
     - Metrics
   * - lizard
     - 1.23.0
     - Python package
     - McCabe, NLOC, token count
   * - radon
     - 6.0.1
     - Python package
     - McCabe, Halstead (Python only)
   * - rust-code-analysis-cli (rca)
     - 0.0.25 (git HEAD)
     - Cargo binary
     - Cyclomatic, Cognitive, Halstead, SLOC, ABC
   * - tokei
     - (cargo install)
     - Cargo binary
     - SLOC by language
   * - cloc
     - (apt)
     - Distribution package
     - SLOC by language

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
     - todo-sqlite-cli (local corpus checkout)
   * - curl
     - C
     - 744
     - curl (local corpus checkout)
   * - mosquitto
     - C++
     - 974
     - mosquitto (local corpus checkout)
   * - gnatcoll-core
     - Ada
     - 452
     - gnatcoll-core (local corpus checkout)
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
     - Excellent; 17 ``.f90`` files in SRC, both tools agree
   * - Fortran (.f)
     - lapack SRC (2,114 files)
     - 2,138
     - 2,106
     - +2%
     - Good agreement; was −45% before scanner fixes (PR #4 on tree-sitter-fixed-form-fortran)

Fortran Dialect Coverage
------------------------

The archived runs covered three Fortran corpora. The lapack/SRC row
superseded the single-row entry in the cross-language table above (same corpus,
same run). All ``.f`` files were passed explicitly: they were explicit-only
in the version measured, so recursive discovery did not include them. The
current substrate does not support fixed-form Fortran.

The corpora were LAPACK, fortran-stdlib and arpack-ng.

.. list-table::
   :header-rows: 1
   :widths: 20 18 8 8 8 8 30

   * - Corpus
     - Dialect
     - Files
     - knots
     - lizard
     - Delta
     - Notes
   * - lapack/SRC
     - Fixed-form ``.f``
     - 2,114
     - 2,138
     - 2,106
     - +2%
     - Good agreement.  knots avg McCabe 33.1 vs lizard 22.9 (+45%): knots
       counts ``.AND.``/``.OR.`` operators per the original McCabe definition;
       lizard does not.  This fixed-form comparison does not apply to
       current parser support.
   * - fortran-stdlib
     - Free-form ``.f90``/``.F90``
     - 411
     - 1,363
     - 805
     - +69%
     - Avg McCabe nearly identical (knots 3.1, lizard 3.0), so the extra
       functions knots finds are low-complexity.  knots visits generic
       procedure declarations and interface procedures inside modules;
       lizard skips them.
   * - arpack-ng
     - Mixed ``.f`` + ``.F90``
     - 334
     - 463
     - 234
     - +98%
     - lizard fails to detect multiple subroutines within single-file
       examples (EXAMPLES/ pattern: one driver + two helper subroutines per
       file).  PARPACK/ agrees closely (191 vs ~174).  knots avg McCabe 17.6
       vs lizard 8.0; gap reflects the missed functions, not a metric formula
       difference.

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
hyperfine 1.15.0, lizard 1.23.0).  Both absolute times and speedup ratios depend on the machine, corpus
and build. They do not guarantee performance on another system or version.

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

In this archived run, at full parallelism knots led across the board: 1.3–1.4× faster than lizard
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

Historical explicit-only extension handling
-------------------------------------------

The pre-v1.13 CLI rejected some explicit-only extensions. The current CLI
uses the substrate's ``is_parseable_extension`` for explicit paths and
recursive discovery, including ``.h`` and ``.ads``. That fix does not imply
current support for fixed-form Fortran; see :doc:`installation`.

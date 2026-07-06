=======================
Baseline / Ratchet Mode
=======================

``--baseline`` lets you adopt a knots threshold gate on an existing codebase
*without* first refactoring every pre-existing offender. You snapshot the
current per-function scores once, then the gate fails only on **regressions** —
a new function over threshold, or a baselined function whose score got *worse*.
Everything already in the snapshot is tolerated until you improve it.

This is the standard "don't make it worse" adoption pattern used by clippy,
eslint, and mypy on legacy code. It complements the :doc:`filters` (``--include``
/ ``--exclude``): filters decide *what gets analyzed*; the baseline decides
*what counts as a failure*.

Usage
-----

.. code-block:: bash

    # 1. Snapshot the current state (run once, commit the file)
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json --write-baseline

    # 2. Gate against the snapshot — fails only on new or worsened functions
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json

    # 3. After you improve (or accept new) code, regenerate the snapshot
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json --write-baseline

Commit ``.knots-baseline.json`` to your repo so the whole team (and CI / the
pre-commit hook) gates against the same snapshot.

How a regression is decided
---------------------------

For each function that is currently over one or more thresholds, knots looks it
up in the baseline by **(file, function)** and decides per metric:

.. list-table::
   :header-rows: 1
   :widths: 60 40

   * - Situation
     - Result
   * - Function absent from baseline (new code)
     - **fail**
   * - Metric value **>** its baselined value (got worse)
     - **fail**
   * - Metric value **≤** its baselined value (same / better)
     - tolerated
   * - Function under all thresholds
     - tolerated

A function that is still over threshold but *better* than its baseline passes —
so partial progress is never punished. Only ``--baseline`` (not
``--write-baseline``) changes gating; ``--write-baseline`` just (re)writes the
file and exits ``0``.

Failure output is labelled so it's clear you're in ratchet mode:

.. code-block:: text

    New or worsened threshold violations vs. baseline (1):
      src/input.rs:1:another_messy — AIRD 29 > 10  (drivers: cognitive 20, nesting 5)
    Error: 1 function(s) regressed beyond the baseline. Run with --write-baseline to accept the current state.

File format
-----------

The baseline is JSON, sorted by ``(file, function)`` for stable diffs. It records
every gateable metric so any threshold combination can be ratcheted. Line
numbers are **deliberately omitted** so the file does not churn when code moves.

.. code-block:: json

    {
      "version": 1,
      "functions": [
        {
          "file": "src/input.rs",
          "function": "process_input_event",
          "mccabe": 77,
          "cognitive": 215,
          "nesting": 6,
          "sloc": 492,
          "abc_magnitude": 120.5,
          "return_count": 3,
          "aird": 98,
          "aicp": 40,
          "external_calls": 12,
          "unreachable_blocks": 0
        }
      ]
    }

The file is normally generated, not hand-edited. ``--write-baseline`` requires
``--baseline <FILE>`` (it names the file to write) and works with any
``--format``.

Notes & limitations
-------------------

- **Key is (file, function).** Two functions with the same name in the same
  file collapse to one baseline entry (last one wins) — an accepted limitation
  shared with clippy/eslint-style baselines. Renaming a function or moving it to
  another file reads as "new" until you regenerate.
- **Snapshot the whole tree.** Generate the baseline with ``-r .`` (or your full
  source set) so every function has an entry; a partial snapshot will report
  un-snapshotted functions as new.
- **Touched-function scoping** (only gate functions in the current diff) is a
  separate, complementary feature — see ``--changed`` / ``--since`` in
  :doc:`cli-reference`. The two compose: scope to what you touched, then fail
  only on what got worse.

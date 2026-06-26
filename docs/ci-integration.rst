CI Integration
==============

Threshold Flags
---------------

Any ``--*-threshold`` flag causes knots to exit with status 1 if any
function exceeds the specified value. Use these as CI gates:

::

    # Fail if any function has AIRD > 85 (recommended threshold)
    knots -r src/ --aird-threshold 85

    # Enforce multiple metrics at once
    knots -r src/ \
        --aird-threshold 85 \
        --mccabe-threshold 20 \
        --cognitive-threshold 15 \
        --nesting-threshold 5

The recommended starting point is ``--aird-threshold 85``, which was
empirically validated against Sonnet 4.6 and Opus 4.8. See
:doc:`metrics-reference` for AIRD formula and corpus distribution.

Adopting on a Legacy Codebase
-----------------------------

On an older or larger project, turning the gate on cold means every
pre-existing offender fails at once. ``--baseline`` snapshots the current scores
and then fails only on **regressions** — new over-threshold functions, or
baselined ones that got worse — so you can adopt the gate today and improve from
there. See ``BASELINE`` for details.

.. code-block:: bash

    # Snapshot once and commit the file
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json --write-baseline

    # CI / pre-commit gate against the snapshot
    knots -r src/ --aird-threshold 85 --baseline .knots-baseline.json

In a pre-commit hook, pass both flags via ``args`` and commit
``.knots-baseline.json`` so every contributor gates against the same snapshot.

GitHub Actions
--------------

Basic CI gate:

.. code-block:: yaml

    - name: Complexity gate
      run: knots -r src/ --aird-threshold 85

SARIF upload for PR annotations:

.. code-block:: yaml

    - name: Run knots
      run: knots -r --format sarif src/ > knots.sarif

    - name: Upload SARIF
      uses: github/codeql-action/upload-sarif@v3
      with:
        sarif_file: knots.sarif

Pre-commit Hook
---------------

Knots integrates with the `pre-commit <https://pre-commit.com>`_ framework.
Add to ``.pre-commit-config.yaml``:

.. code-block:: yaml

    repos:
      - repo: https://github.com/brandon-arrendondo/knots
        rev: v1.10.1
        hooks:
          - id: knots          # default thresholds
          # - id: knots-verbose  # same thresholds, per-function detail
          # - id: knots-strict   # stricter: McCabe 10, Cognitive 10

Then install:

::

    pre-commit install

Custom thresholds via ``args``:

.. code-block:: yaml

        - id: knots
          args:
            - --mccabe-threshold=20
            - --cognitive-threshold=15
            - --aird-threshold=85

Exclude vendor/driver directories:

.. code-block:: yaml

        - id: knots
          args: [--mccabe-threshold=15, --cognitive-threshold=15]
          exclude: ^(Drivers/|Middlewares/|third_party/)

See ``example-pre-commit-config.yaml`` in the repository for a full example.

Combining with Other Tools
---------------------------

::

    # Run alongside cppcheck
    knots -r src/ > complexity.txt
    cppcheck src/ 2> cppcheck.txt

    # Find high-complexity functions that also have cppcheck warnings
    grep -f <(knots -r src/ | grep 😢 | cut -d' ' -f2) cppcheck.txt

Corpus Analysis
---------------

For large-scale analysis across many files, NDJSON is the most composable
format:

::

    # Analyze entire project, one record per function
    find . -name "*.c" -o -name "*.rs" | xargs knots --format ndjson > metrics.ndjson

    # Functions above AIRD threshold
    jq 'select(.aird > 85)' metrics.ndjson

    # Top 10 by cognitive complexity
    jq -s 'sort_by(-.cognitive) | .[0:10]' metrics.ndjson

    # Per-file average AIRD
    jq -s 'group_by(.file) | map({file: .[0].file, avg_aird: (map(.aird) | add / length)})' metrics.ndjson

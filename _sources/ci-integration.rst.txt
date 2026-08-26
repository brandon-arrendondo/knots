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
there. See :doc:`baseline` for details.

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
There are two ways to wire it up.

**Recommended — prebuilt wheel (fast first run).** Point pre-commit at the
``knots-pre-commit`` mirror repo. Its hooks are ``language: python`` and install
the prebuilt ``knots`` wheel from PyPI, so the first run is seconds with no Rust
toolchain and no from-source compile:

.. code-block:: yaml

    repos:
      - repo: https://github.com/brandon-arrendondo/knots-pre-commit
        rev: v1.16.0            # pin to a released knots version
        hooks:
          - id: knots          # default thresholds
          # - id: knots-verbose  # same thresholds, per-function detail
          # - id: knots-strict   # stricter: McCabe 10, Cognitive 10

**Alternative — build from source / bring your own binary.** The main knots repo
also ships hooks. ``id: knots`` there is ``language: rust`` and compiles knots on
first use (slow first run, but no PyPI dependency); the ``knots-system`` /
``knots-system-strict`` variants are ``language: system`` and run a ``knots``
already on your ``PATH`` (e.g. installed via ``pipx install knots`` — see
:doc:`installation`):

.. code-block:: yaml

    repos:
      - repo: https://github.com/brandon-arrendondo/knots
        rev: v1.16.0
        hooks:
          - id: knots          # language: rust — compiles from source
          # - id: knots-system   # language: system — uses knots from PATH

Then install:

::

    pre-commit install

Custom thresholds via ``args`` (works with either repo):

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

JavaScript / TypeScript / React projects:

.. code-block:: yaml

        - id: knots
          args:
            - --aird-threshold=85
            - --mccabe-threshold=20
            - --cognitive-threshold=25

The hooks already cover ``.js``, ``.jsx``, ``.ts``, and ``.tsx`` via
``types_or: [javascript, jsx, ts, tsx]`` — no override needed. To limit
coverage to a subset of languages, override ``types_or`` explicitly:

.. code-block:: yaml

        - id: knots
          args: [--mccabe-threshold=20]
          types_or: [python, javascript, jsx]   # Python + JS only, skip TS

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

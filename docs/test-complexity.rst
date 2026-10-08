Test Quality Analysis (knots-test-complexity)
=============================================

``knots-test-complexity`` is a C test-quality heuristic. It compares test and
source complexity and looks for boundary-testing patterns. It parses both
inputs with the C grammar, does not execute tests, and cannot prove coverage
or test adequacy.

Overview
--------

Use the ratios as review signals alongside executed tests and coverage,
not as evidence that every edge case is tested.

::

    knots-test-complexity test/test_battery.c src/battery.c

    # Custom thresholds
    knots-test-complexity \
      --threshold=0.70 \
      --boundary-threshold=0.80 \
      --level=error \
      test/test_timer.c src/timer.c

Key Features
------------

- **Complexity Ratio Analysis**: compares test complexity with source complexity
- **Boundary Value Detection**: looks for boundary patterns (0, MAX, overflow)
- **Ceedling Integration**: parses ``TEST_SOURCE_FILE`` macro to locate source files automatically
- **Pre-commit Integration**: enforce test quality standards at commit time

Pre-commit Hook
---------------

The hook wrapper defaults to naming-convention source lookup. Choose
``--framework=ceedling`` to locate sources using ``TEST_SOURCE_FILE`` macros:

.. code-block:: yaml

    repos:
      - repo: https://github.com/brandon-arrendondo/knots
        rev: v1.18.0
        hooks:
          # Main complexity check
          - id: knots
            args: [--mccabe-threshold=15, --cognitive-threshold=15]
            exclude: ^(Drivers/|Middlewares/)

          # Test quality validation (Ceedling projects)
          - id: test-complexity
            args:
              - --threshold=0.70
              - --boundary-threshold=0.80
              - --level=error
              - --framework=ceedling
              - --test-dir=Test

The wrapper parses ``TEST_SOURCE_FILE("path/to/source.c")`` from Ceedling test
files to locate the corresponding source automatically. Adjust ``--test-dir``
if your tests live in a non-default location (``test/``, ``Tests/``, ``tests/``).

For complete documentation, see ``knots-test-complexity/README.md`` in the
repository.

``--framework`` and ``--test-dir`` belong to the hook wrapper, not the
``knots-test-complexity`` binary. The binary takes test and source paths
explicitly. Its 0.70 ratio and 0.80 boundary thresholds are defaults with no
recorded calibration; see `ADR-0003
<https://github.com/brandon-arrendondo/knots/blob/main/docs/adr/0003-gate-defaults-have-a-recorded-basis.md>`_.

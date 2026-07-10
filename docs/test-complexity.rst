Test Quality Analysis (knots-test-complexity)
=============================================

``knots-test-complexity`` is a companion tool that validates unit tests have
sufficient complexity and boundary coverage to thoroughly exercise their
corresponding source files.

Overview
--------

Traditional code coverage can be misleading: 100% branch coverage doesn't
guarantee all edge cases are tested. ``knots-test-complexity`` enforces that
tests have adequate cyclomatic complexity *relative to* the source code they
cover, and validates that boundary conditions are explicitly tested.

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

- **Complexity Ratio Analysis**: ensures test complexity is proportional to source complexity
- **Boundary Value Detection**: validates tests cover critical boundary conditions (0, MAX, overflow)
- **Ceedling Integration**: parses ``TEST_SOURCE_FILE`` macro to locate source files automatically
- **Pre-commit Integration**: enforce test quality standards at commit time

Pre-commit Hook
---------------

The ``test-complexity`` hook is designed for the **Ceedling** test framework.
It automatically locates source files by parsing the ``TEST_SOURCE_FILE`` macro
in test files:

.. code-block:: yaml

    repos:
      - repo: https://github.com/brandon-arrendondo/knots
        rev: v1.15.0
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

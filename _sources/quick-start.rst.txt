Quick Start
===========

Single File
-----------

::

    knots src/main.c

Output shows per-function metrics::

    😊 init_system (McCabe: 3, Cognitive: 2, Nesting: 2, SLOC: 15, ABC: 4.12, Returns: 1, TestScore: 5, AIRD: 4, AICP: 8, ExtCalls: 2)
    😠 process_data (McCabe: 28, Cognitive: 45, Nesting: 8, SLOC: 120, ABC: 35.71, Returns: 7, TestScore: 18, AIRD: 87, AICP: 72, ExtCalls: 14)

    Summary:
      Total Functions: 2
      Average McCabe Complexity: 15.50
      Average AIRD Score: 45.50
      Average AICP Score: 40.00

Verbose Mode
------------

::

    knots -v src/main.c

Shows a detailed per-function breakdown including all test scoring sub-axes::

    Function: process_data 😠
      McCabe Complexity: 28
      Cognitive Complexity: 45
      Nesting Depth: 8
      SLOC: 120
      ABC Magnitude: 35.71
      Return Count: 7
      Test Scoring: 18 (Simple)
        - Signature: 3
        - Dependency: 5
        - Observable: 2
        - Implementation: 8
        - Documentation: 0
      AIRD Score: 87
      AICP Score: 72
      External Calls: 14
      Max Complexity: 45

Recursive Directory Analysis
-----------------------------

::

    knots -r ~/projects/myproject/

Recursive mode:

- Scans all ``.c`` / ``.cpp`` / ``.cc`` / ``.cxx`` / ``.rs`` / ``.py`` / ``.js`` / ``.mjs`` / ``.cjs`` files (headers skipped by default)
- Shows top 5 worst functions by complexity
- Displays totals and averages across all files
- Writes per-function detail to ``report.txt``
- Reports file processing statistics

Example output::

    === TOP 5 WORST FUNCTIONS ===

    1. 😢 HAL_RCC_OscConfig [drivers/hal_rcc.c]
       McCabe: 71, Cognitive: 214, Nesting: 11, SLOC: 327, ABC: 134.90, Returns: 23, TestScore: 9

    === TOTALS & AVERAGES ===

      Total Functions: 3404
      Average McCabe Complexity: 2.02
      Average Cognitive Complexity: 1.65

    Detailed per-function output written to report.txt

    === FILES PROCESSED ===

      Total files found: 165
      Successfully processed: 163
      Skipped (encoding/parse errors): 2

To include header files, use an include filter:

.. code-block:: json

    {
      "file_patterns": ["**/*.c", "**/*.h"]
    }

Compile Commands Integration
-----------------------------

Knots can analyze the file set from a ``compile_commands.json`` (CMake, Bear, Clang):

::

    knots --compile-commands build/compile_commands.json
    knots --compile-commands compile_commands.json -v
    knots --compile-commands compile_commands.json -m
    knots --compile-commands compile_commands.json --include filter.json

Generating ``compile_commands.json``:

::

    # CMake
    cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -B build

    # Makefile projects with Bear
    bear -- make

Testability Matrix
------------------

::

    knots -m src/module.c
    knots -r -m ~/projects/myproject/

The matrix places each function in one of four quadrants:

- **QUICK WINS** — low complexity, easy to test → automate testing
- **INVEST IN TESTS** — high complexity, easy to test → priority for unit tests
- **ADD DOCS** — low complexity, hard to test → needs better documentation
- **REFACTOR** — high complexity, hard to test → high risk, refactor first

Example output::

    === TESTABILITY MATRIX ===

    📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!
    =========================================================
      ✓ init_module [src/module.c] (McCabe: 2, TestScore: 3)

    🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!
    ========================================================
      ⛔ process_matrix [src/complex.c] (McCabe: 35, TestScore: 45)

Filtering
---------

Use JSON files to scope analysis to specific files or functions:

::

    knots -r . --include filter.json --exclude exclude.json

Filter schema:

.. code-block:: json

    {
      "file_patterns": [
        "src/**/*.c",
        "!**/vendor/**",
        "!**/test_*.c"
      ],
      "function_patterns": ["^process_.*", "^handle_.*"],
      "min_complexity": 10,
      "max_complexity": 50
    }

All fields are optional and combinable. ``file_patterns`` supports ``*``,
``**``, and ``!`` for negation. ``function_patterns`` are regular expressions.

Examples
--------

Quick health check::

    knots -r ~/myproject/ | head -20

Generate a comprehensive report::

    knots -r -v ~/myproject/
    less report.txt

Find high-complexity, hard-to-test functions::

    echo '{"min_complexity": 15}' > complex.json
    knots -r -m ~/myproject/ --include complex.json

Analyze only files modified in the last commit::

    git diff --name-only HEAD~1 | grep -E '\.(c|cpp|cc|cxx|rs|py|js|mjs|cjs)$' | while read f; do
        knots "$f"
    done

CMake/build system workflow::

    cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -B build
    knots --compile-commands build/compile_commands.json -m

    # For Makefile projects
    bear -- make clean all
    knots --compile-commands compile_commands.json -v

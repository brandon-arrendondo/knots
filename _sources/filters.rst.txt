============
Filter Rules
============

Knots supports filtering files and functions using JSON-based filter rules. Use
``--include`` to whitelist what to analyze, and ``--exclude`` to blacklist what
to skip.

Filters decide *what gets analyzed*; the :doc:`baseline <baseline>` decides
*what counts as a failure*.

Usage
-----

.. code-block:: bash

    # Include only specific files/functions
    knots -r /path/to/code --include filter-include.json

    # Exclude specific files/functions
    knots -r /path/to/code --exclude filter-exclude.json

    # Combine both (include takes precedence, then exclude is applied)
    knots -r /path/to/code --include filter-include.json --exclude filter-exclude.json

JSON Schema
-----------

Filter JSON files support the following fields (all are optional):

.. code-block:: json

    {
      "file_patterns": ["pattern1", "pattern2", "!negation"],
      "function_patterns": ["regex1", "regex2"],
      "min_complexity": 5,
      "max_complexity": 50
    }

``file_patterns`` (array of strings)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Glob-style patterns for matching file paths. Supports:

- ``*`` — matches zero or more characters, including ``/`` with the current matcher
- ``**`` — matches any characters including ``/``
- ``!pattern`` — negation (exclude files matching this pattern)

**Examples:**

- ``"src/**/*.c"`` — all ``.c`` files in ``src/`` and subdirectories
- ``"lib/*.c"`` — ``.c`` files under ``lib/``, including nested directories
- ``"!**/test_*.c"`` — exclude files starting with ``test_``
- ``"!**/vendor/**"`` — exclude everything in vendor directories

**Behavior:**

- Both filters use the same predicate: match at least one positive pattern
  (if any), and match no negated pattern. A negative-only list matches every
  path except those negated.
- An include filter keeps matching files; an exclude filter removes matching
  files. Use positive patterns to exclude directories. A negative-only
  exclude list would remove their complement instead.

``function_patterns`` (array of strings)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Regular expression patterns for matching function names.

**Examples:**

- ``"^process_.*"`` — functions starting with ``process_``
- ``".*_handler$"`` — functions ending with ``_handler``
- ``"^(init|setup|cleanup)_.*"`` — functions starting with ``init_``,
  ``setup_``, or ``cleanup_``

**Behavior:**

- **Include filter:** function must match at least one pattern
- **Exclude filter:** function matching any pattern is excluded

``min_complexity`` (number)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Minimum complexity threshold (inclusive). Functions with complexity below this
are filtered out.

- Complexity is calculated as ``max(McCabe, Cognitive)``
- Default: no minimum (accepts all)

``max_complexity`` (number)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Maximum complexity threshold (inclusive). Functions with complexity above this
are filtered out.

- Complexity is calculated as ``max(McCabe, Cognitive)``
- Default: no maximum (accepts all)

Examples
--------

Example 1: Analyze only high-complexity functions
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``filter-high-complexity.json``:

.. code-block:: json

    {
      "min_complexity": 20
    }

.. code-block:: bash

    knots -r . --include filter-high-complexity.json

Example 2: Exclude test files and test functions
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``filter-no-tests.json``:

.. code-block:: json

    {
      "file_patterns": [
        "**/test_*.c",
        "**/*_test.c",
        "**/tests/**"
      ],
      "function_patterns": [
        "^test_.*"
      ]
    }

.. code-block:: bash

    knots -r . --exclude filter-no-tests.json

Example 3: Focus on specific modules with concerning complexity
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``filter-focus.json``:

.. code-block:: json

    {
      "file_patterns": [
        "src/core/**/*.c",
        "src/drivers/**/*.c",
        "!**/vendor/**"
      ],
      "min_complexity": 10,
      "max_complexity": 50
    }

.. code-block:: bash

    knots -r . --include filter-focus.json

Example 4: Exclude vendor code and simple functions
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: bash

    # Create exclude filter
    cat > filter-exclude.json << 'EOF'
    {
      "file_patterns": [
        "**/vendor/**",
        "**/third_party/**",
        "**/Middlewares/**"
      ],
      "function_patterns": [
        "^HAL_.*",
        "^__.*"
      ],
      "max_complexity": 5
    }
    EOF

    knots -r . --exclude filter-exclude.json

Filter Logic
------------

Include filter (whitelist)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When ``--include`` is specified:

1. Files must match ``file_patterns`` (if specified)
2. Functions must match ``function_patterns`` (if specified)
3. Complexity must be within min/max bounds (if specified)

If any criterion fails, the file/function is skipped.

Exclude filter (blacklist)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When ``--exclude`` is specified:

1. If a file matches ``file_patterns``, it is excluded
   during discovery, independently of function criteria
2. For remaining files, functions must match all specified function
   criteria to be excluded: name patterns AND complexity bounds when both
   are supplied; either criterion alone is sufficient when the other is absent

Combined filters
~~~~~~~~~~~~~~~~~

When both ``--include`` and ``--exclude`` are specified:

1. Include filter is applied first (whitelist)
2. Exclude filter is applied second (blacklist from the whitelist)

This allows you to say "analyze only ``core/``" (include) and then "but skip
test files" (exclude).

Notes
-----

- All filter criteria are optional — specify only the ones you need.
- Empty arrays mean "match everything" for that criterion. In the current
  CLI, an exclude file with missing or empty ``file_patterns`` therefore
  excludes every file during discovery, even if it specifies function
  patterns or bounds. For function-only exclusions, use ``knots.toml``
  (see :doc:`config`). This is a discovery limitation, not an AND relation
  between file and function criteria.
- File patterns are matched against the full file path.
- Function patterns use Rust's regex syntax.
- Invalid regex patterns are silently ignored (the function won't match).

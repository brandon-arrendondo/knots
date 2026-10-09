================================
knots.toml & Inline Suppression
================================

In addition to the JSON :doc:`filter rules <filters>`, knots reads an optional
``knots.toml`` file for thresholds and exclusions, and recognizes inline
``tools:off`` / ``tools:suppress`` comments for per-function suppression. Both
surfaces come from the shared ``lang-parsing-substrate`` crate, so the same
comment syntax is shared across knots, moldy, and aurora-lint; each
consumer decides which configuration features it implements.

CLI threshold flags override TOML thresholds. JSON include/exclude filters
and TOML exclusions are applied together; JSON does not replace TOML exclusions.

``knots.toml`` discovery
------------------------

knots walks up from the current working directory looking for ``knots.toml``,
stopping at the first match. If none is found, knots behaves exactly as
before (CLI flags and JSON filters only).

Thresholds
----------

.. code-block:: toml

    [thresholds]
    mccabe    = 10
    cognitive = 15
    nesting   = 5

    # Per-language overrides — take precedence over [thresholds] for files
    # of that language, but are still overridden by a CLI flag.
    [c.thresholds]
    mccabe = 15   # C idioms inflate cyclomatic complexity vs higher-level languages

Resolution order for each metric, per function: **CLI flag** > **per-language
config** (``[<lang>.thresholds]``) > **global config** (``[thresholds]``) >
off.

Available keys: ``mccabe``, ``cognitive``, ``nesting``, ``sloc``, ``abc``,
``returns``, ``aird``, ``aicp``, ``external_calls``, ``unreachable_blocks``
— these are configuration keys, not literal CLI spellings. For example,
``returns`` maps to ``--return-threshold`` and ``external_calls`` to
``--external-calls-threshold``. ``unreachable_blocks`` is only ever nonzero
for C/C++/Rust files.

File/function exclusion
------------------------

.. code-block:: toml

    [[filter.exclude]]
    file_patterns     = ["tests/**"]
    function_patterns = ["^test_"]

    [[filter.exclude]]
    function_patterns = ["^HAL_"]

Within one ``[[filter.exclude]]`` entry, ``file_patterns`` and
``function_patterns`` are AND'd together; an omitted list matches everything
for that field. A function is excluded if it matches *any* entry. This runs
independently of (and in addition to) ``--include``/``--exclude`` JSON
filters.

Inline suppression
-------------------

Two comment forms, recognized in every supported language using that
language's own comment syntax (``//``, ``#``, ``--``, ``!``):

**Single-line** — suppresses one metric for the function containing (or
immediately following) the comment:

.. code-block:: rust

    // tools:suppress knots:cognitive JUSTIFICATION:"legacy dispatch table; reviewed exception"
    fn big_function() { /* ... */ }

**Block region** — suppresses every knots metric for everything between the
markers:

.. code-block:: c

    /* tools:off knots */
    int legacy_blob() { /* ... */ }
    /* tools:on */

    /* tools:off */          /* no tool qualifier — suppresses every tool, not just knots */
    ...
    /* tools:on */

``tools:off [TOOL[,TOOL,...]]`` accepts a comma-separated tool list
(``knots``, ``funky``/``moldy``, ``sqc``); omitting it suppresses every tool
for that region. An unclosed ``tools:off`` extends to end of file.

Notes
-----

- ``knots.toml`` and inline suppression are both entirely optional — omit
  either (or both) and knots behaves as it always has.
- Suppression is resolved once per file during analysis and attached to each
  ``FunctionMetrics`` as ``suppressed`` — suppressed metrics are not checked
  against thresholds. Their measured values remain in structured output
  and baseline snapshots; suppression does not remove score records.
- See ``lang-parsing-substrate``'s ``docs/unified-config-spec.md`` for the
  full cross-tool spec (``suppress.toml``, per-tool config files, etc.) —
  this page documents only what knots currently implements.

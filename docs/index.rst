================
Knots User Guide
================

Knots is a fast multi-language code complexity analyzer built on tree-sitter.
It measures traditional complexity metrics alongside two AI-specific cost
scores — AIRD (AI Reasoning Difficulty) and AICP (AI Context Pressure) —
to identify which functions are genuinely expensive to modify with AI
assistance.

.. toctree::
   :maxdepth: 3
   :caption: Contents

   installation
   quick-start
   cli-reference
   metrics-reference
   output-formats
   ci-integration
   baseline
   filters
   config
   test-complexity
   alternatives
   benchmarks
   architecture
   troubleshooting
   releasing

Contributor references
----------------------

The `Architecture Decision Records
<https://github.com/brandon-arrendondo/knots/blob/main/docs/adr/README.md>`_
explain metric definitions and gate decisions. They remain Markdown documents
in the repository. See `AGENTS.md
<https://github.com/brandon-arrendondo/knots/blob/main/AGENTS.md>`_ for contributor
instructions and `CLAUDE.md
<https://github.com/brandon-arrendondo/knots/blob/main/CLAUDE.md>`_ for the technical
guide.

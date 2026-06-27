Releasing
=========

This page documents the maintainer release process: how a tagged version becomes
prebuilt PyPI wheels and a matching ``knots-pre-commit`` release.

Overview
--------

- The version is single-sourced in ``Cargo.toml`` under
  ``[workspace.package].version`` (both crates inherit it).
- Pushing a ``v*.*.*`` tag to the ``knots`` repo triggers
  ``.github/workflows/wheels.yml``, which builds per-platform wheels + sdists for
  **knots** and **knots-test-complexity** with maturin and publishes them to PyPI
  via Trusted Publishing (OIDC — no API token).
- The separate `knots-pre-commit
  <https://github.com/brandon-arrendondo/knots-pre-commit>`_ repo is a pure-Python
  shim that pins ``knots==<version>``; its ``language: python`` hooks install the
  prebuilt wheel so users get a zero-compile first run. It is tagged to match each
  knots release. See :doc:`ci-integration` for the user-facing setup.

One-time setup
--------------

On PyPI, configure a **Trusted Publisher** for each project (``knots`` and
``knots-test-complexity``). Until a project's first publish, add it as a *pending*
publisher at https://pypi.org/manage/account/publishing/ with:

============================  =================
PyPI Project Name             ``knots`` / ``knots-test-complexity``
Owner                         ``brandon-arrendondo``
Repository name               ``knots``
Workflow name                 ``wheels.yml``
Environment name              ``release``
============================  =================

The **PyPI Project Name must exactly match** the package name in each crate's
``pyproject.toml`` (normalized) — a mismatch yields a ``400 Non-user identities
cannot create new projects`` error at publish time.

Release steps
-------------

``X.Y.Z`` is the new version.

1. **Bump the version.** Edit ``[workspace.package].version`` in ``Cargo.toml``,
   update the documented ``rev:`` pins (``docs/``, ``example-pre-commit-config.yaml``),
   then ``cargo build`` to refresh ``Cargo.lock``. Commit.

2. **Tag and push knots.** This publishes both wheels:

   .. code-block:: bash

       git tag vX.Y.Z
       git push --follow-tags

   ``wheels.yml`` builds manylinux + musllinux (x86_64, aarch64), macOS
   (x86_64 cross-compiled on Apple Silicon, aarch64), and Windows x64 wheels plus
   sdists, then publishes to PyPI. Watch it under the repo's Actions tab.

3. **Bump and tag the mirror.** Once knots is on PyPI, point the shim at it:

   .. code-block:: bash

       cd ../knots-pre-commit
       ./bump.sh X.Y.Z
       git commit -am "knots X.Y.Z"
       git tag vX.Y.Z
       git push origin main vX.Y.Z      # push the tag explicitly

   **Order matters:** the mirror's ``knots==X.Y.Z`` pin must reference a version
   already on PyPI, so always publish knots (step 2) **before** tagging the mirror.

Verifying a release
-------------------

.. code-block:: bash

    # Wheel installs from PyPI with the binary on PATH
    pipx run --spec "knots==X.Y.Z" knots --version

    # End-to-end zero-compile pre-commit path
    pre-commit try-repo https://github.com/brandon-arrendondo/knots-pre-commit \
        knots --ref vX.Y.Z --files path/to/source.rs

Notes & recovery
----------------

- The publish step uses ``skip-existing: true``, so re-runs are idempotent:
  already-published files are skipped rather than failing on "file already exists".
- **Partial publish** (one package uploaded, the other failed — e.g. a
  trusted-publisher misconfiguration): fix the cause, then re-push the tag
  (``git push -f origin vX.Y.Z``). The rebuild re-runs publish; the package already
  on PyPI is skipped and the missing one uploads.
- PyPI versions are immutable — a published version cannot be re-uploaded or
  overwritten. To ship a fix, cut a new version.
- ``wheels.yml`` can be run manually (Actions → "Wheels (PyPI)" → Run workflow) to
  validate the build matrix without publishing (publish is gated to tags).

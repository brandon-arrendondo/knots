# Using knots with pre-commit Framework

This guide is for using knots with the [pre-commit](https://pre-commit.com/) framework (Python-based).

**Note:** This is different from the standalone shell hooks in this directory. If you're not using the pre-commit framework, see [README.md](README.md) instead.

<!-- Third-party versions in this file (the GitHub Action) and in
example-pre-commit-config.yaml were checked against each project's release
page on 2026-11-03. Refresh them,
and the knots `rev:` pins, at each knots release (docs/releasing.rst,
step 1). -->

## What is pre-commit?

pre-commit is a framework for managing git pre-commit hooks. It's popular in Python projects but works with any language.

Website: https://pre-commit.com/

## Quick Start

### 1. Install pre-commit Framework

```bash
# Using pip
pip install pre-commit

# Or using your package manager
# macOS: brew install pre-commit
# Ubuntu: apt install pre-commit
```

### 2. Create .pre-commit-config.yaml

In your project root, create or edit `.pre-commit-config.yaml`. The
recommended setup uses the `knots-pre-commit` mirror, whose hooks install the
prebuilt knots wheel from PyPI, so there is no Rust toolchain to install and
nothing to compile:

```yaml
repos:
  # ... your other hooks ...

  - repo: https://github.com/brandon-arrendondo/knots-pre-commit
    rev: v2.0.0
    hooks:
      - id: knots
```

The knots repository publishes the same hooks, built from source on first
use, plus variants that use a knots already on `PATH`; see
[Published Hooks](#published-hooks). `docs/ci-integration.rst` describes both
in full.

To run the shell wrapper from a local knots checkout instead (see
[The Wrapper Script](#the-wrapper-script)):

```yaml
repos:
  - repo: local
    hooks:
      - id: knots
        name: Code Complexity Check
        entry: /path/to/knots/hooks/pre-commit-wrapper.sh
        language: script
        files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
        pass_filenames: true
```

The `files:` pattern lists the extensions knots analyses
(`knots --supported-languages`). Narrow it to the languages you use.

### 3. Install the Hooks

```bash
pre-commit install
```

### 4. Done!

Now pre-commit will run knots automatically on commit.

## Published Hooks

The `knots-pre-commit` mirror publishes `knots`, `knots-verbose` and
`knots-strict`, installing knots from its PyPI wheel. The knots repository's
own `.pre-commit-hooks.yaml` publishes these hooks. Each runs `knots` itself
on the staged files:

| Hook id | Installs knots by | Default arguments |
|---------|-------------------|-------------------|
| `knots` | building it with cargo | `--aird-threshold=85` |
| `knots-verbose` | building it with cargo | `--verbose --aird-threshold=85` |
| `knots-strict` | building it with cargo | McCabe 10, Cognitive 10, Nesting 3, SLOC 30, ABC 20.0, Returns 3 |
| `knots-system` | none: uses `knots` on `PATH` | `--aird-threshold=85` |
| `knots-system-strict` | none: uses `knots` on `PATH` | as `knots-strict` |

pre-commit replaces a hook's `args:` with yours rather than adding to them,
so setting `args:` replaces the defaults:

```yaml
  - repo: https://github.com/brandon-arrendondo/knots
    rev: v2.0.0
    hooks:
      - id: knots
        args: [--mccabe-threshold=15, --cognitive-threshold=15]
        exclude: ^(vendor/|third_party/)
```

## The Wrapper Script

`hooks/pre-commit-wrapper.sh` runs knots on each file it is given and checks
six metrics against thresholds of its own. It reads knots' `--format ndjson`
records, so every threshold applies whatever else is set.

| Metric | Option | Default |
|--------|--------|---------|
| McCabe | `--mccabe-threshold` | 15 |
| Cognitive | `--cognitive-threshold` | 15 |
| Nesting | `--nesting-threshold` | 5 |
| SLOC | `--sloc-threshold` | 50 |
| ABC | `--abc-threshold` | 10.0 |
| Returns | `--return-threshold` | 3 |

An option you leave out keeps its default. `--verbose` also prints a summary
for each file that passes. `--validator-path=PATH` (or the `VALIDATOR_PATH`
environment variable) names the knots binary; the default is `knots` on
`PATH`.

Options take either form: `--mccabe-threshold 15` or `--mccabe-threshold=15`.

### Standard

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      pass_filenames: true
```

**Output:** Shows only violations (functions exceeding thresholds). Passes silently if no issues found.

### Verbose

```yaml
- repo: local
  hooks:
    - id: knots-verbose
      name: Code Complexity Check (Verbose)
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh --verbose
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      pass_filenames: true
```

**Output:** Shows violations (if any) and a summary of all files checked, even when they pass.

### Stricter Thresholds

```yaml
- repo: local
  hooks:
    - id: knots-strict
      name: Code Complexity Check (Strict)
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      pass_filenames: true
      args:
        - --mccabe-threshold=10
        - --cognitive-threshold=10
        - --nesting-threshold=3
        - --sloc-threshold=30
        - --abc-threshold=20.0
        - --return-threshold=3
```

These match the published `knots-strict` hook.

## Custom Configuration

### Coming from flake8

flake8's `--max-complexity=N` (its bundled McCabe check) corresponds to knots'
`--mccabe-threshold=N`. knots' other thresholds (cognitive, nesting, SLOC,
ABC, returns, AIRD) have no counterpart in flake8 itself.

### Custom Thresholds

Set only the thresholds you want to change; the others keep their defaults:

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      pass_filenames: true
      args:
        - --mccabe-threshold=12
        - --cognitive-threshold=18
        - --return-threshold=4
```

### Custom File Pattern

Restrict to a specific language subset (e.g., only C source files):

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.c$  # Only .c files
      pass_filenames: true
```

Or restrict to Rust only:

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.rs$
      pass_filenames: true
```

### Exclude Certain Files

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      exclude: ^(vendor/|third_party/)  # Exclude vendor code
      pass_filenames: true
```

## Complete Example .pre-commit-config.yaml

```yaml
# See https://pre-commit.com for more information
repos:
  # ... your other hooks ...

  - repo: https://github.com/brandon-arrendondo/knots-pre-commit
    rev: v2.0.0
    hooks:
      - id: knots
        exclude: ^(vendor/|third_party/)  # Exclude third-party code
        args:
          - --aird-threshold=85
          - --mccabe-threshold=15
          - --cognitive-threshold=15
          - --nesting-threshold=5
          - --sloc-threshold=50
          - --return-threshold=3
```

Run `pre-commit autoupdate` to move each `rev:` in your configuration to its
repository's latest tag.

## Usage

### Run Automatically on Commit

```bash
git add file.c
git commit -m "Add feature"
# pre-commit runs automatically
```

### Run Manually on All Files

```bash
pre-commit run --all-files
```

### Run Manually on Specific Files

```bash
pre-commit run --files src/file1.c src/file2.c
```

### Run Specific Hook

```bash
pre-commit run knots --all-files
```

### Skip Hooks (Emergency)

```bash
git commit --no-verify -m "Emergency fix"
```

## Comparison: pre-commit Framework vs Standalone Hooks

| Feature | pre-commit Framework | Standalone Hooks |
|---------|---------------------|------------------|
| **Installation** | Requires Python + pre-commit | Just copy script |
| **Configuration** | YAML file | git config |
| **Multiple Hooks** | Easy to manage many hooks | One hook file |
| **Runtime** | Python (pre-commit) | A POSIX shell (`sh`) |
| **Auto-update** | `pre-commit autoupdate` | Manual |
| **Team Distribution** | Commit config file | Commit hook scripts |
| **CI Integration** | `pre-commit run --all-files` | Run hook directly |
| **Best For** | Teams using pre-commit | Simple, focused setup |

## When to Use Which?

### Use pre-commit Framework When:
- ✓ Already using pre-commit in your project
- ✓ Need to manage multiple hooks (linters, formatters, etc.)
- ✓ Want centralized configuration
- ✓ Team is familiar with pre-commit

### Use Standalone Hooks When:
- ✓ Don't want Python dependency
- ✓ Want simplest possible setup
- ✓ Only need complexity checking
- ✓ Prefer git config over YAML

## Troubleshooting

### Hook Not Running

```bash
# Check if pre-commit is installed
pre-commit --version

# Reinstall hooks
pre-commit uninstall
pre-commit install

# Test manually
pre-commit run knots --all-files
```

### knots Not Found

When it is given files and cannot find knots, the wrapper fails with
"knots not found". Point it at the binary:

```bash
# Set path via environment variable
export VALIDATOR_PATH=/path/to/knots

# Or modify .pre-commit-config.yaml
- repo: local
  hooks:
    - id: knots
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh --validator-path=/path/to/knots
      language: script
```

The published `knots-system` hooks also fail when `knots` is not on `PATH`.

### Changes Not Detected

```bash
# Update pre-commit cache
pre-commit clean
pre-commit install --install-hooks
```

### Want to See More Output

Add `--verbose` to args or set `verbose: true`:

```yaml
- repo: local
  hooks:
    - id: knots
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: '(?i)\.(c|h|cpp|cc|cxx|hpp|hxx|rs|py|js|mjs|cjs|jsx|ts|tsx|adb|ada|ads|go|java|cs|kt|kts|swift|php|f90|f95|f03|f08|scala|sc|lua)$'
      verbose: true
      args:
        - --verbose
```

## CI/CD Integration

### GitHub Actions

GitHub's Ubuntu runners include pipx, so pre-commit can run through it with
no setup step. The mirror's hooks install knots themselves (the knots
repository's `knots` hook builds it with the runner's Rust toolchain):

```yaml
name: pre-commit

on:
  pull_request:
  push:
    branches: [main]

jobs:
  pre-commit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7

      - run: pipx run pre-commit run --all-files --show-diff-on-failure
```

### GitLab CI

The `python` image has no Rust toolchain, so use the mirror's hooks, which
install the prebuilt knots wheel, or install it yourself (`pip install knots`)
for the knots repository's `knots-system` hook:

```yaml
pre-commit:
  image: python:3
  before_script:
    - pip install pre-commit
  script:
    - pre-commit run --all-files
```

## Advanced: Multiple Profiles

Different thresholds for different parts of codebase:

```yaml
repos:
  # Strict for new code
  - repo: local
    hooks:
      - id: knots-strict
        name: Code Complexity (Strict - New Code)
        entry: /path/to/knots/hooks/pre-commit-wrapper.sh
        language: script
        files: ^src/new/.*\.(c|h)$
        pass_filenames: true
        args:
          - --mccabe-threshold=10
          - --cognitive-threshold=10

  # Lenient for legacy code
  - repo: local
    hooks:
      - id: knots-legacy
        name: Code Complexity (Lenient - Legacy)
        entry: /path/to/knots/hooks/pre-commit-wrapper.sh
        language: script
        files: ^src/legacy/.*\.(c|h)$
        pass_filenames: true
        args:
          - --mccabe-threshold=20
          - --cognitive-threshold=20
```

## Migration from Standalone Hooks

Already using standalone hooks? Easy migration:

```bash
# 1. Install pre-commit framework
pip install pre-commit

# 2. Create .pre-commit-config.yaml (see examples above)

# 3. Uninstall old hook
./hooks/install-hook.sh --uninstall

# 4. Install pre-commit
pre-commit install

# 5. Test
pre-commit run --all-files
```

## Resources

- **pre-commit documentation:** https://pre-commit.com/
- **Hook repository:** https://pre-commit.com/hooks.html
- **This project's hooks:** See `.pre-commit-hooks.yaml`

---

**Questions?** See the main [README.md](README.md) for general hook documentation.

# Using knots with pre-commit Framework

This guide is for using knots with the [pre-commit](https://pre-commit.com/) framework (Python-based).

**Note:** This is different from the standalone bash hooks in this directory. If you're not using the pre-commit framework, see [README.md](README.md) instead.

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

In your project root, create or edit `.pre-commit-config.yaml`:

```yaml
repos:
  # ... your other hooks ...
  
  - repo: local
    hooks:
      - id: knots
        name: Code Complexity Check
        entry: /path/to/knots/hooks/pre-commit-wrapper.sh
        language: script
        files: \.(c|h)$
        pass_filenames: true
```

**Or if knots is published:**

```yaml
repos:
  - repo: https://github.com/brandon-arrendondo/knots
    rev: v0.2.0  # Use specific version
    hooks:
      - id: knots
```

### 3. Install the Hooks

```bash
pre-commit install
```

### 4. Done!

Now pre-commit will run knots automatically on commit.

## Available Hooks

### Standard Hook

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      pass_filenames: true
```

**Thresholds:** McCabe: 15, Cognitive: 15, Nesting: 5, SLOC: 50, ABC: 10.0, Returns: 3

**Output:** Shows only violations (functions exceeding thresholds). Passes silently if no issues found.

### Verbose Hook

```yaml
- repo: local
  hooks:
    - id: knots-verbose
      name: Code Complexity Check (Verbose)
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh --verbose
      language: script
      files: \.(c|h)$
      pass_filenames: true
```

**Output:** Shows violations (if any) and a summary of all files checked, even when they pass.

**Note:** Both hooks internally run `knots -v` to get detailed per-function metrics for parsing. The `--verbose` flag controls whether to show summaries for passing files, not the knots output format.

### Strict Hook

```yaml
- repo: local
  hooks:
    - id: knots-strict
      name: Code Complexity Check (Strict)
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh --mccabe-threshold 10 --cognitive-threshold 10
      language: script
      files: \.(c|h)$
      pass_filenames: true
```

**Thresholds:** McCabe: 10, Cognitive: 10, Returns: 3

## Custom Configuration

### Comparison with flake8 Configuration

If you're familiar with flake8, here's how knots arguments map to flake8 arguments:

| flake8 | knots | Default |
|--------|-------|---------|
| `--max-complexity=15` | `--mccabe-threshold=15` | 15 |
| `--max-cognitive-complexity=15` | `--cognitive-threshold=15` | 15 |
| `--max-returns=3` | `--return-threshold=3` | 3 |
| N/A | `--nesting-threshold=5` | 5 |
| N/A | `--sloc-threshold=50` | 50 |
| N/A | `--abc-threshold=10.0` | 10.0 |

**Example flake8 config:**
```yaml
- repo: https://github.com/PyCQA/flake8
  rev: 7.3.0
  hooks:
    - id: flake8
      args:
        - --max-complexity=15
        - --max-cognitive-complexity=15
        - --max-returns=3
      exclude: ^(vendor/|third_party/)
```

**Equivalent knots config:**
```yaml
- repo: local
  hooks:
    - id: knots
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      exclude: ^(vendor/|third_party/)
      pass_filenames: true
      args:
        - --mccabe-threshold=15
        - --cognitive-threshold=15
        - --return-threshold=3
```

### Custom Thresholds

Configure all available thresholds (similar to flake8):

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      pass_filenames: true
      args:
        - --mccabe-threshold=15
        - --cognitive-threshold=15
        - --return-threshold=3
        - --nesting-threshold=5
        - --sloc-threshold=50
        - --abc-threshold=10.0
```

Or customize specific thresholds only:

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      pass_filenames: true
      args:
        - --mccabe-threshold=12
        - --cognitive-threshold=18
        - --return-threshold=4
```

### Custom File Pattern

Only check .c files (not .h):

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

### Exclude Certain Files

```yaml
- repo: local
  hooks:
    - id: knots
      name: Code Complexity Check
      entry: /path/to/knots/hooks/pre-commit-wrapper.sh
      language: script
      files: \.(c|h)$
      exclude: ^(vendor/|third_party/)  # Exclude vendor code
      pass_filenames: true
```

## Complete Example .pre-commit-config.yaml

```yaml
# See https://pre-commit.com for more information
# See https://pre-commit.com/hooks.html for more hooks
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-added-large-files

  # Code complexity checking (similar to flake8 configuration)
  - repo: local
    hooks:
      - id: knots
        name: Code Complexity Check
        entry: /path/to/knots/hooks/pre-commit-wrapper.sh
        language: script
        files: \.(c|h)$
        exclude: ^(vendor/|third_party/)  # Exclude third-party code
        pass_filenames: true
        args:
          - --mccabe-threshold=15
          - --cognitive-threshold=15
          - --return-threshold=3
          - --nesting-threshold=5
          - --sloc-threshold=50
          - --abc-threshold=10.0
```

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
| **Language Support** | Many languages | Just bash |
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

```bash
# Set path via environment variable
export VALIDATOR_PATH=/path/to/knots

# Or modify .pre-commit-config.yaml
- repo: local
  hooks:
    - id: knots
      entry: bash -c 'VALIDATOR_PATH=/path/to/knots /path/to/hooks/pre-commit-wrapper.sh "$@"'
      language: script
```

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
      files: \.(c|h)$
      verbose: true
      args:
        - --verbose
```

## CI/CD Integration

### GitHub Actions

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
      - uses: actions/checkout@v3
      
      - uses: actions/setup-python@v4
        with:
          python-version: '3.x'
      
      # Install knots
      - name: Install knots
        run: |
          cargo install knots
          # Or: curl -L https://github.com/brandon-arrendondo/knots/releases/download/v0.2.0/knots-linux-x64 -o /usr/local/bin/knots && chmod +x /usr/local/bin/knots
      
      - uses: pre-commit/action@v3.0.0
```

### GitLab CI

```yaml
pre-commit:
  image: python:3.11
  before_script:
    - pip install pre-commit
    - cargo install knots  # Or install from release
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

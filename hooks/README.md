# Knots Pre-Commit Hook

Automatically check code complexity before commits are allowed.

## Quick Start

### Option 1: Automatic Installation (Recommended)

```bash
# Copy hook files to your project
cp pre-commit pre-commit-warning-only install-hook.sh /path/to/your/project/

# Run installer
cd /path/to/your/project
./install-hook.sh
```

### Option 2: Manual Installation

```bash
# Copy the hook
cp pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Hook Variants

### 1. Blocking Hook (`pre-commit`)
**Rejects commits** that exceed complexity thresholds.

**Use when:** You want to enforce strict complexity limits

**Installation:**
```bash
./install-hook.sh
```

### 2. Warning-Only Hook (`pre-commit-warning-only`)
**Warns** about complexity but allows all commits.

**Use when:** 
- Getting started with complexity checks
- Working on legacy code
- Want visibility without enforcement

**Installation:**
```bash
./install-hook.sh --warning-only
```

## Configuration

All configuration is done via git config:

```bash
# Set McCabe complexity threshold (default: 15)
git config hooks.knots.mccabe-threshold 15

# Set Cognitive complexity threshold (default: 15)
git config hooks.knots.cognitive-threshold 15

# Set Nesting depth threshold (default: 5)
git config hooks.knots.nesting-threshold 5

# Set SLOC threshold (default: 50)
git config hooks.knots.sloc-threshold 50

# Set ABC magnitude threshold (default: 10.0)
git config hooks.knots.abc-threshold 10.0

# Set Return count threshold (default: 3)
git config hooks.knots.return-threshold 3

# Enable verbose output (shows per-function details in multi-line format)
# Non-verbose shows single-line format per function
# Note: Summary statistics are always shown regardless of verbose mode
git config hooks.knots.verbose true

# Set custom knots path
git config hooks.knots.path /usr/local/bin/knots

# View current configuration
git config --get-regexp hooks.knots
```

### Per-Repository vs Global

```bash
# Per-repository (default)
git config hooks.knots.mccabe-threshold 10

# Global (all your repos)
git config --global hooks.knots.mccabe-threshold 10
```

## Usage Examples

### Example 1: Commit with Low Complexity (Passes)

```bash
$ git commit -m "Add simple helper function"
Running knots on C files...

✓ All complexity checks passed
[main abc1234] Add simple helper function
 1 file changed, 10 insertions(+)
```

### Example 2: Commit with High Complexity (Blocked)

```bash
$ git commit -m "Add complex function"
Running knots on C files...

✗ src/processor.c:process_data
  McCabe Complexity: 18 (threshold: 15)
✗ src/processor.c:process_data
  Cognitive Complexity: 22 (threshold: 15)
  Nesting Depth: 4
  SLOC: 45
  Return Count: 2

Commit rejected: Complexity thresholds exceeded

To adjust thresholds, run:
  git config hooks.knots.mccabe-threshold <value>
  git config hooks.knots.cognitive-threshold <value>
  git config hooks.knots.nesting-threshold <value>
  git config hooks.knots.sloc-threshold <value>
  git config hooks.knots.abc-threshold <value>
  git config hooks.knots.return-threshold <value>

To bypass this check (not recommended), use:
  git commit --no-verify
```

### Example 3: Bypass Hook (Emergency Override)

```bash
# Skip the hook entirely
git commit --no-verify -m "Emergency fix"
```

## What Gets Checked?

The hook checks:
- All `.c` files being committed
- All `.h` files being committed
- Only files that are staged (in `git add`)

Files NOT checked:
- Deleted files
- Renamed files (only if content changed)
- Files not staged

## Threshold Guidelines

### McCabe Complexity
| Threshold | Meaning |
|-----------|---------|
| 1-10 | Simple, low risk |
| 11-15 | More complex, moderate risk |
| 16-20 | Complex, high risk |
| 21+ | Very complex, very high risk |

**Recommended:** 15 (standard industry practice)

### Cognitive Complexity
| Threshold | Meaning |
|-----------|---------|
| 1-10 | Easy to understand |
| 11-15 | Moderate difficulty |
| 16-20 | Hard to understand |
| 21+ | Very hard to understand |

**Recommended:** 15 (matches SonarSource guidelines)

## Common Workflows

### Workflow 1: Start with Warnings, Graduate to Blocking

```bash
# Phase 1: Install warning-only hook
./install-hook.sh --warning-only

# (Team gets used to seeing warnings for a few weeks)

# Phase 2: Switch to blocking hook
./install-hook.sh
```

### Workflow 2: Strict from the Start

```bash
# Install blocking hook with strict thresholds
./install-hook.sh
git config hooks.knots.mccabe-threshold 10
git config hooks.knots.cognitive-threshold 10
```

### Workflow 3: Legacy Codebase

```bash
# Use warning-only permanently
./install-hook.sh --warning-only

# Higher thresholds to reduce noise
git config hooks.knots.mccabe-threshold 20
git config hooks.knots.cognitive-threshold 20
```

## Team Setup

### Distributing Hooks to Your Team

**Option 1: Include in Repository**

```bash
# Add to repo (not .git/hooks, which is local)
mkdir -p tools/hooks
cp pre-commit tools/hooks/
cp install-hook.sh tools/hooks/

# Team members run:
cd tools/hooks && ./install-hook.sh
```

**Option 2: Setup Script in Root**

```bash
# Create setup.sh in project root
cat > setup.sh << 'SETUP'
#!/bin/bash
echo "Setting up development environment..."
./tools/hooks/install-hook.sh
git config hooks.knots.mccabe-threshold 15
git config hooks.knots.cognitive-threshold 15
echo "Done!"
SETUP
chmod +x setup.sh

# Team members run:
./setup.sh
```

**Option 3: Documentation in README**

Add to your README.md:
```markdown
## Development Setup

Install the complexity check hook:
```bash
./tools/hooks/install-hook.sh
```
```

## Troubleshooting

### Hook Not Running

```bash
# Check if hook exists
ls -la .git/hooks/pre-commit

# Check if executable
chmod +x .git/hooks/pre-commit

# Test manually
.git/hooks/pre-commit
```

### knots Not Found

```bash
# Install knots
cargo install knots

# Or set custom path
git config hooks.knots.path /custom/path/to/knots

# Or add to PATH
export PATH=$PATH:/path/to/knots
```

### Hook Always Passes/Fails

```bash
# Check current thresholds
git config --get-regexp hooks.knots

# Reset to defaults
git config --unset hooks.knots.mccabe-threshold
git config --unset hooks.knots.cognitive-threshold
```

### Need to Bypass Once

```bash
# For one commit only
git commit --no-verify -m "Message"

# Note: This should be rare! Fix the complexity instead.
```

## Uninstallation

```bash
# Remove the hook
./install-hook.sh --uninstall

# Or manually
rm .git/hooks/pre-commit
```

## Advanced: Custom Hook Integration

If you already have a pre-commit hook, integrate knots:

```bash
#!/bin/bash
# Your existing pre-commit hook

# ... your existing checks ...

# Add knots check
if command -v knots &> /dev/null; then
    C_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(c|h)$')
    if [ -n "$C_FILES" ]; then
        for file in $C_FILES; do
            knots "$file" || exit 1
        done
    fi
fi

# ... more of your checks ...
```

## FAQ

**Q: Can I have different thresholds for different files?**
A: Not directly. Consider using warning-only mode and manual review for complex files.

**Q: Does this work with pre-commit framework (Python)?**
A: Not currently, but you can wrap it. See "Advanced" section.

**Q: What happens with syntax errors?**
A: The tool will fail gracefully and report the error. Commit is blocked.

**Q: Can I check complexity without committing?**
A: Yes! Run `knots file.c` manually anytime.

**Q: Does this check files not in git?**
A: No, only staged files. Use `knots *.c` to check all files.

## Best Practices

1. **Start lenient, get stricter over time**
   - Begin with thresholds at 20
   - Lower gradually as team adapts

2. **Use warning-only for legacy code**
   - Don't block commits on old code
   - Enforce limits on new code via code review

3. **Review complexity in PRs**
   - Use `knots -v` output in PR descriptions
   - Discuss functions with high complexity

4. **Fix root causes, not symptoms**
   - High complexity → refactor
   - Don't just split functions artificially

5. **Document exceptions**
   - If you use `--no-verify`, document WHY in commit message

## Support

Issues or questions? Check:
1. This documentation
2. Run `knots --help`
3. Check git config: `git config --get-regexp hooks`

---

**Remember:** The goal is better code, not just passing checks!

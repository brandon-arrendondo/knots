# Pre-Commit Hook - Quick Start

## 1-Minute Setup

```bash
# From your project root:
cd /path/to/your/git/project

# Copy hook files (assuming knots is in parent directory)
cp /path/to/knots/hooks/* .

# Install the hook
./install-hook.sh

# Done! Try making a commit
```

## What You Get

Every time you `git commit`, the hook will:
1. Find all C/H files you're committing
2. Check their complexity
3. Block the commit if any function exceeds thresholds

**Default thresholds:** 15 for both McCabe and Cognitive complexity

## Basic Usage

```bash
# Normal commit (will be checked)
git add file.c
git commit -m "Add feature"

# If complexity is too high:
# ✗ file.c:my_function
#   McCabe Complexity: 18 (threshold: 15)
# Commit rejected!

# Emergency bypass (use sparingly!)
git commit --no-verify -m "Emergency fix"
```

## Configuration

```bash
# Adjust thresholds
git config hooks.knots.mccabe-threshold 10
git config hooks.knots.cognitive-threshold 15

# See all settings
git config --get-regexp hooks.knots
```

## Two Modes Available

### Blocking (Default)
Rejects commits that exceed thresholds
```bash
./install-hook.sh
```

### Warning-Only
Shows warnings but allows all commits (good for legacy code)
```bash
./install-hook.sh --warning-only
```

## Need More Info?

Read the full documentation: [README.md](README.md)

## Quick Commands

```bash
# Check complexity manually (without committing)
knots file.c

# Verbose output (shows all functions)
knots -v file.c

# Uninstall hook
./install-hook.sh --uninstall

# Test hook manually
.git/hooks/pre-commit
```

That's it! Happy coding with cleaner, less complex code! 🚀

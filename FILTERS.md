# Filter Rules Documentation

Knots supports filtering files and functions using JSON-based filter rules. You can use `--include` to whitelist what to analyze, and `--exclude` to blacklist what to skip.

## Usage

```bash
# Include only specific files/functions
knots -r /path/to/code --include filter-include.json

# Exclude specific files/functions
knots -r /path/to/code --exclude filter-exclude.json

# Combine both (include takes precedence, then exclude is applied)
knots -r /path/to/code --include filter-include.json --exclude filter-exclude.json
```

## JSON Schema

Filter JSON files support the following fields (all are optional):

```json
{
  "file_patterns": ["pattern1", "pattern2", "!negation"],
  "function_patterns": ["regex1", "regex2"],
  "min_complexity": 5,
  "max_complexity": 50
}
```

### Fields

#### `file_patterns` (array of strings)
Glob-style patterns for matching file paths. Supports:
- `*` - matches any characters except `/`
- `**` - matches any characters including `/`
- `!pattern` - negation (exclude files matching this pattern)

**Examples:**
- `"src/**/*.c"` - all .c files in src/ directory and subdirectories
- `"lib/*.c"` - .c files directly in lib/ directory
- `"!**/test_*.c"` - exclude files starting with `test_`
- `"!**/vendor/**"` - exclude everything in vendor directories

**Behavior:**
- **Include filter**: File must match at least one positive pattern AND not match any negation patterns
- **Exclude filter**: File matching any pattern (positive or negative) will be excluded

#### `function_patterns` (array of strings)
Regular expression patterns for matching function names.

**Examples:**
- `"^process_.*"` - functions starting with `process_`
- `".*_handler$"` - functions ending with `_handler`
- `"^(init|setup|cleanup)_.*"` - functions starting with init_, setup_, or cleanup_

**Behavior:**
- **Include filter**: Function must match at least one pattern
- **Exclude filter**: Function matching any pattern will be excluded

#### `min_complexity` (number)
Minimum complexity threshold (inclusive). Functions with complexity below this will be filtered out.

- Complexity is calculated as `max(McCabe, Cognitive)`
- Default: no minimum (accepts all)

#### `max_complexity` (number)
Maximum complexity threshold (inclusive). Functions with complexity above this will be filtered out.

- Complexity is calculated as `max(McCabe, Cognitive)`
- Default: no maximum (accepts all)

## Examples

### Example 1: Analyze only high-complexity functions

**filter-high-complexity.json:**
```json
{
  "min_complexity": 20
}
```

```bash
knots -r . --include filter-high-complexity.json
```

### Example 2: Exclude test files and low-complexity functions

**filter-no-tests.json:**
```json
{
  "file_patterns": [
    "!**/test_*.c",
    "!**/*_test.c",
    "!**/tests/**"
  ],
  "function_patterns": [
    "^test_.*"
  ],
  "max_complexity": 100
}
```

```bash
knots -r . --exclude filter-no-tests.json
```

### Example 3: Focus on specific modules with concerning complexity

**filter-focus.json:**
```json
{
  "file_patterns": [
    "src/core/**/*.c",
    "src/drivers/**/*.c",
    "!**/vendor/**"
  ],
  "min_complexity": 10,
  "max_complexity": 50
}
```

```bash
knots -r . --include filter-focus.json
```

### Example 4: Exclude vendor code and simple functions

```bash
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
```

## Filter Logic

### Include Filter (Whitelist)
When `--include` is specified:
1. Files must match file_patterns (if specified)
2. Functions must match function_patterns (if specified)
3. Complexity must be within min/max bounds (if specified)

If any criterion fails, the file/function is skipped.

### Exclude Filter (Blacklist)
When `--exclude` is specified:
1. If a file matches file_patterns, it's excluded
2. If a function matches function_patterns AND complexity bounds, it's excluded

### Combined Filters
When both `--include` and `--exclude` are specified:
1. Include filter is applied first (whitelist)
2. Exclude filter is applied second (blacklist from whitelist)

This allows you to say "analyze only core/ directory" (include) and then "but skip test files" (exclude).

## Notes

- All filter criteria are optional - you can specify only the ones you need
- Empty arrays mean "match everything" for that criterion
- File patterns are matched against the full file path
- Function patterns use Rust's regex syntax
- Invalid regex patterns are silently ignored (function won't match)

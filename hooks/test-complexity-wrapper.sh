#!/bin/sh
#
# Pre-commit wrapper for knots-test-complexity
# Analyzes test files and their corresponding source files for quality
#
# Source file discovery strategies (--framework):
#   - convention (default): naming convention (test_foo.c / foo_test.c -> foo.c)
#   - ceedling: parses TEST_SOURCE_FILE macro
#
# Called by pre-commit framework with list of test files to check
#

# Default configuration
THRESHOLD=0.70
BOUNDARY_THRESHOLD=0.80
LEVEL="warn"
CHECK_BOUNDARIES=true
VERBOSE=false
TOOL_PATH=${TOOL_PATH:-knots-test-complexity}
FRAMEWORK="convention"
TEST_DIR="Test"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse arguments. sh has no arrays, so each filename is moved to the end of
# the positional parameters and "$@" holds only the files afterwards. A
# `--name VALUE` option is read as `--name=VALUE`.
n=$#
while [ "$n" -gt 0 ]; do
    arg=$1
    shift
    n=$((n - 1))
    case $arg in
        --threshold|--boundary-threshold|--level|--tool-path|--framework|--test-dir)
            if [ "$n" -eq 0 ]; then
                printf '%s\n' "Error: $arg needs a value" >&2
                exit 2
            fi
            arg="$arg=$1"
            shift
            n=$((n - 1))
            ;;
    esac
    case $arg in
        --threshold=*) THRESHOLD="${arg#*=}" ;;
        --boundary-threshold=*) BOUNDARY_THRESHOLD="${arg#*=}" ;;
        --level=*) LEVEL="${arg#*=}" ;;
        --tool-path=*) TOOL_PATH="${arg#*=}" ;;
        --framework=*) FRAMEWORK="${arg#*=}" ;;
        --test-dir=*) TEST_DIR="${arg#*=}" ;;
        --no-check-boundaries) CHECK_BOUNDARIES=false ;;
        --verbose) VERBOSE=true ;;
        *) set -- "$@" "$arg" ;;
    esac
done

# Validate framework
if [ "$FRAMEWORK" != "convention" ] && [ "$FRAMEWORK" != "ceedling" ]; then
    printf '%b\n' "${RED}Error: Unsupported framework '$FRAMEWORK'${NC}"
    printf '%s\n' "Supported frameworks: convention (default), ceedling"
    exit 1
fi

# Check if knots-test-complexity is available
if ! command -v "$TOOL_PATH" >/dev/null 2>&1; then
    printf '%b\n' "${YELLOW}Warning: knots-test-complexity not found at $TOOL_PATH${NC}"
    printf '%s\n' "Install it or set TOOL_PATH environment variable"
    exit 0  # Don't fail if tool not installed
fi

# If no files provided, exit successfully
if [ $# -eq 0 ]; then
    exit 0
fi

# Function to find source file via naming conventions:
#   test_foo.c -> foo.c
#   foo_test.c -> foo.c
# Both are called in $(...), a subshell, so their variables need no `local`.
find_source_file_convention() {
    test_file="$1"
    base=$(basename "$test_file")
    ext="${base##*.}"
    stem="${base%.*}"

    # Derive candidate source stem by stripping test_ prefix or _test suffix
    case $stem in
        test_*) source_stem="${stem#test_}" ;;
        *_test) source_stem="${stem%_test}" ;;
        *) return 1 ;;
    esac

    source_name="${source_stem}.${ext}"

    # Search project for matching source file, excluding test directories
    found=$(find . -name "$source_name" -type f 2>/dev/null \
        | grep -v "/$TEST_DIR/" \
        | grep -v "/test/" \
        | grep -v "/tests/" \
        | head -1)

    if [ -n "$found" ]; then
        printf '%s\n' "$found"
        return 0
    fi

    return 1
}

# Function to find source file for Ceedling framework
# Parses TEST_SOURCE_FILE("path/to/source.c") from test file
find_source_file_ceedling() {
    test_file="$1"

    # Look for TEST_SOURCE_FILE macro in the test file
    # Format: TEST_SOURCE_FILE("path/to/source.c") or TEST_SOURCE_FILE("source.c")
    source_path=$(grep -m 1 'TEST_SOURCE_FILE(' "$test_file" | sed -n 's/.*TEST_SOURCE_FILE("\([^"]*\)").*/\1/p')

    if [ -z "$source_path" ]; then
        return 1
    fi

    # If the path from TEST_SOURCE_FILE exists directly, use it
    if [ -f "$source_path" ]; then
        printf '%s\n' "$source_path"
        return 0
    fi

    # Otherwise, try to find the file in the project
    base_name=$(basename "$source_path")
    found_file=$(find . -name "$base_name" -type f 2>/dev/null | grep -v "$TEST_DIR" | head -1)

    if [ -n "$found_file" ]; then
        printf '%s\n' "$found_file"
        return 0
    fi

    return 1
}

FAILED=0
TOTAL_VIOLATIONS=0
PASSED_COUNT=0

for test_file in "$@"; do
    if [ ! -f "$test_file" ]; then
        continue
    fi

    # Find source file based on framework
    case "$FRAMEWORK" in
        convention)
            source_file=$(find_source_file_convention "$test_file")
            if [ -z "$source_file" ]; then
                printf '%b\n' "${YELLOW}⚠ $test_file: Cannot find source file via naming convention${NC}"
                printf '%s\n' "  Expected: test_foo.c or foo_test.c -> foo.c"
                printf '%s\n' "  Or use --framework=ceedling and add TEST_SOURCE_FILE(\"path/to/source.c\")"
                continue
            fi
            ;;
        ceedling)
            source_file=$(find_source_file_ceedling "$test_file")
            if [ -z "$source_file" ]; then
                printf '%b\n' "${YELLOW}⚠ $test_file: Cannot find TEST_SOURCE_FILE macro or source file${NC}"
                printf '%s\n' "  Add TEST_SOURCE_FILE(\"path/to/source.c\") to your test file"
                printf '%s\n' "  Or use --framework=convention for naming-convention based discovery"
                continue
            fi
            ;;
        *)
            printf '%b\n' "${RED}Error: Framework '$FRAMEWORK' not implemented${NC}"
            exit 1
            ;;
    esac

    # Build command
    CMD="$TOOL_PATH \"$test_file\" \"$source_file\" --threshold=$THRESHOLD --boundary-threshold=$BOUNDARY_THRESHOLD --level=$LEVEL"

    if [ "$CHECK_BOUNDARIES" = false ]; then
        CMD="$CMD --no-check-boundaries"
    fi

    if [ "$VERBOSE" = true ]; then
        CMD="$CMD --verbose"
    fi

    # Run analysis
    OUTPUT=$(eval $CMD 2>&1)
    EXIT_CODE=$?

    if [ $EXIT_CODE -ne 0 ]; then
        printf '%b\n' "${RED}✗ $test_file${NC}"
        printf '%s\n' "  Source: $source_file"
        printf '%s\n' "$OUTPUT"
        FAILED=1
        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
    else
        PASSED_COUNT=$((PASSED_COUNT + 1))
        if [ "$VERBOSE" = true ]; then
            printf '%b\n' "${GREEN}✓ $test_file${NC}"
            printf '%s\n' "  Source: $source_file"
            printf '%s\n' "$OUTPUT"
        fi
    fi
done

# Final summary
if [ $FAILED -eq 1 ]; then
    printf '%s\n' ""
    printf '%b\n' "${RED}Found $TOTAL_VIOLATIONS test quality violation(s)${NC}"
    if [ "$LEVEL" = "error" ]; then
        printf '%s\n' "Pre-commit check failed - fix the issues or use --no-verify to bypass"
        exit 1
    else
        printf '%b\n' "${YELLOW}Running in warning mode - pre-commit will pass${NC}"
        exit 0
    fi
else
    if [ "$VERBOSE" = true ] || [ $PASSED_COUNT -gt 0 ]; then
        printf '%s\n' ""
        printf '%b\n' "${GREEN}✓ All test quality checks passed ($PASSED_COUNT files)${NC}"
    fi
    exit 0
fi

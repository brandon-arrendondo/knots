#!/bin/bash
#
# Pre-commit wrapper for test-complexity
# Analyzes test files and their corresponding source files for quality
#
# Supports test frameworks:
#   - ceedling: Uses TEST_SOURCE_FILE macro to find source files
#
# Called by pre-commit framework with list of test files to check
#

# Default configuration
THRESHOLD=0.70
BOUNDARY_THRESHOLD=0.80
LEVEL="warn"
CHECK_BOUNDARIES=true
VERBOSE=false
TOOL_PATH=${TOOL_PATH:-test-complexity}
FRAMEWORK="ceedling"
TEST_DIR="Test"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse arguments
FILES=()
while [[ $# -gt 0 ]]; do
    case $1 in
        --threshold=*)
            THRESHOLD="${1#*=}"
            shift
            ;;
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --boundary-threshold=*)
            BOUNDARY_THRESHOLD="${1#*=}"
            shift
            ;;
        --boundary-threshold)
            BOUNDARY_THRESHOLD="$2"
            shift 2
            ;;
        --level=*)
            LEVEL="${1#*=}"
            shift
            ;;
        --level)
            LEVEL="$2"
            shift 2
            ;;
        --no-check-boundaries)
            CHECK_BOUNDARIES=false
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --tool-path=*)
            TOOL_PATH="${1#*=}"
            shift
            ;;
        --tool-path)
            TOOL_PATH="$2"
            shift 2
            ;;
        --framework=*)
            FRAMEWORK="${1#*=}"
            shift
            ;;
        --framework)
            FRAMEWORK="$2"
            shift 2
            ;;
        --test-dir=*)
            TEST_DIR="${1#*=}"
            shift
            ;;
        --test-dir)
            TEST_DIR="$2"
            shift 2
            ;;
        *)
            # This is a filename
            FILES+=("$1")
            shift
            ;;
    esac
done

# Validate framework
if [ "$FRAMEWORK" != "ceedling" ]; then
    echo -e "${RED}Error: Unsupported framework '$FRAMEWORK'${NC}"
    echo "Currently supported frameworks: ceedling"
    exit 1
fi

# Check if test-complexity is available
if ! command -v "$TOOL_PATH" &> /dev/null; then
    echo -e "${YELLOW}Warning: test-complexity not found at $TOOL_PATH${NC}"
    echo "Install it or set TOOL_PATH environment variable"
    exit 0  # Don't fail if tool not installed
fi

# If no files provided, exit successfully
if [ ${#FILES[@]} -eq 0 ]; then
    exit 0
fi

# Function to find source file for Ceedling framework
# Parses TEST_SOURCE_FILE("path/to/source.c") from test file
find_source_file_ceedling() {
    local test_file="$1"

    # Look for TEST_SOURCE_FILE macro in the test file
    # Format: TEST_SOURCE_FILE("path/to/source.c") or TEST_SOURCE_FILE("source.c")
    local source_path=$(grep -m 1 'TEST_SOURCE_FILE(' "$test_file" | sed -n 's/.*TEST_SOURCE_FILE("\([^"]*\)").*/\1/p')

    if [ -z "$source_path" ]; then
        return 1
    fi

    # If the path from TEST_SOURCE_FILE exists directly, use it
    if [ -f "$source_path" ]; then
        echo "$source_path"
        return 0
    fi

    # Otherwise, try to find the file in the project
    local base_name=$(basename "$source_path")
    local found_file=$(find . -name "$base_name" -type f 2>/dev/null | grep -v "$TEST_DIR" | head -1)

    if [ -n "$found_file" ]; then
        echo "$found_file"
        return 0
    fi

    return 1
}

FAILED=0
TOTAL_VIOLATIONS=0
PASSED_COUNT=0

for test_file in "${FILES[@]}"; do
    if [ ! -f "$test_file" ]; then
        continue
    fi

    # Find source file based on framework
    case "$FRAMEWORK" in
        ceedling)
            source_file=$(find_source_file_ceedling "$test_file")
            if [ -z "$source_file" ]; then
                echo -e "${YELLOW}⚠ $test_file: Cannot find TEST_SOURCE_FILE macro or source file${NC}"
                echo "  Add TEST_SOURCE_FILE(\"path/to/source.c\") to your test file"
                continue
            fi
            ;;
        *)
            echo -e "${RED}Error: Framework '$FRAMEWORK' not implemented${NC}"
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
        echo -e "${RED}✗ $test_file${NC}"
        echo "  Source: $source_file"
        echo "$OUTPUT"
        FAILED=1
        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
    else
        PASSED_COUNT=$((PASSED_COUNT + 1))
        if [ "$VERBOSE" = true ]; then
            echo -e "${GREEN}✓ $test_file${NC}"
            echo "  Source: $source_file"
            echo "$OUTPUT"
        fi
    fi
done

# Final summary
if [ $FAILED -eq 1 ]; then
    echo ""
    echo -e "${RED}Found $TOTAL_VIOLATIONS test quality violation(s)${NC}"
    if [ "$LEVEL" = "error" ]; then
        echo "Pre-commit check failed - fix the issues or use --no-verify to bypass"
        exit 1
    else
        echo -e "${YELLOW}Running in warning mode - pre-commit will pass${NC}"
        exit 0
    fi
else
    if [ "$VERBOSE" = true ] || [ $PASSED_COUNT -gt 0 ]; then
        echo ""
        echo -e "${GREEN}✓ All test quality checks passed ($PASSED_COUNT files)${NC}"
    fi
    exit 0
fi

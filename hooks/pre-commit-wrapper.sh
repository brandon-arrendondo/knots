#!/bin/sh
#
# Wrapper script for pre-commit framework
# Called by pre-commit with list of files to check
#

# Default configuration
DEFAULT_MCCABE_THRESHOLD=15
DEFAULT_COGNITIVE_THRESHOLD=15
DEFAULT_NESTING_THRESHOLD=5
DEFAULT_SLOC_THRESHOLD=50
DEFAULT_ABC_THRESHOLD=10.0
DEFAULT_RETURN_THRESHOLD=3
MCCABE_THRESHOLD=$DEFAULT_MCCABE_THRESHOLD
COGNITIVE_THRESHOLD=$DEFAULT_COGNITIVE_THRESHOLD
NESTING_THRESHOLD=$DEFAULT_NESTING_THRESHOLD
SLOC_THRESHOLD=$DEFAULT_SLOC_THRESHOLD
ABC_THRESHOLD=$DEFAULT_ABC_THRESHOLD
RETURN_THRESHOLD=$DEFAULT_RETURN_THRESHOLD
VERBOSE=false
VALIDATOR_PATH=${VALIDATOR_PATH:-knots}

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# True when $1 is a whole number (sh has no =~).
is_uint() {
    case $1 in
        ''|*[!0-9]*) return 1 ;;
    esac
}

# True when $1 is digits with at most one decimal point, not leading.
is_decimal() {
    case $1 in
        ''|.*|*.*.*|*[!0-9.]*) return 1 ;;
    esac
}

# Parse arguments. sh has no arrays, so each filename is moved to the end of
# the positional parameters and "$@" holds only the files afterwards. A
# `--name VALUE` option is read as `--name=VALUE`.
n=$#
while [ "$n" -gt 0 ]; do
    arg=$1
    shift
    n=$((n - 1))
    case $arg in
        --mccabe-threshold|--cognitive-threshold|--nesting-threshold|--sloc-threshold|--abc-threshold|--return-threshold|--validator-path)
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
        --mccabe-threshold=*) MCCABE_THRESHOLD="${arg#*=}" ;;
        --cognitive-threshold=*) COGNITIVE_THRESHOLD="${arg#*=}" ;;
        --nesting-threshold=*) NESTING_THRESHOLD="${arg#*=}" ;;
        --sloc-threshold=*) SLOC_THRESHOLD="${arg#*=}" ;;
        --abc-threshold=*) ABC_THRESHOLD="${arg#*=}" ;;
        --return-threshold=*) RETURN_THRESHOLD="${arg#*=}" ;;
        --validator-path=*) VALIDATOR_PATH="${arg#*=}" ;;
        --verbose) VERBOSE=true ;;
        *) set -- "$@" "$arg" ;;
    esac
done

# Determine if we should use -v flag with knots
# Use -v if: verbose mode OR any threshold differs from defaults (strict mode)
USE_VERBOSE_FLAG=false
if [ "$VERBOSE" = true ]; then
    USE_VERBOSE_FLAG=true
elif [ "$MCCABE_THRESHOLD" != "$DEFAULT_MCCABE_THRESHOLD" ] || \
     [ "$COGNITIVE_THRESHOLD" != "$DEFAULT_COGNITIVE_THRESHOLD" ] || \
     [ "$NESTING_THRESHOLD" != "$DEFAULT_NESTING_THRESHOLD" ] || \
     [ "$SLOC_THRESHOLD" != "$DEFAULT_SLOC_THRESHOLD" ] || \
     [ "$ABC_THRESHOLD" != "$DEFAULT_ABC_THRESHOLD" ] || \
     [ "$RETURN_THRESHOLD" != "$DEFAULT_RETURN_THRESHOLD" ]; then
    USE_VERBOSE_FLAG=true
fi

# Debug: Uncomment to verify thresholds
# printf '%s\n' "DEBUG: McCabe=$MCCABE_THRESHOLD, Cognitive=$COGNITIVE_THRESHOLD, Nesting=$NESTING_THRESHOLD" >&2

# Check if knots is available
if ! command -v "$VALIDATOR_PATH" >/dev/null 2>&1; then
    printf '%b\n' "${YELLOW}Warning: knots not found at $VALIDATOR_PATH${NC}"
    printf '%s\n' "Install it or set VALIDATOR_PATH environment variable"
    exit 0  # Don't fail if tool not installed
fi

# If no files provided, exit successfully
if [ $# -eq 0 ]; then
    exit 0
fi

FAILED=0
TOTAL_VIOLATIONS=0

for file in "$@"; do
    if [ ! -f "$file" ]; then
        continue
    fi
    
    # Run knots (use -v for verbose/strict modes)
    if [ "$USE_VERBOSE_FLAG" = true ]; then
        OUTPUT=$("$VALIDATOR_PATH" -v "$file" 2>&1)
    else
        OUTPUT=$("$VALIDATOR_PATH" "$file" 2>&1)
    fi
    EXIT_CODE=$?

    # Check if command succeeded
    if [ $EXIT_CODE -ne 0 ]; then
        # Skip UTF-8 errors silently (common in third-party/generated code)
        if printf '%s\n' "$OUTPUT" | grep -q "did not contain valid UTF-8"; then
            continue
        fi
        # Show other errors
        printf '%b\n' "${RED}Error running knots on $file${NC}"
        printf '%s\n' "$OUTPUT" | head -3
        FAILED=1
        continue
    fi
    
    # Parse output for violations
    CURRENT_FUNCTION=""
    FILE_HAS_VIOLATIONS=false

    if [ "$USE_VERBOSE_FLAG" = true ]; then
        # Parse verbose format (multi-line with separate metrics)
        while IFS= read -r line; do
            # Stop parsing at Summary section
            if printf '%s\n' "$line" | grep -q "^Summary:"; then
                break
            fi

            if printf '%s\n' "$line" | grep -q "Function:"; then
                # Extract function name and remove emoji (emojis are at the end)
                CURRENT_FUNCTION=$(printf '%s\n' "$line" | sed 's/Function: //' | sed 's/ [😊😐😠😢]$//')
            elif printf '%s\n' "$line" | grep -q "  McCabe Complexity:"; then
                MCCABE=$(printf '%s\n' "$line" | awk '{print $3}')
                if is_uint "$MCCABE" && [ "$MCCABE" -gt "$MCCABE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  McCabe Complexity: $MCCABE (threshold: $MCCABE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif printf '%s\n' "$line" | grep -q "  Cognitive Complexity:"; then
                COGNITIVE=$(printf '%s\n' "$line" | awk '{print $3}')
                if is_uint "$COGNITIVE" && [ "$COGNITIVE" -gt "$COGNITIVE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Cognitive Complexity: $COGNITIVE (threshold: $COGNITIVE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif printf '%s\n' "$line" | grep -q "  Nesting Depth:"; then
                NESTING=$(printf '%s\n' "$line" | awk '{print $3}')
                if is_uint "$NESTING" && [ "$NESTING" -gt "$NESTING_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Nesting Depth: $NESTING (threshold: $NESTING_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif printf '%s\n' "$line" | grep -q "  SLOC:"; then
                SLOC=$(printf '%s\n' "$line" | awk '{print $2}')
                if is_uint "$SLOC" && [ "$SLOC" -gt "$SLOC_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  SLOC: $SLOC (threshold: $SLOC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif printf '%s\n' "$line" | grep -q "  ABC:"; then
                # Extract ABC magnitude from line like "ABC: <2,4,5> (magnitude: 6.71)"
                ABC_MAG=$(printf '%s\n' "$line" | sed -n 's/.*magnitude: \([0-9.]*\).*/\1/p')
                if is_decimal "$ABC_MAG"; then
                    # Use bc for floating point comparison
                    if [ $(printf '%s\n' "$ABC_MAG > $ABC_THRESHOLD" | bc -l) -eq 1 ]; then
                        if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                            printf '%b\n' "${RED}✗ $file${NC}"
                            FILE_HAS_VIOLATIONS=true
                        fi
                        printf '%s\n' "  Function: $CURRENT_FUNCTION"
                        printf '%s\n' "  ABC Magnitude: $ABC_MAG (threshold: $ABC_THRESHOLD)"
                        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                        FAILED=1
                    fi
                fi
            elif printf '%s\n' "$line" | grep -q "  Return Count:"; then
                RETURN_COUNT=$(printf '%s\n' "$line" | awk '{print $3}')
                if is_uint "$RETURN_COUNT" && [ "$RETURN_COUNT" -gt "$RETURN_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Return Count: $RETURN_COUNT (threshold: $RETURN_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            fi
        done <<EOF
$OUTPUT
EOF
    else
        # Parse standard format (single line per function)
        while IFS= read -r line; do
            # Stop parsing at Summary section
            if printf '%s\n' "$line" | grep -q "^Summary:"; then
                break
            fi

            # Skip empty lines
            if [ -z "$line" ]; then
                continue
            fi

            # Parse line format: "😊 FunctionName (McCabe: 1, Cognitive: 0, ...)"
            if printf '%s\n' "$line" | grep -qE "^[😊😐😠😢]"; then
                # Extract function name (between emoji and opening parenthesis)
                CURRENT_FUNCTION=$(printf '%s\n' "$line" | sed -E 's/^[😊😐😠😢] ([^ ]+) .*/\1/')

                # Extract metrics
                MCCABE=$(printf '%s\n' "$line" | sed -n 's/.*McCabe: \([0-9]*\).*/\1/p')
                COGNITIVE=$(printf '%s\n' "$line" | sed -n 's/.*Cognitive: \([0-9]*\).*/\1/p')
                NESTING=$(printf '%s\n' "$line" | sed -n 's/.*Nesting: \([0-9]*\).*/\1/p')
                SLOC=$(printf '%s\n' "$line" | sed -n 's/.*SLOC: \([0-9]*\).*/\1/p')
                ABC_MAG=$(printf '%s\n' "$line" | sed -n 's/.*ABC: \([0-9.]*\).*/\1/p')
                RETURN_COUNT=$(printf '%s\n' "$line" | sed -n 's/.*Returns: \([0-9]*\).*/\1/p')

                # Check McCabe threshold
                if is_uint "$MCCABE" && [ "$MCCABE" -gt "$MCCABE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  McCabe Complexity: $MCCABE (threshold: $MCCABE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Cognitive threshold
                if is_uint "$COGNITIVE" && [ "$COGNITIVE" -gt "$COGNITIVE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Cognitive Complexity: $COGNITIVE (threshold: $COGNITIVE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Nesting threshold
                if is_uint "$NESTING" && [ "$NESTING" -gt "$NESTING_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Nesting Depth: $NESTING (threshold: $NESTING_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check SLOC threshold
                if is_uint "$SLOC" && [ "$SLOC" -gt "$SLOC_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  SLOC: $SLOC (threshold: $SLOC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check ABC threshold
                if is_decimal "$ABC_MAG" && [ $(printf '%s\n' "$ABC_MAG > $ABC_THRESHOLD" | bc -l) -eq 1 ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  ABC Magnitude: $ABC_MAG (threshold: $ABC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Return Count threshold
                if is_uint "$RETURN_COUNT" && [ "$RETURN_COUNT" -gt "$RETURN_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        printf '%b\n' "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    printf '%s\n' "  Function: $CURRENT_FUNCTION"
                    printf '%s\n' "  Return Count: $RETURN_COUNT (threshold: $RETURN_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            fi
        done <<EOF
$OUTPUT
EOF
    fi

    # Show verbose output if requested and no violations
    if [ "$VERBOSE" = true ] && [ "$FILE_HAS_VIOLATIONS" = false ]; then
        printf '%b\n' "${GREEN}✓ $file${NC}"
        printf '%s\n' "$OUTPUT" | grep -A 5 "Summary:" || true
    fi
done

# Final summary
if [ $FAILED -eq 1 ]; then
    printf '%s\n' ""
    printf '%b\n' "${RED}Found $TOTAL_VIOLATIONS complexity violation(s)${NC}"
    exit 1
else
    if [ "$VERBOSE" = true ]; then
        printf '%s\n' ""
        printf '%b\n' "${GREEN}✓ All complexity checks passed${NC}"
    fi
    exit 0
fi

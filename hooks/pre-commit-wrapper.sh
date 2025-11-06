#!/bin/bash
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

# Parse arguments
FILES=()
while [[ $# -gt 0 ]]; do
    case $1 in
        --mccabe-threshold=*)
            MCCABE_THRESHOLD="${1#*=}"
            shift
            ;;
        --mccabe-threshold)
            MCCABE_THRESHOLD="$2"
            shift 2
            ;;
        --cognitive-threshold=*)
            COGNITIVE_THRESHOLD="${1#*=}"
            shift
            ;;
        --cognitive-threshold)
            COGNITIVE_THRESHOLD="$2"
            shift 2
            ;;
        --nesting-threshold=*)
            NESTING_THRESHOLD="${1#*=}"
            shift
            ;;
        --nesting-threshold)
            NESTING_THRESHOLD="$2"
            shift 2
            ;;
        --sloc-threshold=*)
            SLOC_THRESHOLD="${1#*=}"
            shift
            ;;
        --sloc-threshold)
            SLOC_THRESHOLD="$2"
            shift 2
            ;;
        --abc-threshold=*)
            ABC_THRESHOLD="${1#*=}"
            shift
            ;;
        --abc-threshold)
            ABC_THRESHOLD="$2"
            shift 2
            ;;
        --return-threshold=*)
            RETURN_THRESHOLD="${1#*=}"
            shift
            ;;
        --return-threshold)
            RETURN_THRESHOLD="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --validator-path=*)
            VALIDATOR_PATH="${1#*=}"
            shift
            ;;
        --validator-path)
            VALIDATOR_PATH="$2"
            shift 2
            ;;
        *)
            # This is a filename
            FILES+=("$1")
            shift
            ;;
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
# echo "DEBUG: McCabe=$MCCABE_THRESHOLD, Cognitive=$COGNITIVE_THRESHOLD, Nesting=$NESTING_THRESHOLD" >&2

# Check if knots is available
if ! command -v "$VALIDATOR_PATH" &> /dev/null; then
    echo -e "${YELLOW}Warning: knots not found at $VALIDATOR_PATH${NC}"
    echo "Install it or set VALIDATOR_PATH environment variable"
    exit 0  # Don't fail if tool not installed
fi

# If no files provided, exit successfully
if [ ${#FILES[@]} -eq 0 ]; then
    exit 0
fi

FAILED=0
TOTAL_VIOLATIONS=0

for file in "${FILES[@]}"; do
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
        if echo "$OUTPUT" | grep -q "did not contain valid UTF-8"; then
            continue
        fi
        # Show other errors
        echo -e "${RED}Error running knots on $file${NC}"
        echo "$OUTPUT" | head -3
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
            if echo "$line" | grep -q "^Summary:"; then
                break
            fi

            if echo "$line" | grep -q "Function:"; then
                # Extract function name and remove emoji (emojis are at the end)
                CURRENT_FUNCTION=$(echo "$line" | sed 's/Function: //' | sed 's/ [😊😐😠😢]$//')
            elif echo "$line" | grep -q "  McCabe Complexity:"; then
                MCCABE=$(echo "$line" | awk '{print $3}')
                if [[ "$MCCABE" =~ ^[0-9]+$ ]] && [ "$MCCABE" -gt "$MCCABE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  McCabe Complexity: $MCCABE (threshold: $MCCABE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif echo "$line" | grep -q "  Cognitive Complexity:"; then
                COGNITIVE=$(echo "$line" | awk '{print $3}')
                if [[ "$COGNITIVE" =~ ^[0-9]+$ ]] && [ "$COGNITIVE" -gt "$COGNITIVE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Cognitive Complexity: $COGNITIVE (threshold: $COGNITIVE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif echo "$line" | grep -q "  Nesting Depth:"; then
                NESTING=$(echo "$line" | awk '{print $3}')
                if [[ "$NESTING" =~ ^[0-9]+$ ]] && [ "$NESTING" -gt "$NESTING_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Nesting Depth: $NESTING (threshold: $NESTING_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif echo "$line" | grep -q "  SLOC:"; then
                SLOC=$(echo "$line" | awk '{print $2}')
                if [[ "$SLOC" =~ ^[0-9]+$ ]] && [ "$SLOC" -gt "$SLOC_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  SLOC: $SLOC (threshold: $SLOC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            elif echo "$line" | grep -q "  ABC:"; then
                # Extract ABC magnitude from line like "ABC: <2,4,5> (magnitude: 6.71)"
                ABC_MAG=$(echo "$line" | sed -n 's/.*magnitude: \([0-9.]*\).*/\1/p')
                if [[ "$ABC_MAG" =~ ^[0-9]+\.?[0-9]*$ ]]; then
                    # Use bc for floating point comparison
                    if [ $(echo "$ABC_MAG > $ABC_THRESHOLD" | bc -l) -eq 1 ]; then
                        if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                            echo -e "${RED}✗ $file${NC}"
                            FILE_HAS_VIOLATIONS=true
                        fi
                        echo "  Function: $CURRENT_FUNCTION"
                        echo "  ABC Magnitude: $ABC_MAG (threshold: $ABC_THRESHOLD)"
                        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                        FAILED=1
                    fi
                fi
            elif echo "$line" | grep -q "  Return Count:"; then
                RETURN_COUNT=$(echo "$line" | awk '{print $3}')
                if [[ "$RETURN_COUNT" =~ ^[0-9]+$ ]] && [ "$RETURN_COUNT" -gt "$RETURN_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Return Count: $RETURN_COUNT (threshold: $RETURN_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            fi
        done <<< "$OUTPUT"
    else
        # Parse standard format (single line per function)
        while IFS= read -r line; do
            # Stop parsing at Summary section
            if echo "$line" | grep -q "^Summary:"; then
                break
            fi

            # Skip empty lines
            if [ -z "$line" ]; then
                continue
            fi

            # Parse line format: "😊 FunctionName (McCabe: 1, Cognitive: 0, ...)"
            if echo "$line" | grep -qE "^[😊😐😠😢]"; then
                # Extract function name (between emoji and opening parenthesis)
                CURRENT_FUNCTION=$(echo "$line" | sed -E 's/^[😊😐😠😢] ([^ ]+) .*/\1/')

                # Extract metrics
                MCCABE=$(echo "$line" | sed -n 's/.*McCabe: \([0-9]*\).*/\1/p')
                COGNITIVE=$(echo "$line" | sed -n 's/.*Cognitive: \([0-9]*\).*/\1/p')
                NESTING=$(echo "$line" | sed -n 's/.*Nesting: \([0-9]*\).*/\1/p')
                SLOC=$(echo "$line" | sed -n 's/.*SLOC: \([0-9]*\).*/\1/p')
                ABC_MAG=$(echo "$line" | sed -n 's/.*ABC: \([0-9.]*\).*/\1/p')
                RETURN_COUNT=$(echo "$line" | sed -n 's/.*Returns: \([0-9]*\).*/\1/p')

                # Check McCabe threshold
                if [[ "$MCCABE" =~ ^[0-9]+$ ]] && [ "$MCCABE" -gt "$MCCABE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  McCabe Complexity: $MCCABE (threshold: $MCCABE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Cognitive threshold
                if [[ "$COGNITIVE" =~ ^[0-9]+$ ]] && [ "$COGNITIVE" -gt "$COGNITIVE_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Cognitive Complexity: $COGNITIVE (threshold: $COGNITIVE_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Nesting threshold
                if [[ "$NESTING" =~ ^[0-9]+$ ]] && [ "$NESTING" -gt "$NESTING_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Nesting Depth: $NESTING (threshold: $NESTING_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check SLOC threshold
                if [[ "$SLOC" =~ ^[0-9]+$ ]] && [ "$SLOC" -gt "$SLOC_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  SLOC: $SLOC (threshold: $SLOC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check ABC threshold
                if [[ "$ABC_MAG" =~ ^[0-9]+\.?[0-9]*$ ]] && [ $(echo "$ABC_MAG > $ABC_THRESHOLD" | bc -l) -eq 1 ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  ABC Magnitude: $ABC_MAG (threshold: $ABC_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi

                # Check Return Count threshold
                if [[ "$RETURN_COUNT" =~ ^[0-9]+$ ]] && [ "$RETURN_COUNT" -gt "$RETURN_THRESHOLD" ]; then
                    if [ "$FILE_HAS_VIOLATIONS" = false ]; then
                        echo -e "${RED}✗ $file${NC}"
                        FILE_HAS_VIOLATIONS=true
                    fi
                    echo "  Function: $CURRENT_FUNCTION"
                    echo "  Return Count: $RETURN_COUNT (threshold: $RETURN_THRESHOLD)"
                    TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
                    FAILED=1
                fi
            fi
        done <<< "$OUTPUT"
    fi

    # Show verbose output if requested and no violations
    if [ "$VERBOSE" = true ] && [ "$FILE_HAS_VIOLATIONS" = false ]; then
        echo -e "${GREEN}✓ $file${NC}"
        echo "$OUTPUT" | grep -A 5 "Summary:" || true
    fi
done

# Final summary
if [ $FAILED -eq 1 ]; then
    echo ""
    echo -e "${RED}Found $TOTAL_VIOLATIONS complexity violation(s)${NC}"
    exit 1
else
    if [ "$VERBOSE" = true ]; then
        echo ""
        echo -e "${GREEN}✓ All complexity checks passed${NC}"
    fi
    exit 0
fi

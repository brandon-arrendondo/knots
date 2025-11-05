#!/bin/bash
#
# Wrapper script for pre-commit framework
# Called by pre-commit with list of files to check
#

# Default configuration
MCCABE_THRESHOLD=15
COGNITIVE_THRESHOLD=15
NESTING_THRESHOLD=5
SLOC_THRESHOLD=50
ABC_THRESHOLD=10.0
RETURN_THRESHOLD=3
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
        --mccabe-threshold)
            MCCABE_THRESHOLD="$2"
            shift 2
            ;;
        --cognitive-threshold)
            COGNITIVE_THRESHOLD="$2"
            shift 2
            ;;
        --nesting-threshold)
            NESTING_THRESHOLD="$2"
            shift 2
            ;;
        --sloc-threshold)
            SLOC_THRESHOLD="$2"
            shift 2
            ;;
        --abc-threshold)
            ABC_THRESHOLD="$2"
            shift 2
            ;;
        --return-threshold)
            RETURN_THRESHOLD="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
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
    
    # Run knots (always use verbose mode for parsing)
    OUTPUT=$("$VALIDATOR_PATH" -v "$file" 2>&1)
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

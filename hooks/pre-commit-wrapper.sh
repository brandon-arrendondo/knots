#!/bin/sh
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

# knots --format ndjson prints one JSON record per function, ordered by name.
# This awk program prints "LINE<TAB>SEQ<TAB>LABEL<TAB>MESSAGE" for each metric
# over its threshold, in the order the text report lists the metrics, and
# breaches() sorts by LINE to put functions in source order, as the text report
# does. LABEL is file:line:function, as the text report names a function, and
# ABC is compared as the report shows it, to two decimal places. Reading the records, not the text report, keeps a change of
# report layout from silently switching a threshold off.
BREACHES_AWK='
function str(key,    p, s, c, out) {
    p = index($0, "\"" key "\":\"")
    if (p == 0) return ""
    s = substr($0, p + length(key) + 4)
    out = ""
    while (s != "") {
        c = substr(s, 1, 1)
        if (c == "\"") break
        if (c == "\\") {
            c = substr(s, 2, 1)
            s = substr(s, 3)
            if (c == "n") c = "\n"
            else if (c == "t") c = "\t"
            else if (c == "r") c = "\r"
            else if (c == "u") { c = "?"; s = substr(s, 5) }
        } else {
            s = substr(s, 2)
        }
        out = out c
    }
    return out
}
function num(key,    p, s) {
    p = index($0, "\"" key "\":")
    if (p == 0) return ""
    s = substr($0, p + length(key) + 3)
    if (!match(s, /^-?[0-9][0-9.eE+-]*/)) return ""
    return substr(s, 1, RLENGTH)
}
function check(label, name, value, limit) {
    if (value != "" && limit != "" && value + 0 > limit + 0)
        printf "%s\t%d\t%s\t%s: %s (threshold: %s)\n", line + 0, ++seq, label, name, value, limit
}
/^\{/ {
    line = num("start_line")
    label = str("file") ":" line ":" str("function")
    check(label, "McCabe Complexity", num("mccabe"), mccabe)
    check(label, "Cognitive Complexity", num("cognitive"), cognitive)
    check(label, "Nesting Depth", num("nesting"), nesting)
    check(label, "SLOC", num("sloc"), sloc)
    abc = num("abc_magnitude")
    if (abc != "") abc = sprintf("%.2f", abc)
    check(label, "ABC Magnitude", abc, abc_limit)
    check(label, "Return Count", num("return_count"), returns)
}
'

# Print "LABEL<TAB>MESSAGE" for each breach in $1, knots --format ndjson
# output, in source order.
breaches() {
    printf '%s\n' "$1" | awk -v mccabe="$MCCABE_THRESHOLD" \
        -v cognitive="$COGNITIVE_THRESHOLD" -v nesting="$NESTING_THRESHOLD" \
        -v sloc="$SLOC_THRESHOLD" -v abc_limit="$ABC_THRESHOLD" \
        -v returns="$RETURN_THRESHOLD" "$BREACHES_AWK" |
        sort -t "$TAB" -k1,1n -k2,2n | cut -f3-
}

TAB=$(printf '\t')

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

    OUTPUT=$("$VALIDATOR_PATH" --format ndjson "$file" 2>&1)
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

    FILE_HAS_VIOLATIONS=false
    while IFS="$TAB" read -r label message; do
        [ -n "$label" ] || continue
        if [ "$FILE_HAS_VIOLATIONS" = false ]; then
            printf '%b%s%b\n' "$RED" "✗ $file" "$NC"
            FILE_HAS_VIOLATIONS=true
        fi
        printf '%s\n' "  Function: $label"
        printf '%s\n' "  $message"
        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + 1))
        FAILED=1
    done <<EOF
$(breaches "$OUTPUT")
EOF

    # Show verbose output if requested and no violations
    if [ "$VERBOSE" = true ] && [ "$FILE_HAS_VIOLATIONS" = false ]; then
        printf '%b%s%b\n' "$GREEN" "✓ $file" "$NC"
        "$VALIDATOR_PATH" -v "$file" 2>&1 | grep -A 5 "Summary:" || true
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

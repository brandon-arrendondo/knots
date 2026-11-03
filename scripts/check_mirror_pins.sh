#!/bin/sh
# Check that every knots-pre-commit `rev:` pin in the tracked docs and
# examples names a tag that exists in the mirror on GitHub. The mirror is
# tagged after knots is published (docs/releasing.rst, step 3), so run this
# once that tag is pushed. Needs network access.
#
# Exit status: 0 every pin resolves; 1 a pin names a tag the mirror does
# not have (printed as MISSING); 2 the mirror could not be reached, so
# nothing was decided.
set -u
# KNOTS_PRE_COMMIT_URL points the check at another copy of the mirror.
MIRROR=${KNOTS_PRE_COMMIT_URL:-https://github.com/brandon-arrendondo/knots-pre-commit}
cd "$(git rev-parse --show-toplevel)" || exit 2

# "FILE:LINE<TAB>REV" for each rev: under a knots-pre-commit repo: entry.
# A later repo: entry, of any repository, ends the search, so an entry with
# no rev: never takes the next repository's.
pins=$(git -c core.quotePath=false ls-files -- '*.md' '*.rst' '*.yaml' '*.yml' |
    while IFS= read -r f; do
        awk -v f="$f" '
            /repo:/ { want = ($0 ~ /github\.com\/brandon-arrendondo\/knots-pre-commit/); next }
            want && /rev:/ {
                rev = $0
                sub(/.*rev:[ \t]*/, "", rev)
                sub(/[ \t]*#.*/, "", rev)
                gsub(/["\047 \t]/, "", rev)
                print f ":" FNR "\t" rev
                want = 0
            }
        ' "$f"
    done)

if [ -z "$pins" ]; then
    echo "No knots-pre-commit pins found" >&2
    exit 1
fi

TAB=$(printf '\t')
status=0
for rev in $(printf '%s\n' "$pins" | cut -f2 | sort -u); do
    git ls-remote --exit-code --tags "$MIRROR" "refs/tags/$rev" \
        </dev/null >/dev/null 2>&1
    rc=$?
    case $rc in
        0) result=ok ;;
        2) result=MISSING; status=1 ;;
        *)
            echo "cannot reach mirror $MIRROR (git ls-remote exited $rc)" >&2
            exit 2
            ;;
    esac
    printf '%s\n' "$pins" | while IFS="$TAB" read -r where pin; do
        [ "$pin" = "$rev" ] && printf '%-8s %s %s\n' "$result" "$where" "$pin"
    done
done
exit "$status"

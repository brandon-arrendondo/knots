#!/bin/sh
# Check that every knots-pre-commit `rev:` pin in the tracked docs and
# examples names a tag that exists in the mirror on GitHub. The mirror is
# tagged after knots is published (docs/releasing.rst, step 3), so run this
# once that tag is pushed. Needs network access; exits 1 if any pin is
# missing.
set -u
MIRROR=https://github.com/brandon-arrendondo/knots-pre-commit
cd "$(git rev-parse --show-toplevel)" || exit 2

pins=$(for f in $(git ls-files '*.md' '*.rst' '*.yaml' '*.yml'); do
    awk -v f="$f" '
        /repo: https:\/\/github\.com\/brandon-arrendondo\/knots-pre-commit/ { want = 1; next }
        want && /rev:/ { print f ":" FNR " " $2; want = 0 }
    ' "$f"
done)

if [ -z "$pins" ]; then
    echo "No knots-pre-commit pins found" >&2
    exit 1
fi

status=0
for rev in $(printf '%s\n' "$pins" | awk '{ print $2 }' | sort -u); do
    if git ls-remote --exit-code --tags "$MIRROR" "refs/tags/$rev" \
        </dev/null >/dev/null 2>&1; then
        result=ok
    else
        result=MISSING
        status=1
    fi
    printf '%s\n' "$pins" | awk -v rev="$rev" -v r="$result" \
        '$2 == rev { printf "%-8s %s %s\n", r, $1, rev }'
done
exit "$status"

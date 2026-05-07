#!/usr/bin/env bash
#
# check_no_new_inherent_conversions.sh
#
# Forbids new inherent `to_json` / `from_json` / `to_object` / `from_object`
# / `into_object` methods on rs-dpp types. They should use the canonical
# `JsonConvertible` / `ValueConvertible` traits instead.
#
# See: docs/json-value-conversion-canonical-pattern.md
#
# Run from the repo root:
#   ./scripts/lint/check_no_new_inherent_conversions.sh
#
# Update the allowlist (only when intentionally removing an entry):
#   ./scripts/lint/check_no_new_inherent_conversions.sh --update
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ALLOWLIST="${REPO_ROOT}/scripts/lint/inherent_conversions.allowlist"

# Match `pub fn to_json` etc. at module scope (NOT inside `impl Trait for`
# blocks — those satisfy the canonical traits and are allowed).
PATTERN='^[[:space:]]*pub fn (to_json|from_json|to_object|from_object|into_object)\b'

# Strip line numbers from grep output so allowlist entries don't churn on
# unrelated edits above.
strip_line_numbers() {
    sed 's/:[0-9][0-9]*:/:/'
}

scan() {
    # cd into the repo root so grep emits paths relative to it — keeps the
    # allowlist portable across machines / CI runners.
    (cd "$REPO_ROOT" && grep -rEn "$PATTERN" \
        --include='*.rs' \
        packages/rs-dpp/src) \
        | strip_line_numbers \
        | sort -u
}

if [[ "${1:-}" == "--update" ]]; then
    scan > "$ALLOWLIST"
    echo "Updated $ALLOWLIST"
    wc -l "$ALLOWLIST"
    exit 0
fi

if [[ ! -f "$ALLOWLIST" ]]; then
    echo "ERROR: missing allowlist at $ALLOWLIST" >&2
    echo "Run with --update to bootstrap." >&2
    exit 2
fi

actual="$(scan)"
expected="$(cat "$ALLOWLIST")"

if [[ "$actual" != "$expected" ]]; then
    echo "Inherent conversion methods on rs-dpp types changed."
    echo "Diff (expected vs actual):"
    diff <(echo "$expected") <(echo "$actual") || true
    echo
    echo "If you ADDED a method: this is forbidden — use the canonical"
    echo "JsonConvertible / ValueConvertible traits instead. See"
    echo "docs/json-value-conversion-canonical-pattern.md."
    echo
    echo "If you DELETED a method: regenerate the allowlist with"
    echo "  ./scripts/lint/check_no_new_inherent_conversions.sh --update"
    echo "and commit the change."
    exit 1
fi

echo "OK — no new inherent conversion methods on rs-dpp types."

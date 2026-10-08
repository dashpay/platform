#!/bin/bash

# Fails when a macOS installer holds a native binary that is not signed with a
# Developer ID certificate. Apple does not notarize such a package.
#
# Usage: check_dashmate_macos_pkg.sh PKG...

set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "Usage: check_dashmate_macos_pkg.sh PKG..." >&2
  exit 1
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

# `file` prints extra lines for a universal binary. They do not carry the
# separator, so a line without it is not the start of a new file.
separator='|dashmate-pkg-check|'

failed=false
for pkg in "$@"; do
  expanded="$work_dir/expanded"
  unsigned="$work_dir/unsigned.txt"
  rm -rf "$expanded"
  : > "$unsigned"
  pkgutil --expand-full "$pkg" "$expanded"

  while IFS= read -r line; do
    case "$line" in
      *"$separator"*Mach-O*) ;;
      *) continue ;;
    esac
    path="${line%%"$separator"*}"
    signature="$(codesign -dvv "$path" 2>&1 || true)"
    case "$signature" in
      *"Authority=Developer ID Application:"*) ;;
      *) echo "${path#"$expanded"/}" >> "$unsigned" ;;
    esac
  done < <(find "$expanded" -type f -exec file -F "$separator" {} +)

  if [ -s "$unsigned" ]; then
    failed=true
    count="$(wc -l < "$unsigned" | tr -d ' ')"
    echo "::error::$(basename "$pkg") holds $count native binaries without a Developer ID signature, so Apple will not notarize it."
    cat "$unsigned"
  else
    echo "$(basename "$pkg"): every native binary has a Developer ID signature."
  fi
done

if "$failed"; then
  exit 1
fi

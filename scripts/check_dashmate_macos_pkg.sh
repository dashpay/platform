#!/bin/bash

# Fails when a macOS installer holds a native binary that is not signed with a
# Developer ID certificate. Apple does not notarize such a package.
#
# This catches the certain rejections early. It does not look inside nested
# archives or at the hardened runtime and the timestamp, so the notary service
# still has the last word.
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
  binaries=0
  pkgutil --expand-full "$pkg" "$expanded"

  while IFS= read -r line; do
    case "$line" in
      *"$separator"*Mach-O*) ;;
      *) continue ;;
    esac
    path="${line%%"$separator"*}"
    binaries=$((binaries + 1))
    # stdin is the list of files, so keep it away from codesign.
    signature="$(codesign -dvv "$path" 2>&1 < /dev/null || true)"
    if ! grep -q '^Authority=Developer ID Application:' <<< "$signature" \
      || ! codesign --verify --strict "$path" > /dev/null 2>&1 < /dev/null; then
      echo "${path#"$expanded"/}" >> "$unsigned"
    fi
  done < <(find "$expanded" -type f -exec file -F "$separator" {} +)

  # Every package holds at least the node binary.
  if [ "$binaries" -eq 0 ]; then
    failed=true
    echo "::error::Found no native binary in $(basename "$pkg"), not even node: the scan did not work."
  elif [ -s "$unsigned" ]; then
    failed=true
    count="$(wc -l < "$unsigned" | tr -d ' ')"
    echo "::error::$(basename "$pkg") holds $count native binaries without a valid Developer ID signature, so Apple will not notarize it."
    cat "$unsigned"
  else
    echo "$(basename "$pkg"): native binaries checked: $binaries, each with a valid Developer ID signature."
  fi
done

if "$failed"; then
  exit 1
fi

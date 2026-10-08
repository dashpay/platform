#!/bin/bash

# Fails when a macOS installer holds a native binary that is not signed with a
# Developer ID certificate. Apple does not notarize such a package.
#
# This catches the certain rejections early. It skips object files, which the
# notary service does not judge, does not look inside nested archives (zip
# files, static libraries), and leaves the hardened runtime and the timestamp
# alone, because what Apple asks there depends on the kind of binary. The
# notary service still has the last word.
#
# Usage: check_dashmate_macos_pkg.sh PKG_OR_DIRECTORY...

set -euo pipefail

packages=()
for argument in "$@"; do
  if [ -d "$argument" ]; then
    while IFS= read -r -d '' pkg; do
      packages+=("$pkg")
    done < <(find "$argument" -type f -name '*.pkg' -print0)
  elif [ -f "$argument" ]; then
    packages+=("$argument")
  else
    echo "::error::$argument is neither a package nor a directory."
    exit 1
  fi
done

if [ "${#packages[@]}" -eq 0 ]; then
  echo '::error::No .pkg to check. Usage: check_dashmate_macos_pkg.sh PKG_OR_DIRECTORY...'
  exit 1
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

# Apple's own definition of "signed with a Developer ID Application
# certificate". It covers every architecture of a universal binary.
developer_id='anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists'

# `file` prints extra lines for a universal binary. They do not carry the
# separator, so a line without it is not the start of a new file.
separator='|dashmate-pkg-check|'

failed=false
for pkg in "${packages[@]}"; do
  expanded="$work_dir/expanded"
  listing="$work_dir/listing.txt"
  report="$work_dir/report.txt"
  rm -rf "$expanded"
  : > "$report"
  binaries=0
  unsigned=0
  pkgutil --expand-full "$pkg" "$expanded"
  # `file` reports a file it cannot read and still exits 0.
  chmod -R u+r "$expanded"
  # Written to a file first, so that a failing scan stops the script.
  find "$expanded" -type f -exec file -F "$separator" {} + > "$listing"

  while IFS= read -r line; do
    case "$line" in
      *"$separator"*Mach-O*' object '*) continue ;;
      *"$separator"*Mach-O*) ;;
      *) continue ;;
    esac
    path="${line%%"$separator"*}"
    binaries=$((binaries + 1))
    # stdin is the listing, so keep it away from codesign.
    if ! reason="$(codesign --verify --strict -R="$developer_id" "$path" 2>&1 < /dev/null)"; then
      unsigned=$((unsigned + 1))
      {
        echo "${path#"$expanded"/}"
        while IFS= read -r reason_line; do
          echo "    ${reason_line#"$path": }"
        done <<< "$reason"
      } >> "$report"
    fi
  done < "$listing"

  # Every package holds at least the node binary.
  if [ "$binaries" -eq 0 ]; then
    failed=true
    echo "::error::Found no native binary in $(basename "$pkg"), not even node: the scan did not work."
  elif [ "$unsigned" -gt 0 ]; then
    failed=true
    echo "::error::$(basename "$pkg") holds $unsigned native binaries without a valid Developer ID signature, so Apple will not notarize it."
    cat "$report"
  else
    echo "$(basename "$pkg"): native binaries checked: $binaries, each with a valid Developer ID signature."
  fi
done

if "$failed"; then
  exit 1
fi

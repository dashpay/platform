#!/bin/bash
# shellcheck disable=SC2250
set -e

files=$(find "$PWD/clients/core/v0/web" "$PWD/clients/core/v0/nodejs" "$PWD/clients/platform/v0/web" "$PWD/clients/platform/v0/nodejs" -name "*_pb.js" -o -name "*_protoc.js")
OS=$(uname)

function replace_in_file() {
  if [[ "$OS" = 'Darwin' ]]; then
    # for MacOS
    sed -i '' -e "$1" "$2"
  else
    # for Linux and Windows
    sed -i'' -e "$1" "$2"
  fi
}

# Loop over the files
for file in $files; do

  replace_in_file 's/var global = Function('\''return this'\'')();/const proto = {};/g' "$file"
  if grep -qrE "[^a-zA-Z]Function\(" "$file"; then
    echo "Error: Function( still present"
  fi

  replace_in_file 's/, global);/, { proto });/g' "$file"
  if grep -qrE '(^|[^a-zA-Z."])global([^a-zA-Z]|$)' "$file"; then
    echo "Error: global still present"
  fi

done

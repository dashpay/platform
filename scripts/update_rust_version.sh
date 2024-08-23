#! /bin/bash -e

# This script updates the rust version to the one provided in the first argument.
# Requires `yq` to be installed: https://github.com/mikefarah/yq
PROJECT_ROOT="$(realpath "$(dirname "$0")"/..)"

# Check if the first argument is provided
if [ -z "${1}" ]; then
    echo "Please provide the new rust version as the first argument."
    exit 1
fi

VERSION="$1"

function check {
    for file in "$@"; do
        if ! grep -q "${VERSION}" "${file}"; then
            echo "The file ${file} does not contain the version ${VERSION}."
            exit 1
        fi
    done
}

echo Update the rust version in the Cargo.toml file
sed -i "s/^rust-version = \".*\"/rust-version = \"${VERSION}\"/" "${PROJECT_ROOT}"/Cargo.toml
check "${PROJECT_ROOT}"/Cargo.toml

echo Update the rust version in the README.md
sed -i "s/\(\[rust\](https.*)\) v[0-9.]\++,/\1 v$VERSION+,/" "${PROJECT_ROOT}/README.md"
check "${PROJECT_ROOT}/README.md"

echo Update the rust version in rust-toolchain.toml
sed -i "s/^channel = \".*\"/channel = \"${VERSION}\"/" "${PROJECT_ROOT}/rust-toolchain.toml"
check "${PROJECT_ROOT}/rust-toolchain.toml"

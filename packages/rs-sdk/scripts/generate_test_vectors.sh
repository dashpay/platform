#! /bin/bash -ex

CARGO_DIR="$(realpath "$(dirname "$0")/..")"

pushd "$CARGO_DIR"

rm -f "${CARGO_DIR}"/tests/vectors/*

cargo test -p rs-sdk \
    --no-default-features \
    --features generate-test-vectors

popd

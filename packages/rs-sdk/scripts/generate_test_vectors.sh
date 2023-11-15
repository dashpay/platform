#! /bin/bash -ex

# This script generates test vectors for offline testing of the SDK.
# Test vectors will be generated in the `tests/vectors` directory.
#
# Existing test vectors are removed before generating new ones.
#
# Generation of test vectors is done by running the SDK tests with the
# `generate-test-vectors` feature enabled.

CARGO_DIR="$(realpath "$(dirname "$0")/..")"

pushd "$CARGO_DIR"

rm -f "${CARGO_DIR}"/tests/vectors/*

cargo test -p dash-platform-sdk \
    --no-default-features \
    --features generate-test-vectors

popd

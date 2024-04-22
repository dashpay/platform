#! /bin/bash -ex

# This script generates test vectors for offline testing of the SDK.
# Test vectors will be generated in the `tests/vectors` directory.
#
# Generation of test vectors is done by running the SDK tests with the
# `generate-test-vectors` feature enabled.
#
#
# Usage:
#   ./generate_test_vectors.sh
#   ./generate_test_vectors.sh test::name
#
# When test::name is specified, only the specified test is run and
# its test vector is generated.
#
# Otherwise, all existing test vectors are removed and regenerated.

CARGO_DIR="$(realpath "$(dirname "$0")/..")"

pushd "$CARGO_DIR"

cargo test -p dash-sdk \
    --no-default-features \
    --features generate-test-vectors \
    "$1"

popd

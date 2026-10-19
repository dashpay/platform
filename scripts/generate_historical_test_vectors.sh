#!/usr/bin/env bash
# Reproduce compatibility vectors using the historical Platform dependency graph.
set -euo pipefail

usage() {
  echo "Usage: $0 <new-output-directory>"
  echo "Creates a pinned checkout and writes generated vectors below <directory>/vectors."
  echo "Set HISTORICAL_VECTOR_CARGO to a Cargo wrapper executable if needed."
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi
if [[ $# -ne 1 ]]; then
  usage >&2
  exit 2
fi

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
revision=6499c680c6bc311e8933f396423a79459e2a88bd
cargo_runner=${HISTORICAL_VECTOR_CARGO:-cargo}
git -C "$repo" cat-file -e "$revision^{commit}"

# Require a new directory, so neither existing results nor a user checkout is overwritten.
mkdir -- "$1"
output=$(cd -- "$1" && pwd)
checkout="$output/source"
git clone --quiet --shared --no-checkout "$repo" "$checkout"
git -C "$checkout" checkout --quiet --detach "$revision"
printf '%s\n' "$revision" > "$output/platform-revision.txt"

install_generators() {
  local package=$1
  shift
  local tests="$checkout/packages/$package/tests"
  local harness="$tests/generate_historical_vectors.rs"
  mkdir -p -- "$tests/historical_vectors"
  cat > "$harness" <<'RUST'
fn output_dir(group: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(
        std::env::var_os("HISTORICAL_VECTOR_OUTPUT_DIR").expect("generator output directory"),
    ).join(group);
    std::fs::create_dir_all(&path).expect("create generator output directory");
    path
}
RUST
  local name
  for name in "$@"; do
    cp -- "$repo/scripts/historical-test-vectors/$name.rs" "$tests/historical_vectors/$name.rs"
    printf '#[path = "historical_vectors/%s.rs"]\nmod %s;\n' "$name" "${name//-/_}" >> "$harness"
  done
}

install_generators rs-platform-encryption encryption
install_generators rs-dpp bls core-transactions signing
install_generators rs-drive-abci quorum-v0 quorum populated

export HISTORICAL_VECTOR_OUTPUT_DIR="$output/vectors"
cd -- "$checkout"
# Signing fixtures embed ciphertexts produced by the encryption generator.
"$cargo_runner" test -p platform-encryption --test generate_historical_vectors --locked -- --ignored
"$cargo_runner" test -p dpp -p drive-abci --test generate_historical_vectors --all-features --locked -- --ignored
printf 'Generated vectors: %s\nHistorical checkout retained: %s\n' "$HISTORICAL_VECTOR_OUTPUT_DIR" "$checkout"

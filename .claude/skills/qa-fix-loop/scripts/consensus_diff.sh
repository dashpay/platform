#!/usr/bin/env bash
# Say whether a build of <test-ref> can be tested against a network that runs
# <deployed-ref>: list what changed between the two in the code that decides
# consensus, proofs and the node API, and in the pinned GroveDB, Tenderdash and
# Dash Core crates.
#
#   consensus_diff.sh <deployed-ref> <test-ref>
#
# Exit 0: nothing changed there. Exit 1: something did; read it before testing.
# Exit 2: usage or git error. Read-only. Run it inside a platform checkout that
# has both refs (git fetch --tags origin first).

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <deployed-ref> <test-ref>" >&2
  exit 2
fi

deployed="$1"
tested="$2"

for ref in "${deployed}" "${tested}"; do
  if ! git rev-parse --verify --quiet "${ref}^{commit}" >/dev/null; then
    echo "unknown ref: ${ref} (fetch it first)" >&2
    exit 2
  fi
done

paths=(
  packages/rs-drive
  packages/rs-drive-abci
  packages/rs-dpp
  packages/rs-platform-version
  packages/rs-platform-versioning
  packages/rs-platform-value
  packages/rs-platform-serialization
  packages/rs-platform-serialization-derive
  packages/rs-drive-proof-verifier
  packages/rs-dapi
  packages/dapi
  packages/dapi-grpc
  packages/data-contracts
  packages/dpns-contract
  packages/dashpay-contract
  packages/withdrawals-contract
  packages/masternode-reward-shares-contract
  packages/token-history-contract
  packages/keyword-search-contract
  packages/wallet-utils-contract
  packages/decision-proposals-contract
  packages/moderation-charters-contract
  packages/document-history-contract
  packages/token-market-contract
  packages/app-connect-contract
)

changed=0

echo "Between ${deployed} ($(git rev-parse --short=10 "${deployed}^{commit}")) and ${tested} ($(git rev-parse --short=10 "${tested}^{commit}")):"

for path in "${paths[@]}"; do
  # Version bumps in a manifest are not code: a release commit touches every one.
  files="$(git diff --name-only "${deployed}" "${tested}" -- "${path}" \
    | grep -v -E '(^|/)(Cargo\.toml|package\.json|CHANGELOG\.md|README\.md)$' || true)"
  if [ -n "${files}" ]; then
    changed=1
    echo
    echo "${path}: $(echo "${files}" | wc -l | tr -d ' ') file(s) changed"
    echo "${files}" | head -n 20 | sed 's/^/  /'
  fi
done

# One line per crate family, version and source: the GroveDB crates all move together.
pinned() {
  git show "$1:Cargo.lock" | awk '
    /^name = /    { name = $3; gsub(/"/, "", name) }
    /^version = / { version = $3; gsub(/"/, "", version) }
    /^source = /  { source = $3; gsub(/"/, "", source); sub(/#.*/, "", source) }
    /^$/ {
      if (name ~ /^(grovedb|tenderdash|dashcore|dash-spv|key-wallet|bls-)/) {
        family = name
        sub(/[-_].*/, "", family)
        print family, version, source
      }
      name = ""; version = ""; source = ""
    }
  ' | sort -u
}

lock_diff="$(diff <(pinned "${deployed}") <(pinned "${tested}") || true)"
if [ -n "${lock_diff}" ]; then
  changed=1
  echo
  echo "Cargo.lock: pinned GroveDB, Tenderdash or Dash Core crates differ"
  echo "${lock_diff}" | sed 's/^/  /'
  echo "  Compare the two revisions in that repository: a version bump with no code change is fine."
fi

echo
if [ "${changed}" -eq 0 ]; then
  echo "No change in consensus, proof or node API code. A build of ${tested} can be tested against ${deployed}."
  exit 0
fi
echo "Changes found. Read them: if any can alter what a node accepts, stores or proves, test a build of ${deployed} or wait for the network to be updated."
exit 1

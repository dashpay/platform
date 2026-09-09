#!/usr/bin/env bash

set -euo pipefail

usage() {
  echo "Usage: $0 [--allow-non-tag-target] <baseline-tag> <target-tag-or-ref> [old-protocol] [new-protocol]" >&2
}

allow_non_tag_target=false
if [ "${1:-}" = "--allow-non-tag-target" ]; then
  allow_non_tag_target=true
  shift
fi

if [ "$#" -lt 2 ] || [ "$#" -gt 4 ]; then
  usage
  exit 2
fi

baseline_ref=$1
target_ref=$2
old_protocol=${3:-}
new_protocol=${4:-}

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "Error: run this script inside a git worktree" >&2
  exit 1
}
cd "$repo_root"

require_release_tag() {
  ref=$1
  label=$2

  if ! git show-ref --verify --quiet "refs/tags/${ref}"; then
    echo "Error: $label '$ref' is not an exact local release tag" >&2
    exit 1
  fi
}

resolve_commit() {
  git rev-parse --verify "${1}^{commit}" 2>/dev/null || {
    echo "Error: local ref '$1' does not resolve to a commit; review mode does not fetch" >&2
    exit 1
  }
}

package_version() {
  git show "${1}:packages/dashmate/package.json" 2>/dev/null \
    | awk -F'"' '/"version"[[:space:]]*:/ { print $4; exit }'
}

verify_protocol_support() {
  commit=$1
  ref_label=$2
  protocol=$3
  version_path="packages/rs-platform-version/src/version/v${protocol}.rs"

  case "$protocol" in
    ''|*[!0-9]*)
      echo "Error: protocol '$protocol' must be an unsigned integer" >&2
      exit 2
      ;;
  esac

  registry_path="packages/rs-platform-version/src/version/protocol_version.rs"
  modules_path="packages/rs-platform-version/src/version/mod.rs"
  if ! git cat-file -e "${commit}:${version_path}" 2>/dev/null \
    || ! git show "${commit}:${modules_path}" 2>/dev/null \
      | grep -Eq "^pub mod v${protocol};$" \
    || ! git show "${commit}:${registry_path}" 2>/dev/null \
      | grep -Eq "^[[:space:]]*PLATFORM_V${protocol},$"; then
    echo "Error: $ref_label ref does not register requested protocol $protocol" >&2
    exit 1
  fi

  echo "$ref_label protocol $protocol: ${commit}:${version_path}"
}

require_release_tag "$baseline_ref" "baseline"
target_kind="release tag"
if git show-ref --verify --quiet "refs/tags/${target_ref}"; then
  :
elif [ "$allow_non_tag_target" = true ]; then
  target_kind="pre-release ref"
else
  echo "Error: target '$target_ref' is not an exact local release tag" >&2
  echo "Use --allow-non-tag-target only for a pinned pre-release review." >&2
  exit 1
fi

baseline_sha=$(resolve_commit "$baseline_ref")
target_sha=$(resolve_commit "$target_ref")
baseline_version=$(package_version "$baseline_sha")
target_version=$(package_version "$target_sha")

echo "Protocol upgrade review inventory"
echo
echo "Baseline: $baseline_ref"
echo "  Kind: release tag"
echo "  SHA: $baseline_sha"
echo "  Dashmate: ${baseline_version:-unavailable}"
echo "Target: $target_ref"
echo "  Kind: $target_kind"
echo "  SHA: $target_sha"
echo "  Dashmate: ${target_version:-unavailable}"
if [ -n "$old_protocol" ] || [ -n "$new_protocol" ]; then
  echo "Requested protocol: ${old_protocol:-auto} -> ${new_protocol:-auto}"
fi

if [ -n "$old_protocol" ]; then
  echo
  echo "Requested protocol support"
  verify_protocol_support "$baseline_sha" "Baseline" "$old_protocol"
  verify_protocol_support "$target_sha" "Target" "$old_protocol"
fi
if [ -n "$new_protocol" ]; then
  if [ -z "$old_protocol" ]; then
    echo
    echo "Requested protocol support"
  fi
  verify_protocol_support "$target_sha" "Target" "$new_protocol"
fi

echo
echo "Changed upgrade-sensitive seed files (not a complete consensus inventory)"
git diff --name-status "$baseline_sha..$target_sha" -- \
  packages/rs-platform-version \
  packages/rs-drive-abci/src/execution/engine \
  packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade \
  packages/rs-drive-abci/tests \
  packages/rs-drive/src/cache \
  packages/data-contracts \
  packages/dpns-contract \
  packages/document-history-contract \
  packages/dashmate/configs \
  packages/dashmate/src/commands

echo
echo "Complete consensus-package directory summary"
git diff --dirstat=files,0 "$baseline_sha..$target_sha" -- \
  packages/rs-drive \
  packages/rs-drive-abci \
  packages/rs-dpp \
  packages/rs-platform-version \
  packages/data-contracts \
  packages/dpns-contract \
  packages/document-history-contract

echo
echo "Target dispatch and threshold files"
git grep -l \
  -e 'upgrade_protocol_version_on_epoch_change' \
  -e 'check_for_desired_protocol_upgrade' \
  -e 'perform_events_on_first_block_of_protocol_change' \
  -e 'next_epoch_protocol_version' \
  "$target_sha" -- \
  packages/rs-drive-abci/src/execution \
  packages/rs-platform-version/src/version \
  || true

if [ -n "$new_protocol" ]; then
  echo
  echo "Target transition symbols for protocol $new_protocol"
  git grep -n \
    -e "transition_to_version_${new_protocol}" \
    "$target_sha" -- \
    packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade \
    || true
fi

echo
echo "Target transition dispatcher branches"
git grep -n \
  -e 'transition_to_version_' \
  -e 'protocol_version.*=>' \
  "$target_sha" -- \
  packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade \
  | sed -n '1,160p' \
  || true

echo
echo "Existing upgrade test files"
git ls-tree -r --name-only "$target_sha" -- \
  packages/rs-drive-abci/tests \
  packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade \
  | awk '/upgrade|protocol/'

echo
echo "Focused transition diff summary"
diff_paths=(
  packages/rs-drive-abci/src/execution/engine/run_block_proposal/mod.rs
  packages/rs-drive-abci/src/execution/engine/run_block_proposal/v0/mod.rs
  packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade
  packages/rs-drive/src/cache/system_contracts.rs
)
if [ -n "$new_protocol" ]; then
  diff_paths+=("packages/rs-platform-version/src/version/v${new_protocol}.rs")
fi

git diff --stat "$baseline_sha..$target_sha" -- "${diff_paths[@]}"
echo
echo "Focused transition diff (first 1200 lines; inspect source files directly for the verdict)"
git diff --find-renames --unified=12 "$baseline_sha..$target_sha" -- "${diff_paths[@]}" \
  | sed -n '1,1200p'

echo
echo "Review required: read references/review-checklist.md and classify the transition"
echo "before deciding the deterministic, local-release, and snapshot test modes."

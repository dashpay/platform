#!/bin/bash
set -euo pipefail

# Extract the SPV storage of SwiftExampleApp from an iOS Simulator
#
# `CoreContentView.startSync()` points the platform-wallet SPV client at
# a per-network directory inside the app's Documents sandbox:
#
#     <sandbox>/Documents/SPV/<network>/
#         headers / filters / masternodes / state ...
#
# (`network` is one of mainnet/testnet/devnet/regtest etc.) This script
# picks a simulator + app container, then lets the developer pick which
# network's SPV store to extract (defaults to the only one present, or
# an interactive picker when there are several).
#
# Usage:
#   ./get_spv_storage.sh [--bundle-id <id>] [--out <dir>]
#                                  [--device <name|udid>]
#                                  [--network <name|all>]
#
# Defaults:
#   bundle-id: org.dashfoundation.SwiftExampleApp
#   out:       ./spv-storage-<device-name>-<network>-<timestamp>
#   device:    interactive picker when more than one is available
#   network:   interactive picker when more than one is available
#              (use --network all to grab every network at once)

BUNDLE_ID="org.dashfoundation.SwiftExampleApp"
OUT_DIR=""
DEVICE=""
NETWORK=""

need_value() {
  if [ "$#" -lt 2 ]; then
    echo "Missing value for $1" >&2; exit 2
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --bundle-id) need_value "$@"; BUNDLE_ID="$2"; shift 2 ;;
    --out)       need_value "$@"; OUT_DIR="$2";   shift 2 ;;
    --device)    need_value "$@"; DEVICE="$2";    shift 2 ;;
    --network)   need_value "$@"; NETWORK="$2";   shift 2 ;;
    -h|--help)
      sed -n '4,27p' "$0"; exit 0 ;;
    *)
      echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

# ---------- simulator picker ----------

json="$(xcrun simctl list devices --json available)"
table="$(printf '%s' "$json" | jq -r '
  .devices
  | to_entries[]
  | .key as $rt
  | .value[]
  | select(.isAvailable)
  | [.udid, .name, .state, ($rt | sub("com.apple.CoreSimulator.SimRuntime."; ""))]
  | @tsv
')"

if [ -z "$table" ]; then
  echo "No available simulators found." >&2
  exit 1
fi

pick_device() {
  if [ -n "$DEVICE" ]; then
    matched="$(printf '%s\n' "$table" | awk -F'\t' -v q="$DEVICE" '$1==q || index($2,q)')"

    if [ -z "$matched" ]; then
      echo "No simulator matches '$DEVICE'." >&2
      exit 1
    fi

    booted="$(printf '%s\n' "$matched" | awk -F'\t' '$3=="Booted"')"

    if [ -n "$booted" ]; then
      printf '%s\n' "$booted" | head -1
    else
      printf '%s\n' "$matched" | head -1
    fi

    return
  fi

  sorted="$(printf '%s\n' "$table" | awk -F'\t' '
    BEGIN { OFS="\t" }
    { print ($3=="Booted" ? "0" : "1"), $0 }
  ' | sort -k1,1 | cut -f2-)"

  booted_count="$(printf '%s\n' "$sorted" | awk -F'\t' '$3=="Booted"' | wc -l | tr -d ' ')"
  if [ "$booted_count" = "1" ]; then
    printf '%s\n' "$sorted" | awk -F'\t' '$3=="Booted"' | head -1
    return
  fi

  echo "Available simulators:" >&2

  i=1
  while IFS=$'\t' read -r udid name state runtime; do
    printf "  [%d] %-32s %-9s %-22s %s\n" "$i" "$name" "$state" "$runtime" "$udid" >&2
    i=$((i + 1))
  done <<< "$sorted"

  printf "Pick one [1]: " >&2

  read -r choice
  choice="${choice:-1}"

  if ! [[ "$choice" =~ ^[0-9]+$ ]]; then
    echo "Not a number: $choice" >&2; exit 1
  fi

  row="$(printf '%s\n' "$sorted" | sed -n "${choice}p")"

  if [ -z "$row" ]; then
    echo "Choice out of range." >&2; exit 1
  fi

  printf '%s\n' "$row"
}

row="$(pick_device)"
UDID="$(printf '%s' "$row" | cut -f1)"
NAME="$(printf '%s' "$row" | cut -f2)"
STATE="$(printf '%s' "$row" | cut -f3)"
echo "Using simulator: $NAME ($UDID, $STATE)"

# ---------- resolve sandbox ----------

if ! container="$(xcrun simctl get_app_container "$UDID" "$BUNDLE_ID" data 2>/dev/null)"; then
  echo "Could not resolve the data container for $BUNDLE_ID on $NAME." >&2
  echo "Is the app installed? Try installing + launching it first" >&2
  exit 1
fi

root="$container/Documents/SPV"
if [ ! -d "$root" ]; then
  echo "No SPV storage directory at $root." >&2
  echo "The app hasn't started an SPV sync yet (CoreContentView.startSync())." >&2
  echo "Launch the app, start a sync once, and re-run." >&2
  exit 1
fi

# ---------- network picker ----------

# Networks are subdirectories directly under the SPV root
# (mainnet/testnet/devnet/...). Sort by name for a stable picker.
#
# For macOS compability:
# macOS still ships Bash 3.2 where `mapfile` is unavailable, use an
# explicit loop.
networks=()
while IFS= read -r n; do
  networks+=("$n")
done < <(find "$root" -mindepth 1 -maxdepth 1 -type d \
  -exec basename {} \; | sort)

if [ "${#networks[@]}" -eq 0 ]; then
  echo "No network directories under $root." >&2
  echo "Start an SPV sync once so a <network> store is created." >&2
  exit 1
fi

pick_network() {
  if [ "$NETWORK" = "all" ]; then
    printf '%s\n' "__all__"
    return
  fi

  if [ -z "$NETWORK" ] && [ "${#networks[@]}" -eq 1 ]; then
    printf '%s\n' "${networks[0]}"
    return
  fi

  if [ -n "$NETWORK" ]; then
    # Match exactly or by unique prefix.
    matched=()
    for n in "${networks[@]}"; do
      case "$n" in
        "$NETWORK"*) matched+=("$n") ;;
      esac
    done

    if [ "${#matched[@]}" -eq 0 ]; then
      echo "No network matches '$NETWORK'." >&2; exit 1
    fi

    if [ "${#matched[@]}" -gt 1 ]; then
      echo "Network prefix '$NETWORK' is ambiguous; matches:" >&2
      printf '  %s\n' "${matched[@]}" >&2
      exit 1
    fi

    printf '%s\n' "${matched[0]}"

    return
  fi

  echo "Available networks:" >&2

  i=1
  for n in "${networks[@]}"; do
    # Annotate with on-disk size so the picker doubles as a
    # "how much store is here" view.
    size="$(du -sh "$root/$n" 2>/dev/null | cut -f1 || true)"
    printf "  [%d] %-12s (%s)\n" "$i" "$n" "${size:-?}" >&2
    i=$((i + 1))
  done

  printf "Pick one [1]: " >&2

  read -r choice
  choice="${choice:-1}"

  if ! [[ "$choice" =~ ^[0-9]+$ ]] \
      || [ "$choice" -lt 1 ] || [ "$choice" -gt "${#networks[@]}" ]; then
    echo "Choice out of range." >&2; exit 1
  fi

  pick="${networks[$((choice - 1))]}"

  printf '%s\n' "$pick"
}

NETWORK_ID="$(pick_network)"

# ---------- copy ----------

safe_name="$(printf '%s' "$NAME" | tr ' /' '__')"

if [ "$NETWORK_ID" = "__all__" ]; then
  src="$root"
  label="all"
else
  src="$root/$NETWORK_ID"
  label="$NETWORK_ID"
fi

if [ -z "$OUT_DIR" ]; then
  stamp="$(date +%Y-%m-%dT%H-%M-%S)"
  OUT_DIR="spv-storage-${safe_name}-${label}-${stamp}"
fi

mkdir -p "$OUT_DIR"
cp -R "$src/." "$OUT_DIR/"

echo
echo "Copied ${label} SPV storage to $OUT_DIR/"

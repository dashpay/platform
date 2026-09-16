#!/bin/bash
set -euo pipefail

# Extract a single SwiftDashSDK session's logs from an iOS Simulator
#
# `LoggingPreferences.configure()` (SDKLogger.swift) independently starts
# Swift and Rust file sinks under a session-stamped root:
#
#     <sandbox>/Library/Logs/SwiftDashSDK/<YYYY-MM-DDTHH-mm-ss>/
#         swift/run.log
#         dash_sdk/run.log
#         platform_wallet/run.log
#         dash_spv/run.log
#         key_wallet/run.log
#         grpc/run.log
#
# This script picks a simulator + app container, then lets the
# developer pick which session to extract (defaults to the latest).
#
# Usage:
#   ./get_logs.sh [--bundle-id <id>] [--out <dir>]
#                               [--device <name|udid>]
#                               [--session <ts|latest>]
#
# Defaults:
#   bundle-id: org.dashfoundation.DashDeveloperPro
#   out:       ./logs-<device-name>-<session-ts>
#   device:    interactive picker when more than one is available
#   session:   interactive picker when more than one is available

BUNDLE_ID="org.dashfoundation.DashDeveloperPro"
OUT_DIR=""
DEVICE=""
SESSION=""

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
    --session)   need_value "$@"; SESSION="$2";   shift 2 ;;
    -h|--help)
      sed -n '3,30p' "$0"; exit 0 ;;
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

root="$container/Library/Logs/SwiftDashSDK"
if [ ! -d "$root" ]; then
  echo "No SwiftDashSDK log directory at $root." >&2
  echo "The app hasn't completed a launch that called" >&2
  echo "  LoggingPreferences.configure()" >&2
  echo "yet. Launch the app once and re-run." >&2
  exit 1
fi

# ---------- session picker ----------

# Sessions are timestamped directories directly under the SDK root.
# Sort newest-first so the picker default ([1]) matches what the
# developer most likely wants right after a run.
#
# For macOS compability:
# macOS still ships Bash 3.2 where `mapfile` is unavailable, use an
# explicit loop.
sessions=()
while IFS= read -r s; do
  sessions+=("$s")
done < <(find "$root" -mindepth 1 -maxdepth 1 -type d \
  -name '[0-9]*' -exec basename {} \; | sort -r)

if [ "${#sessions[@]}" -eq 0 ]; then
  echo "No session directories under $root." >&2
  echo "Check that the app actually called LoggingPreferences.configure()" >&2
  exit 1
fi

pick_session() {
  if [ "$SESSION" = "latest" ] || { [ -z "$SESSION" ] && [ "${#sessions[@]}" -eq 1 ]; }; then
    printf '%s\n' "${sessions[0]}"
    return
  fi

  if [ -n "$SESSION" ]; then
    # Match exactly or by unique prefix (handy for short stamps
    # like just "2026-06-01").
    matched=()
    for s in "${sessions[@]}"; do
      case "$s" in
        "$SESSION"|"$SESSION"*) matched+=("$s") ;;
      esac
    done

    if [ "${#matched[@]}" -eq 0 ]; then
      echo "No session matches '$SESSION'." >&2; exit 1
    fi

    if [ "${#matched[@]}" -gt 1 ]; then
      echo "Session prefix '$SESSION' is ambiguous; matches:" >&2
      printf '  %s\n' "${matched[@]}" >&2
      exit 1
    fi

    printf '%s\n' "${matched[0]}"

    return
  fi

  echo "Available sessions (newest first):" >&2

  i=1
  for s in "${sessions[@]}"; do
    # Annotate with the latest mtime of any contained file so the
    # picker doubles as a "how recent is this session" view.
    last="$(find "$root/$s" -type f -name 'run.log' \
              -exec stat -f '%m %Sm' -t '%H:%M:%S' {} \; 2>/dev/null \
              | sort -n | tail -1 | cut -d' ' -f2- || true)"
    printf "  [%d] %s  (last write: %s)\n" "$i" "$s" "${last:-<empty>}" >&2
    i=$((i + 1))
  done

  printf "Pick one [1]: " >&2

  read -r choice
  choice="${choice:-1}"

  if ! [[ "$choice" =~ ^[0-9]+$ ]]; then
    echo "Not a number: $choice" >&2; exit 1
  fi

  pick="${sessions[$((choice - 1))]:-}"

  if [ -z "$pick" ]; then
    echo "Choice out of range." >&2; exit 1
  fi

  printf '%s\n' "$pick"
}

SESSION_ID="$(pick_session)"
src="$root/$SESSION_ID"

# ---------- copy ----------

if [ -z "$OUT_DIR" ]; then
  safe_name="$(printf '%s' "$NAME" | tr ' /' '__')"
  OUT_DIR="logs-${safe_name}-${SESSION_ID}"
fi

mkdir -p "$OUT_DIR"
cp -R "$src/." "$OUT_DIR/"

echo
echo "Copied session to $OUT_DIR/"

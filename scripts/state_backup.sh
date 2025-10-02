#!/usr/bin/env bash
set -euo pipefail
DASHMATE_BIN=${DASHMATE_BIN:-dashmate}
abci_usage() {
  printf 'usage: transfer_drive_abci_state.sh export|import --component abci|tenderdash\n\n  export [archive] [config]\n  import <archive> [config]\n' >&2
  exit 1
}
abci_timestamp() { date +%Y%m%dT%H%M%S; }
abci_resolve_config() {
  local cfg=${1:-}
  if [ -n "$cfg" ]; then echo "$cfg"; else ${DASHMATE_BIN} config default; fi
}
abci_resolve_volume() {
  local cfg=$1
  local project=$(${DASHMATE_BIN} config envs --config "$cfg" | awk -F= '$1=="COMPOSE_PROJECT_NAME"{print $2}')
  echo "${project}_${ABCI_VOLUME_SUFFIX:-drive_abci_data}"
}
abci_export_state() {
  local archive=${1:-drive_abci_state_$(abci_timestamp).tar.gz}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(abci_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  mkdir -p "$dir"
  docker run --rm -v "${volume}:/data:ro" -v "$dir:/out" busybox:1.36 sh -c "cd /data && tar cz --numeric-owner -f /out/$file ."
  echo "$archive"
  abci_transfer_help >&2
}
abci_import_state() {
  local archive=${1:?archive required}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(abci_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  [ -f "$archive" ] || {
    echo "archive not found" >&2
    exit 1
  }
  docker volume inspect "$volume" >/dev/null 2>&1 || docker volume create "$volume" >/dev/null
  docker run --rm -v "${volume}:/data" busybox:1.36 sh -c 'rm -rf /data/*'
  docker run --rm -v "${volume}:/data" -v "$dir:/in:ro" busybox:1.36 sh -c "cd /data && tar xzp -f /in/$file"
}
abci_transfer_help() { printf 'Move the archive manually, for example:\n  scp drive_abci_state_<timestamp>.tar.gz user@remote:/path\nThen on the target host run:\n  scripts/transfer_drive_abci_state.sh import /path/drive_abci_state_<timestamp>.tar.gz [config] --component abci\n'; }
tenderdash_resolve_volume() {
  local cfg=$1
  local project=$(${DASHMATE_BIN} config envs --config "$cfg" | awk -F= '$1=="COMPOSE_PROJECT_NAME"{print $2}')
  echo "${project}_${TENDERDASH_VOLUME_SUFFIX:-drive_tenderdash}"
}
tenderdash_export_state() {
  local archive=${1:-tenderdash_state_$(abci_timestamp).tar.gz}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(tenderdash_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  mkdir -p "$dir"
  docker run --rm -v "${volume}:/tenderdash:ro" -v "$dir:/out" busybox:1.36 sh -c "set -e; cd /tenderdash; for f in data/blockstore.db data/evidence.db data/state.db data/tx_index.db; do [ -e \"\$f\" ] || { echo \"missing \$f\" >&2; exit 1; }; done; tar cz --numeric-owner -f /out/$file data/blockstore.db data/evidence.db data/state.db data/tx_index.db"
  echo "$archive"
  tenderdash_transfer_help >&2
}
tenderdash_import_state() {
  local archive=${1:?archive required}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(tenderdash_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  [ -f "$archive" ] || {
    echo "archive not found" >&2
    exit 1
  }
  docker volume inspect "$volume" >/dev/null 2>&1 || docker volume create "$volume" >/dev/null
  docker run --rm -v "${volume}:/tenderdash" busybox:1.36 sh -c 'set -e; mkdir -p /tenderdash/data; rm -rf /tenderdash/data/blockstore.db /tenderdash/data/evidence.db /tenderdash/data/state.db /tenderdash/data/tx_index.db'
  docker run --rm -v "${volume}:/tenderdash" -v "$dir:/in:ro" busybox:1.36 sh -c "cd /tenderdash && tar xzp -f /in/$file"
}
tenderdash_transfer_help() { printf 'Move the archive manually, for example:\n  scp tenderdash_state_<timestamp>.tar.gz user@remote:/path\nThen on the target host run:\n  scripts/transfer_drive_abci_state.sh import /path/tenderdash_state_<timestamp>.tar.gz [config] --component tenderdash\n'; }
cmd=${1:-}
[ -n "$cmd" ] || abci_usage
shift || true
component=""
rest=()
while [ $# -gt 0 ]; do
  case $1 in
  --component)
    if [ $# -lt 2 ]; then
      echo "Missing value for --component" >&2
      exit 1
    fi
    component=${2:-}
    shift 2
    ;;
  --component=*)
    component=${1#*=}
    shift
    ;;
  *)
    rest+=("$1")
    shift
    ;;
  esac
done
set -- "${rest[@]}"
if [ -z "$component" ]; then
  abci_usage
fi
case $component in
abci)
  case $cmd in
  export) abci_export_state "${1:-}" "${2:-}" ;;
  import) abci_import_state "${1:-}" "${2:-}" ;;
  *) abci_usage ;;
  esac
  ;;
tenderdash)
  case $cmd in
  export) tenderdash_export_state "${1:-}" "${2:-}" ;;
  import) tenderdash_import_state "${1:-}" "${2:-}" ;;
  *) abci_usage ;;
  esac
  ;;
*)
  echo "Unsupported component: $component" >&2
  abci_usage
  ;;
esac

#!/usr/bin/env bash
set -euo pipefail
DASHMATE_CMD_RAW=${DASHMATE_CMD:-${DASHMATE_BIN:-dashmate}}
read -r -a DASHMATE_CMD <<<"$DASHMATE_CMD_RAW"
abci_usage() {
  cat >&2 <<'EOF'
Usage:
  scripts/state_backup.sh export <component> [archive] [options]
  scripts/state_backup.sh import <component> <archive> [options]

Components:
  abci | tenderdash

Options:
  --config <name>     Dashmate config name (passed to --config)
  --dashmate <cmd>    Dashmate command (default: dashmate)
  -h, --help          Show this help

Examples:
  scripts/state_backup.sh export abci
  scripts/state_backup.sh export tenderdash /tmp/td_state.tar.gz --config local
  scripts/state_backup.sh import abci /tmp/abci_state.tar.gz --dashmate "docker compose run dashmate"
EOF
  exit 1
}
timestamp() { date +%Y%m%dT%H%M%S; }
abci_resolve_config() {
  local cfg=${1:-}
  if [ -n "$cfg" ]; then echo "$cfg"; else "${DASHMATE_CMD[@]}" config default; fi
}
abci_resolve_volume() {
  local cfg=$1
  local project=$("${DASHMATE_CMD[@]}" config envs --config "$cfg" | awk -F= '$1=="COMPOSE_PROJECT_NAME"{print $2}')
  echo "${project}_${ABCI_VOLUME_SUFFIX:-drive_abci_data}"
}
transfer_help() {
  local component=$1
  local archive=$2
  printf 'Move the archive manually, for example:\n  scp %s user@remote:\nThen on the target host run:\n  scripts/state_backup.sh import %s %s [--config <name>]\n' "$archive" "$component" "$(basename "$archive")"
}
abci_export_state() {
  local cfg=$(abci_resolve_config "${2:-}")
  local archive=${1:-drive_abci_state_${cfg}_$(timestamp).tar.gz}
  local volume
  volume=$(abci_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  mkdir -p "$dir"
  docker run --rm --network none \
    -v "${volume}:/data:ro" -v "$dir:/out" -w /data \
    busybox:1.36 tar cz --numeric-owner -f "/out/$file" .
  echo "$archive"
  transfer_help "abci" "$archive" >&2
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
  docker run --rm --network none -v "${volume}:/data" \
    busybox:1.36 sh -c 'rm -rf /data/*'
  docker run --rm --network none \
    -v "${volume}:/data" -v "$dir:/in:ro" -w /data \
    busybox:1.36 tar xzp -f "/in/$file"
}
tenderdash_resolve_volume() {
  local cfg=$1
  local project=$("${DASHMATE_CMD[@]}" config envs --config "$cfg" | awk -F= '$1=="COMPOSE_PROJECT_NAME"{print $2}')
  echo "${project}_${TENDERDASH_VOLUME_SUFFIX:-drive_tenderdash}"
}
tenderdash_export_state() {
  local cfg=$(abci_resolve_config "${2:-}")
  local archive=${1:-tenderdash_state_${cfg}_$(timestamp).tar.gz}
  local volume
  volume=$(tenderdash_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  mkdir -p "$dir"
  docker run --rm --network none \
    -v "${volume}:/tenderdash:ro" -v "$dir:/out" \
    busybox:1.36 sh -c \
    'set -e; cd /tenderdash; for f in data/blockstore.db data/evidence.db data/state.db data/tx_index.db; do [ -e "$f" ] || { echo "missing $f" >&2; exit 1; }; done; exec tar cz --numeric-owner -f "/out/$1" data/blockstore.db data/evidence.db data/state.db data/tx_index.db' \
    export "$file"
  echo "$archive"
  transfer_help "tenderdash" "$archive" >&2
}
tenderdash_import_state() (
  local archive=${1:?archive required}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(tenderdash_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  [ -f "$archive" ] || {
    echo "archive not found" >&2
    exit 1
  }

  local staging_volume
  staging_volume=$(docker volume create)
  trap 'docker volume rm -f "$staging_volume" >/dev/null 2>&1 || true' EXIT

  # Extract the untrusted archive into an isolated volume first. Archive paths
  # can affect only the disposable container and staging volume at this point.
  docker run --rm --network none \
    -v "${staging_volume}:/staging" -v "$dir:/in:ro" -w /staging \
    busybox:1.36 tar xzp -f "/in/$file"

  # Accept only the four database trees emitted by tenderdash_export_state.
  docker run --rm --network none -v "${staging_volume}:/staging" \
    busybox:1.36 sh -c '
      set -eu
      for database in blockstore.db evidence.db state.db tx_index.db; do
        [ -d "/staging/data/$database" ] || {
          echo "missing required Tenderdash database" >&2
          exit 1
        }
      done
      unexpected=$(find /staging -mindepth 1 \
        ! -path /staging/data \
        ! -path /staging/data/blockstore.db ! -path "/staging/data/blockstore.db/*" \
        ! -path /staging/data/evidence.db ! -path "/staging/data/evidence.db/*" \
        ! -path /staging/data/state.db ! -path "/staging/data/state.db/*" \
        ! -path /staging/data/tx_index.db ! -path "/staging/data/tx_index.db/*" \
        -print -quit)
      [ -z "$unexpected" ] || {
        echo "unexpected Tenderdash archive member" >&2
        exit 1
      }
      unsupported=$(find /staging -mindepth 1 ! -type d ! -type f -print -quit)
      [ -z "$unsupported" ] || {
        echo "unsupported Tenderdash archive member type" >&2
        exit 1
      }
      [ "$(find /staging -mindepth 1 | wc -l)" -le 200000 ] || {
        echo "Tenderdash archive contains too many members" >&2
        exit 1
      }
      [ "$(du -sk /staging | cut -f1)" -le 209715200 ] || {
        echo "Tenderdash archive exceeds the restore budget" >&2
        exit 1
      }
      chmod -R u=rwX,go= /staging/data
    '

  docker volume inspect "$volume" >/dev/null 2>&1 || docker volume create "$volume" >/dev/null

  # Copy and validate the candidate tree before replacing the four live roots.
  docker run --rm --network none \
    -v "${staging_volume}:/staging:ro" -v "${volume}:/target" \
    busybox:1.36 sh -c '
      set -eu
      rm -rf /target/.dashmate-restore
      mkdir -p /target/.dashmate-restore
      cd /staging
      tar cf - data/blockstore.db data/evidence.db data/state.db data/tx_index.db \
        | (cd /target/.dashmate-restore && tar xpf -)
      mkdir -p /target/data
      for database in blockstore.db evidence.db state.db tx_index.db; do
        [ -d "/target/.dashmate-restore/data/$database" ]
        rm -rf "/target/data/$database"
        mv "/target/.dashmate-restore/data/$database" "/target/data/$database"
      done
      rm -rf /target/.dashmate-restore
    '
)
cmd=${1:-}
component=${2:-}
[ -n "$cmd" ] || abci_usage
[ -n "$component" ] || abci_usage
shift 2 || true

archive=""
config=""
dashmate=""
while [ $# -gt 0 ]; do
  case $1 in
  --config)
    [ $# -ge 2 ] || { echo "Missing value for --config" >&2; exit 1; }
    config=$2
    shift 2
    ;;
  --config=*)
    config=${1#*=}
    shift
    ;;
  --dashmate)
    [ $# -ge 2 ] || { echo "Missing value for --dashmate" >&2; exit 1; }
    dashmate=$2
    shift 2
    ;;
  --dashmate=*)
    dashmate=${1#*=}
    shift
    ;;
  -h|--help)
    abci_usage
    ;;
  *)
    if [ -z "$archive" ]; then
      archive=$1
      shift
    else
      echo "Unexpected argument: $1" >&2
      exit 1
    fi
    ;;
  esac
done

if [ -n "$dashmate" ]; then
  DASHMATE_CMD_RAW=$dashmate
  read -r -a DASHMATE_CMD <<<"$DASHMATE_CMD_RAW"
fi

case $component in
abci)
  case $cmd in
  export) abci_export_state "${archive:-}" "${config:-}" ;;
  import)
    [ -n "$archive" ] || { echo "archive is required for import" >&2; exit 1; }
    abci_import_state "$archive" "${config:-}"
    ;;
  *) abci_usage ;;
  esac
  ;;
tenderdash)
  case $cmd in
  export) tenderdash_export_state "${archive:-}" "${config:-}" ;;
  import)
    [ -n "$archive" ] || { echo "archive is required for import" >&2; exit 1; }
    tenderdash_import_state "$archive" "${config:-}"
    ;;
  *) abci_usage ;;
  esac
  ;;
*)
  echo "Unsupported component: $component" >&2
  abci_usage
  ;;
esac

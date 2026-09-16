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
abci_import_state() (
  local archive=${1:?archive required}
  local cfg=$(abci_resolve_config "${2:-}")
  local volume
  volume=$(abci_resolve_volume "$cfg")
  local dir=$(dirname "$archive") file=$(basename "$archive")
  [ -f "$archive" ] || {
    echo "archive not found" >&2
    exit 1
  }

  local staging_volume
  staging_volume=$(docker volume create)
  trap 'docker volume rm -f "$staging_volume" >/dev/null 2>&1 || true' EXIT

  docker run --rm --network none -v "$dir:/in:ro" \
    busybox:1.36 sh -c '
      set -euo pipefail
      archive=$1
      tar tvzf "$archive" | while read -r mode owner size date time member extra; do
        [ -n "$member" ] && [ -z "${extra:-}" ] || {
          echo "unsupported ABCI archive member name" >&2
          exit 1
        }
        case "$mode" in
          d*|-*) ;;
          *)
            echo "unsupported ABCI archive member type" >&2
            exit 1
            ;;
        esac
        case "$size" in
          ""|*[!0-9]*)
            echo "invalid ABCI archive member size" >&2
            exit 1
            ;;
        esac
        members=$(( ${members:-0} + 1 ))
        bytes=$(( ${bytes:-0} + size ))
        [ "$members" -le 200000 ] || {
          echo "ABCI archive contains too many members" >&2
          exit 1
        }
        [ "$bytes" -le 214748364800 ] || {
          echo "ABCI archive exceeds the restore budget" >&2
          exit 1
        }
      done
      if ! tar tzf "$archive" | awk "
        {
          name = \$0
          if (name ~ /[[:space:]]/ || name ~ /^\// || index(name, sprintf(\"%c\", 92))) {
            invalid = 1
          }
          while (sub(/^\.\//, \"\", name)) {}
          while (sub(/\/$/, \"\", name)) {}
          if (name == \"\" || name == \".\") next
          if (name == \".dashmate-restore\" || name ~ /^\.dashmate-restore\// ||
              name == \".dashmate-previous\" || name ~ /^\.dashmate-previous\//) {
            invalid = 1
          }
          count = split(name, component, \"/\")
          for (i = 1; i <= count; i++) {
            if (component[i] == \"\" || component[i] == \".\" || component[i] == \"..\") {
              invalid = 1
            }
          }
          if (seen[name]++) invalid = 1
        }
        END { exit invalid ? 1 : 0 }
      "; then
        echo "ABCI archive contains an unsafe or duplicate member name" >&2
        exit 1
      fi
    ' preflight "/in/$file"

  docker run --rm --network none \
    -v "${staging_volume}:/staging" -v "$dir:/in:ro" -w /staging \
    busybox:1.36 sh -c '
      set -euo pipefail
      ulimit -f 419430400
      tar xzp -f "$1" &
      extraction_pid=$!
      while kill -0 "$extraction_pid" 2>/dev/null; do
        used_kib=$(du -sk /staging | cut -f1)
        if [ "$used_kib" -gt 209715200 ]; then
          kill "$extraction_pid" 2>/dev/null || true
          wait "$extraction_pid" 2>/dev/null || true
          echo "ABCI archive exceeds the restore budget" >&2
          exit 1
        fi
        sleep 1
      done
      wait "$extraction_pid"
      [ "$(du -sk /staging | cut -f1)" -le 209715200 ] || {
        echo "ABCI archive exceeds the restore budget" >&2
        exit 1
      }
      unsupported=$(find /staging -mindepth 1 ! -type d ! -type f -print -quit)
      [ -z "$unsupported" ] || {
        echo "unsupported ABCI archive member type" >&2
        exit 1
      }
      [ "$(find /staging -mindepth 1 | wc -l)" -le 200000 ] || {
        echo "ABCI archive contains too many members" >&2
        exit 1
      }
      chmod -R u=rwX,go= /staging
    ' extract "/in/$file"

  docker volume inspect "$volume" >/dev/null 2>&1 || docker volume create "$volume" >/dev/null

  # Build the complete candidate inside the live volume before swapping it in.
  # If any move fails, the EXIT trap restores every previous top-level entry.
  docker run --rm --network none \
    -v "${staging_volume}:/staging:ro" -v "${volume}:/target" \
    busybox:1.36 sh -c '
      set -euo pipefail
      candidate=/target/.dashmate-restore
      previous=/target/.dashmate-previous
      rm -rf "$candidate" "$previous"
      mkdir -p "$candidate" "$previous"
      cd /staging
      tar cf - . | (cd "$candidate" && tar xpf -)

      phase=backup
      rollback() {
        status=$?
        if [ "$status" -ne 0 ] && [ -d "$previous" ]; then
          if [ "$phase" = install ]; then
            find /target -mindepth 1 -maxdepth 1 \
              ! -name .dashmate-restore ! -name .dashmate-previous \
              -exec rm -rf {} \;
          fi
          find "$previous" -mindepth 1 -maxdepth 1 -exec mv {} /target/ \;
          rm -rf "$candidate" "$previous"
        fi
        trap - EXIT
        exit "$status"
      }
      trap rollback EXIT

      find /target -mindepth 1 -maxdepth 1 \
        ! -name .dashmate-restore ! -name .dashmate-previous \
        -exec mv {} "$previous"/ \;
      phase=install
      find "$candidate" -mindepth 1 -maxdepth 1 -exec mv {} /target/ \;
      rm -rf "$candidate" "$previous"
    '
)
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

  # Validate the archive metadata before writing any untrusted member to disk.
  # Tenderdash LevelDB trees use only simple names, so rejecting whitespace also
  # makes the verbose tar listing unambiguous to this preflight parser.
  docker run --rm --network none -v "$dir:/in:ro" \
    busybox:1.36 sh -c '
      set -euo pipefail
      archive=$1
      tar tvzf "$archive" | while read -r mode owner size date time member extra; do
        [ -n "$member" ] && [ -z "${extra:-}" ] || {
          echo "unsupported Tenderdash archive member name" >&2
          exit 1
        }
        case "$mode" in
          d*|-*) ;;
          *)
            echo "unsupported Tenderdash archive member type" >&2
            exit 1
            ;;
        esac
        case "$size" in
          ""|*[!0-9]*)
            echo "invalid Tenderdash archive member size" >&2
            exit 1
            ;;
        esac
        case "$member" in
          data|data/|data/blockstore.db|data/blockstore.db/*|data/evidence.db|data/evidence.db/*|data/state.db|data/state.db/*|data/tx_index.db|data/tx_index.db/*) ;;
          *)
            echo "unexpected Tenderdash archive member" >&2
            exit 1
            ;;
        esac
        members=$(( ${members:-0} + 1 ))
        bytes=$(( ${bytes:-0} + size ))
        [ "$members" -le 200000 ] || {
          echo "Tenderdash archive contains too many members" >&2
          exit 1
        }
        [ "$bytes" -le 214748364800 ] || {
          echo "Tenderdash archive exceeds the restore budget" >&2
          exit 1
        }
      done
      if ! tar tzf "$archive" | awk "
        /[[:space:]]/ || seen[\$0]++ { invalid = 1 }
        END { exit invalid ? 1 : 0 }
      "; then
        echo "Tenderdash archive contains an unsafe or duplicate member name" >&2
        exit 1
      fi
    ' preflight "/in/$file"

  # Extract into an isolated volume while continuously enforcing the actual
  # staging-volume budget. ulimit also prevents any single member from crossing it.
  docker run --rm --network none \
    -v "${staging_volume}:/staging" -v "$dir:/in:ro" -w /staging \
    busybox:1.36 sh -c '
      set -eu
      ulimit -f 419430400
      tar xzp -f "$1" &
      extraction_pid=$!
      while kill -0 "$extraction_pid" 2>/dev/null; do
        used_kib=$(du -sk /staging | cut -f1)
        if [ "$used_kib" -gt 209715200 ]; then
          kill "$extraction_pid" 2>/dev/null || true
          wait "$extraction_pid" 2>/dev/null || true
          echo "Tenderdash archive exceeds the restore budget" >&2
          exit 1
        fi
        sleep 1
      done
      wait "$extraction_pid"
      [ "$(du -sk /staging | cut -f1)" -le 209715200 ] || {
        echo "Tenderdash archive exceeds the restore budget" >&2
        exit 1
      }
    ' extract "/in/$file"

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

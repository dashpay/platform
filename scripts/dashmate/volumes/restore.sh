#!/usr/bin/env bash

set -euo pipefail

volume_dump_dir="$PWD/dashmate_volumes_dump"
metadata_file="$volume_dump_dir/metadata.json"
archive_file="$volume_dump_dir/data.tar.gz"

[ -f "$metadata_file" ] || { echo "metadata.json not found" >&2; exit 1; }
[ -f "$archive_file" ] || { echo "data.tar.gz not found" >&2; exit 1; }

jq -e 'type == "array" and length > 0 and all(.[]; type == "object")' \
  "$metadata_file" >/dev/null || {
  echo "invalid volume metadata" >&2
  exit 1
}

metadata_entries=()
while IFS= read -r metadata; do
  metadata_entries+=("$metadata")
done < <(jq -c '.[]' "$metadata_file")

volumes=()
for metadata in "${metadata_entries[@]}"; do
  volume=$(jq -er '.Name | select(type == "string")' <<<"$metadata")
  [[ $volume =~ ^dashmate_[A-Za-z0-9][A-Za-z0-9_.-]{0,199}$ ]] || {
    echo "invalid backup volume name" >&2
    exit 1
  }

  create_args=(docker volume create)
  jq -e '(.Labels == null) or (.Labels | type == "object")' <<<"$metadata" >/dev/null || {
    echo "volume labels must be an object" >&2
    exit 1
  }
  label_entries=()
  while IFS= read -r label; do
    label_entries+=("$label")
  done < <(jq -c '(.Labels // {}) | to_entries[]' <<<"$metadata")

  [ "${#label_entries[@]}" -le 128 ] || {
    echo "too many backup volume labels" >&2
    exit 1
  }

  for label in "${label_entries[@]}"; do
    key=$(jq -er '.key | select(type == "string")' <<<"$label")
    value=$(jq -er '.value | select(type == "string")' <<<"$label")
    [[ $key =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$ ]] || {
      echo "invalid backup volume label key" >&2
      exit 1
    }
    [[ ${#value} -le 1024 && $value != *$'\n'* && $value != *$'\r'* ]] || {
      echo "invalid backup volume label value" >&2
      exit 1
    }
    create_args+=(--label "$key=$value")
  done

  create_args+=(-- "$volume")
  "${create_args[@]}" >/dev/null
  volumes+=("$volume")
done

[ "${#volumes[@]}" -gt 0 ] || { echo "backup contains no volumes" >&2; exit 1; }

run_args=(docker run --rm --network none)
for volume in "${volumes[@]}"; do
  run_args+=(-v "$volume:/dashmate_volumes/$volume")
done
run_args+=(-v "$volume_dump_dir:/backup:ro")
run_args+=(busybox tar xf /backup/data.tar.gz -C /dashmate_volumes)

"${run_args[@]}"

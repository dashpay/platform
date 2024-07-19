#!/usr/bin/env bash

set -ex

# Directory where volume dumps are stored
volume_dump_dir="$PWD/dashmate_volumes_dump"
metadata_file="$volume_dump_dir/metadata.json"

# Read each line (metadata for each volume) and recreate the volume
cat $metadata_file | jq -c '.[]' | while read -r metadata; do
  volume=$(echo $metadata | jq -r '.Name')
  labels=$(echo $metadata | jq -r '.Labels | to_entries | map("--label \(.key)=\(.value)") | .[]')

  # Create volume with labels
  docker volume create $labels $volume
done

# Restore all volumes from the tarball
volume_args=$(cat $metadata_file | jq -r '.[].Name' | xargs -I {} echo -n "-v {}:/dashmate_volumes/{} ")

docker run --rm \
  $volume_args \
  -v $volume_dump_dir:/backup busybox tar xf /backup/data.tar.gz -C /dashmate_volumes

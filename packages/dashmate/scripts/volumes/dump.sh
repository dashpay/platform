#!/usr/bin/env bash

set -ex

# Directory where volume dumps will be stored
volume_dump_dir="$PWD/dashmate_volumes_dump"
metadata_file="$volume_dump_dir/metadata.json"

mkdir -p $volume_dump_dir

# Initialize the volume mount arguments
volume_mounts=""

# Initialize metadata file
echo "[]" > $metadata_file

# Temporary file for individual volume metadata
volume_metadata_file="$volume_dump_dir/volume_metadata.json"
temp_metadata_file="$volume_dump_dir/temp_metadata.json"

# Loop through each volume, prepare mounts and save metadata
for volume in $(docker volume ls --filter name=dashmate_ -q); do
  volume_mounts="$volume_mounts -v $volume:/all_volumes/$volume"

  # Get metadata for the current volume
  docker volume inspect $volume > $volume_metadata_file

  # Append metadata to the main file
  # Merge the current metadata array with the existing one
  jq -s '.[0] + .[1]' $metadata_file $volume_metadata_file > $temp_metadata_file && mv $temp_metadata_file $metadata_file

  rm $volume_metadata_file
done

# Run a container with all volumes mounted and perform the dump
docker run --rm $volume_mounts -v $volume_dump_dir:/backup busybox tar cf /backup/data.tar.gz -C /all_volumes .



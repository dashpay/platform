#!/bin/sh

# Check if file was passed
if [ $# -ne 1 ]; then
  echo "Usage: $0 <quorum_info_file.txt>"
  exit 1
fi

INPUT_FILE="$1"

if [ ! -f "$INPUT_FILE" ]; then
  echo "Error: File '$INPUT_FILE' does not exist."
  exit 1
fi

echo "Extracting hashes from $INPUT_FILE..."

# Extract and deduplicate 64-character hex strings
HASHES=$(grep -oE '"[0-9a-f]{64}"' "$INPUT_FILE" | tr -d '"' | sort -u)

echo "Found $(echo "$HASHES" | wc -l | tr -d ' ') unique hashes."

for HASH in $HASHES; do
  echo "Fetching quorum info for $HASH..."
  OUTPUT_FILE="quorum_info_60_75_${HASH}.json"

  dash-cli quorum info 5 "$HASH" > "$OUTPUT_FILE"

  if [ $? -ne 0 ]; then
    echo "⚠️  Failed to fetch quorum info for $HASH" >&2
    rm -f "$OUTPUT_FILE"
  else
    echo "✅ Saved to $OUTPUT_FILE"
  fi
done
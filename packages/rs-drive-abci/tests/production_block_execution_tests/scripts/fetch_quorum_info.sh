#!/bin/bash

set -euo pipefail

# === CONFIGURATION ===
RPC_USER="dashrpc"
RPC_PASSWORD="password"
RPC_PORT=9998  # default for mainnet
RPC_HOST="127.0.0.1"

# === INPUT PARAMETERS ===
if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <quorum_type_number> <input_file>"
  exit 1
fi

QUORUM_TYPE="$1"
INPUT_FILE="$2"

# === Resolve Quorum Type Name for Output Files ===
case "$QUORUM_TYPE" in
  1) TYPE_NAME="50_60" ;;
  2) TYPE_NAME="400_60" ;;
  3) TYPE_NAME="400_85" ;;
  4) TYPE_NAME="100_67" ;;
  5) TYPE_NAME="60_75" ;;
  6) TYPE_NAME="25_67" ;;
  100) TYPE_NAME="test" ;;
  101) TYPE_NAME="devnet" ;;
  102) TYPE_NAME="test_v17" ;;
  103) TYPE_NAME="test_dip0024" ;;
  104) TYPE_NAME="test_instantsend" ;;
  105) TYPE_NAME="devnet_dip0024" ;;
  106) TYPE_NAME="test_platform" ;;
  107) TYPE_NAME="devnet_platform" ;;
  111) TYPE_NAME="single_node" ;;
  0) TYPE_NAME="unknown" ;;
  *)
    echo "❌ Unknown quorum type: $QUORUM_TYPE"
    exit 1
    ;;
esac

# === Validate input file ===
if [[ ! -f "$INPUT_FILE" ]]; then
  echo "❌ Input file not found: $INPUT_FILE"
  exit 1
fi

# === Extract unique hashes from file ===
HASHES=$(grep -Eo '"[a-f0-9]{64}"' "$INPUT_FILE" | tr -d '"' | sort -u)

for HASH in $HASHES; do
  echo "Fetching quorum info for $HASH..."

  OUTPUT_FILE="quorum_info_${TYPE_NAME}_${HASH}.json"

  RESPONSE=$(curl -s --user "$RPC_USER:$RPC_PASSWORD" \
    --data-binary @- \
    -H 'content-type: application/json;' \
    http://$RPC_HOST:$RPC_PORT <<EOF
{
  "jsonrpc": "1.0",
  "id": "fetch_quorum_info",
  "method": "quorum",
  "params": ["info", $QUORUM_TYPE, "$HASH"]
}
EOF
  )

  if echo "$RESPONSE" | grep -q '"error":null'; then
    echo "$RESPONSE" > "$OUTPUT_FILE"
    echo "✅ Saved to $OUTPUT_FILE"
  else
    echo "⚠️  Failed to fetch quorum info for $HASH"
    echo "$RESPONSE"
  fi
done
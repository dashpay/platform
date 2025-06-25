#!/bin/bash

# Fetch proofs from Dash testnet using gRPC-web endpoints

TESTNET_NODE="35.166.18.166:1443"
OUTPUT_DIR="./fixtures"

mkdir -p "$OUTPUT_DIR"

echo "Fetching proofs from testnet node: $TESTNET_NODE"

# Function to make gRPC-web request
make_grpc_request() {
    local method=$1
    local data=$2
    local output_file=$3
    
    echo "Fetching $method..."
    
    # gRPC-web uses base64 encoded protobuf in the body
    # This is a simplified example - real implementation would need proper protobuf encoding
    curl -X POST \
        -H "Content-Type: application/grpc-web+proto" \
        -H "X-User-Agent: grpc-web-javascript/0.1" \
        --data-binary "$data" \
        "https://${TESTNET_NODE}/org.dash.platform.dapi.v0.Platform/${method}" \
        -o "$OUTPUT_DIR/${output_file}" \
        --insecure \
        2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Saved to $OUTPUT_DIR/${output_file}"
    else
        echo "✗ Failed to fetch $method"
    fi
}

# Note: These requests would need proper protobuf encoding
# For now, let's create a Node.js script instead that can properly encode the requests

echo "Creating Node.js script for proper gRPC communication..."
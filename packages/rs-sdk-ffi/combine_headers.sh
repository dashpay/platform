#!/bin/bash
# Script to add missing Core SDK type definitions to generated header

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
OUT_DIR="${OUT_DIR:-$SCRIPT_DIR/build}"
HEADER_FILE="$OUT_DIR/dash_sdk_ffi.h"

echo "Adding Core SDK type aliases to $HEADER_FILE"

# Check if header file exists
if [ ! -f "$HEADER_FILE" ]; then
    echo "Header file not found: $HEADER_FILE"
    exit 1
fi

# Add type aliases after the FFI type definitions
# Find where to insert the typedefs (after FFIDashSpvClient typedef)
if grep -q "typedef struct FFIDashSpvClient FFIDashSpvClient;" "$HEADER_FILE"; then
    # Create a temporary file
    TEMP_FILE=$(mktemp)
    
    # Process the file to add typedefs
    awk '
    /typedef struct FFIDashSpvClient FFIDashSpvClient;/ {
        print $0
        print ""
        print "/**"
        print " * Type aliases for Core SDK compatibility"
        print " */"
        print "typedef FFIClientConfig CoreSDKConfig;"
        print "typedef FFIDashSpvClient CoreSDKClient;"
        added = 1
        next
    }
    { print }
    ' "$HEADER_FILE" > "$TEMP_FILE"
    
    # Replace original file
    mv "$TEMP_FILE" "$HEADER_FILE"
    echo "Successfully added Core SDK type aliases"
else
    echo "Warning: Could not find FFIDashSpvClient typedef in header"
fi
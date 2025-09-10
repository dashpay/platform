#!/bin/bash

# Fix Double Replacement Issue
# Corrects dash_dash_wasm_sdk_bg.wasm → dash_wasm_sdk_bg.wasm

set -e

echo "🔧 Fixing double-replacement issue..."

# Fix double-dash in WASM file paths
find examples test -name "*.mjs" -o -name "*.js" -o -name "*.md" | xargs sed -i '' 's|dash_dash_wasm_sdk_bg\.wasm|dash_wasm_sdk_bg.wasm|g'

# Fix malformed import statements with extra quotes
find examples test -name "*.mjs" -o -name "*.js" | xargs sed -i '' "s|from \"../pkg/dash_wasm_sdk.js\"';|from '../pkg/dash_wasm_sdk.js';|g"

echo "✅ Double-replacement issues fixed"

# Verify the fixes
echo "🔍 Verifying fixes..."
remaining_double=$(find examples test -name "*.mjs" -o -name "*.js" | xargs grep -l "dash_dash_wasm_sdk" 2>/dev/null | wc -l)
remaining_malformed=$(find examples test -name "*.mjs" -o -name "*.js" | xargs grep -l "\.js\"';" 2>/dev/null | wc -l)

if [ "$remaining_double" -eq 0 ] && [ "$remaining_malformed" -eq 0 ]; then
    echo "✅ All double-replacement issues resolved"
else
    echo "⚠️ Some issues remain:"
    echo "  Double-dash files: $remaining_double"
    echo "  Malformed imports: $remaining_malformed"
fi
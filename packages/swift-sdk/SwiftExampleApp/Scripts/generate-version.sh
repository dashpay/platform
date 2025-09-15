#!/bin/bash

# Get the first 5 characters of the git commit hash
cd "$SRCROOT/../../.."
GIT_COMMIT=$(git rev-parse --short=5 HEAD 2>/dev/null || echo "00000")

# Create the Version.swift file
cat > "${SRCROOT:-..}/Version.swift" << EOF
// Auto-generated file - DO NOT EDIT
// Generated at build time with git commit hash

struct AppVersion {
    static let gitCommit = "$GIT_COMMIT"
}
EOF

echo "Generated Version.swift with commit: $GIT_COMMIT"
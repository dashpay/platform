#!/bin/bash

# Script to run Kotlin SDK tests

echo "================================"
echo "Kotlin SDK Test Runner"
echo "================================"

# Check if gradle/gradlew is available
if [ -x "./gradlew" ]; then
    GRADLE="./gradlew"
elif command -v gradle &> /dev/null; then
    GRADLE="gradle"
else
    echo "Error: Gradle is not installed and gradlew is not available"
    echo "Please install Gradle: https://gradle.org/install/"
    exit 1
fi

# Build the native library first
echo ""
echo "Building native library..."
(cd ../../rs-sdk-ffi && cargo build --release)

# Run tests
echo ""
echo "Running Kotlin SDK tests..."
$GRADLE test --info

# Generate test report
echo ""
echo "Test report will be available at:"
echo "build/reports/tests/test/index.html"
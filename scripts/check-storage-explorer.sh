#!/usr/bin/env bash
# Verifies the Storage Explorer UI covers all SwiftData models registered in
# DashModelContainer. Fails if a model type is added/removed in the container
# but the explorer views aren't updated to match.
#
# Usage: ./scripts/check-storage-explorer.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CONTAINER="$REPO_ROOT/packages/swift-sdk/Sources/SwiftDashSDK/Persistence/DashModelContainer.swift"
EXPLORER="$REPO_ROOT/packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/StorageExplorerView.swift"
LIST_VIEWS="$REPO_ROOT/packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/StorageModelListViews.swift"
DETAIL_VIEWS="$REPO_ROOT/packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/StorageRecordDetailViews.swift"

errors=0

# Extract model type names from DashModelContainer.modelTypes array.
# Matches lines like "PersistentFoo.self," and extracts "PersistentFoo".
# Scoped to the body of the `modelTypes` computed property so other
# `.self` references in the file (e.g. `migrationPlan:
# DashMigrationPlan.self` passed to ModelContainer) aren't mistaken
# for SwiftData models.
model_types=$(awk '/var modelTypes/{flag=1} flag{print} flag && /^    \}/{flag=0}' "$CONTAINER" \
    | grep -oE '[A-Z][A-Za-z0-9]+\.self' \
    | sed 's/\.self//' \
    | sort -u)

if [ -z "$model_types" ]; then
    echo "ERROR: Could not extract any model types from DashModelContainer.swift"
    exit 1
fi

echo "=== SwiftData Model Types in DashModelContainer ==="
echo "$model_types"
echo ""

# Check each required file exists.
for file in "$EXPLORER" "$LIST_VIEWS" "$DETAIL_VIEWS"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Missing file: $file"
        errors=$((errors + 1))
    fi
done

if [ $errors -gt 0 ]; then
    echo ""
    echo "FAILED: $errors missing file(s). Create the Storage Explorer views."
    exit 1
fi

# Check each model type is referenced in the explorer top-level view.
echo "=== Checking StorageExplorerView.swift ==="
for model in $model_types; do
    if ! grep -q "$model" "$EXPLORER"; then
        echo "  MISSING: $model not referenced in StorageExplorerView.swift"
        errors=$((errors + 1))
    else
        echo "  OK: $model"
    fi
done
echo ""

# Check each model type has a list view (struct named *StorageListView or
# containing @Query of the model type).
echo "=== Checking StorageModelListViews.swift ==="
for model in $model_types; do
    if ! grep -q "$model" "$LIST_VIEWS"; then
        echo "  MISSING: $model has no list view in StorageModelListViews.swift"
        errors=$((errors + 1))
    else
        echo "  OK: $model"
    fi
done
echo ""

# Check each model type has a detail view.
echo "=== Checking StorageRecordDetailViews.swift ==="
for model in $model_types; do
    if ! grep -q "$model" "$DETAIL_VIEWS"; then
        echo "  MISSING: $model has no detail view in StorageRecordDetailViews.swift"
        errors=$((errors + 1))
    else
        echo "  OK: $model"
    fi
done
echo ""

if [ $errors -gt 0 ]; then
    echo "FAILED: $errors model type(s) missing from Storage Explorer views."
    echo "Update the explorer views to cover all models in DashModelContainer."
    exit 1
else
    echo "PASSED: All $( echo "$model_types" | wc -l | tr -d ' ') model types covered."
    exit 0
fi

#!/bin/bash
# Display current multi-llm version information

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$WORKSPACE_ROOT"

echo "multi-llm Version Information"
echo "=============================="
echo ""

# Extract version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)
echo "Version: $VERSION"

# Git information
if git rev-parse --git-dir > /dev/null 2>&1; then
    BRANCH=$(git rev-parse --abbrev-ref HEAD)
    COMMIT=$(git rev-parse --short HEAD)
    echo "Branch: $BRANCH"
    echo "Commit: $COMMIT"

    # Check for uncommitted changes
    if ! git diff-index --quiet HEAD --; then
        echo "Status: Modified (uncommitted changes)"
    else
        echo "Status: Clean"
    fi
fi

echo ""

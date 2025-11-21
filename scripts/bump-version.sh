#!/bin/bash
# multi-llm Version Management Script
# Usage: ./scripts/bump-version.sh [major|minor|patch] [reason]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Ensure we're in the project root
cd "$PROJECT_ROOT"

# Parse arguments
BUMP_TYPE="${1:-patch}"
REASON="${2:-Version bump}"

if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo "Usage: $0 [major|minor|patch] [reason]"
    echo "Example: $0 minor 'Add streaming support'"
    exit 1
fi

# Read current version from Cargo.toml
CURRENT_VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)

if [[ -z "$CURRENT_VERSION" ]]; then
    echo "Error: Could not read version from Cargo.toml"
    exit 1
fi

echo "Current version: v$CURRENT_VERSION"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate new version
case "$BUMP_TYPE" in
    major)
        NEW_MAJOR=$((MAJOR + 1))
        NEW_MINOR=0
        NEW_PATCH=0
        ;;
    minor)
        NEW_MAJOR=$MAJOR
        NEW_MINOR=$((MINOR + 1))
        NEW_PATCH=0
        ;;
    patch)
        NEW_MAJOR=$MAJOR
        NEW_MINOR=$MINOR
        NEW_PATCH=$((PATCH + 1))
        ;;
esac

NEW_VERSION="$NEW_MAJOR.$NEW_MINOR.$NEW_PATCH"
echo "New version: v$NEW_VERSION"

# Update Cargo.toml
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm -f Cargo.toml.bak

echo "✓ Updated Cargo.toml"

# Update Cargo.lock if it exists
if [[ -f "Cargo.lock" ]]; then
    cargo update --workspace --quiet 2>/dev/null || true
    echo "✓ Updated Cargo.lock"
fi

# Get git info for commit message
BUILD_NUMBER=$(git rev-list --count HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# Commit changes if git repo
if git rev-parse --git-dir > /dev/null 2>&1; then
    echo ""
    echo "Creating git commit and tag..."

    git add Cargo.toml
    if [[ -f "Cargo.lock" ]]; then
        git add Cargo.lock
    fi

    git commit -m "Bump version to v$NEW_VERSION

$REASON

Updated:
- Cargo.toml
- Cargo.lock"

    git tag "v$NEW_VERSION"

    echo "✓ Created commit and tag v$NEW_VERSION"
    echo ""
    echo "To push changes:"
    echo "  git push"
    echo "  git push --tags"
else
    echo "Not a git repository - skipping commit and tag"
fi

echo ""
echo "✅ Version successfully bumped to v$NEW_VERSION"
echo ""
echo "Version info:"
echo "  Version: v$NEW_VERSION"
echo "  Branch: $CURRENT_BRANCH"
echo "  Commit: $GIT_COMMIT"
echo "  Build: $BUILD_NUMBER"

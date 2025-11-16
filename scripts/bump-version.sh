#!/bin/bash
# Version bump helper script
# Usage: ./scripts/bump-version.sh <new-version>

set -euo pipefail

NEW_VERSION="$1"

if [ -z "$NEW_VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.1"
    exit 1
fi

# Validate version format (basic check)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+'; then
    echo "Error: Version must follow SemVer format (e.g., 0.1.1)"
    exit 1
fi

echo "Bumping version to $NEW_VERSION..."

# Update VERSION file
echo "$NEW_VERSION" > VERSION
echo "✓ Updated VERSION file"

# Update Cargo.toml workspace version
if [ -f "Cargo.toml" ]; then
    sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
    rm -f Cargo.toml.bak
    echo "✓ Updated Cargo.toml workspace version"
fi

# Update pyproject.toml
if [ -f "sdk/python/northroot/pyproject.toml" ]; then
    sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" sdk/python/northroot/pyproject.toml
    rm -f sdk/python/northroot/pyproject.toml.bak
    echo "✓ Updated pyproject.toml version"
fi

echo ""
echo "✓ Version bumped to $NEW_VERSION"
echo ""
echo "Review changes:"
echo "  git diff VERSION Cargo.toml sdk/python/northroot/pyproject.toml"
echo ""
echo "Commit with:"
echo "  git add VERSION Cargo.toml sdk/python/northroot/pyproject.toml"
echo "  git commit -m \"chore: bump version to $NEW_VERSION\""


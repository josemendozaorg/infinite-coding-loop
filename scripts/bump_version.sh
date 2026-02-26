#!/bin/bash
set -e

# Function to display usage
usage() {
    echo "Usage: $0 {major|minor|patch|release}"
    echo "Examples:"
    echo "  $0 patch       # Bump patch version, generate changelog, commit, and tag"
    echo "  $0 release     # Auto-detect version bump from commits, generate changelog, commit, and tag"
    exit 1
}

if [ -z "$1" ]; then
    usage
fi

BUMP_TYPE=$1

if [ "$BUMP_TYPE" = "release" ]; then
    # Auto-detect based on conventional commits
    npm run release
elif [[ "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    npm run release -- --release-as "$BUMP_TYPE"
else
    echo "Error: Invalid bump type '$BUMP_TYPE'"
    usage
fi

#!/bin/bash
set -e

# Function to display usage
usage() {
    echo "Usage: $0 {major|minor|patch}"
    exit 1
}

# Check argument
if [ -z "$1" ]; then
    usage
fi

BUMP_TYPE=$1
CORE_CARGO_TOML="crates/dass-engine/Cargo.toml"
CLI_CARGO_TOML="crates/dass-cli/Cargo.toml"
TUI_CARGO_TOML="crates/dass-tui/Cargo.toml"
TOOLS_CARGO_TOML="crates/ontology-tools/Cargo.toml"
E2E_CARGO_TOML="crates/dass-e2e/Cargo.toml"

# Check if files exist
if [ ! -f "$CORE_CARGO_TOML" ]; then
    echo "Error: $CORE_CARGO_TOML not found."
    exit 1
fi

# Get current version from crates/dass-engine/Cargo.toml
# Assumes 'version = "x.y.z"' is in the first few lines
CURRENT_VERSION=$(grep '^version = ' "$CORE_CARGO_TOML" | head -n 1 | cut -d '"' -f 2)

echo "Current version: $CURRENT_VERSION"

# Calculate new version using python for reliability
NEW_VERSION=$(python3 -c "
import sys
v = '$CURRENT_VERSION'.split('.')
major = int(v[0])
minor = int(v[1])
patch = int(v[2])

bump_type = '$BUMP_TYPE'

if bump_type == 'major':
    major += 1
    minor = 0
    patch = 0
elif bump_type == 'minor':
    minor += 1
    patch = 0
elif bump_type == 'patch':
    patch += 1
else:
    print('Error: Invalid bump type', file=sys.stderr)
    sys.exit(1)

print(f'{major}.{minor}.{patch}')
")

echo "New version: $NEW_VERSION"

# Update Cargo.toml files
# We use a temporary file to ensure we don't break things if sed fails
# Mac OS sed handles -i differently, but we are on Linux.
# Match exactly 'version = "..."' at the start of the line to avoid dependencies
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$CORE_CARGO_TOML"
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$CLI_CARGO_TOML"
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$TUI_CARGO_TOML"
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$TOOLS_CARGO_TOML"
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$E2E_CARGO_TOML"

echo "Updated all crates to version $NEW_VERSION"

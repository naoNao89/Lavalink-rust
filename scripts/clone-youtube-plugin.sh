#!/bin/bash

# Script to properly clone the YouTube source plugin from the external repository
# This fixes the "Remote branch plugin not found" error

set -e

REPO_URL="https://github.com/lavalink-devs/youtube-source.git"
TARGET_DIR="youtube-source-external"
BRANCH="main"  # Use main branch instead of non-existent plugin branch

echo "Cloning YouTube source plugin from external repository..."

# Remove existing directory if it exists
if [ -d "$TARGET_DIR" ]; then
    echo "Removing existing $TARGET_DIR directory..."
    rm -rf "$TARGET_DIR"
fi

# Clone the repository from the main branch
echo "Cloning from $REPO_URL (branch: $BRANCH)..."
git clone --branch "$BRANCH" --single-branch "$REPO_URL" "$TARGET_DIR"

if [ $? -eq 0 ]; then
    echo "✅ Successfully cloned YouTube source plugin to $TARGET_DIR"
    echo "Available projects in the repository:"
    ls -la "$TARGET_DIR"
    
    # Show the plugin directory specifically
    if [ -d "$TARGET_DIR/plugin" ]; then
        echo "✅ Plugin directory found at $TARGET_DIR/plugin"
    else
        echo "⚠️  Plugin directory not found. Available directories:"
        find "$TARGET_DIR" -type d -maxdepth 1 | grep -v "\.git"
    fi
else
    echo "❌ Failed to clone repository"
    exit 1
fi

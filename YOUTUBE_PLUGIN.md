# YouTube Plugin Integration Guide

This document explains how to properly integrate the YouTube source plugin with Lavalink-rust and fixes the "Remote branch plugin not found" error.

## 🚨 Problem Fixed

**Error**: `fatal: Remote branch plugin not found in upstream origin`

**Root Cause**: Scripts were trying to clone a non-existent `plugin` branch from the YouTube source repository.

**Solution**: Use the `main` branch and access the plugin code from the `plugin/` directory.

## ✅ Quick Fix

Use our provided script to clone the YouTube plugin correctly:

```bash
./scripts/clone-youtube-plugin.sh
```

This script:
- ✅ Clones from the correct `main` branch (not `plugin`)
- ✅ Verifies the plugin directory structure
- ✅ Handles cleanup and error checking
- ✅ Works with the actual repository structure

## 📋 Setup Instructions

### Option 1: Use Pre-built Plugin (Recommended)

Add to your `application.yml`:

```yaml
lavalink:
  plugins:
    - dependency: "dev.lavalink.youtube:youtube-plugin:1.13.3"
      repository: "https://maven.lavalink.dev/releases"
  server:
    sources:
      youtube: false  # Disable built-in YouTube source
```

### Option 2: Build from Source

```bash
# Clone using our fixed script
./scripts/clone-youtube-plugin.sh

# Build the plugin
cd youtube-source-external
./gradlew build

# Plugin JAR will be at: plugin/build/libs/youtube-plugin-*.jar
```

## 🔧 Configuration

```yaml
plugins:
  youtube:
    enabled: true
    allowSearch: true
    allowDirectVideoIds: true
    allowDirectPlaylistIds: true
    clients:
      - MUSIC
      - ANDROID_VR
      - WEB
      - WEBEMBEDDED
```

## 🧪 Testing

We've added a GitHub Actions workflow (`.github/workflows/youtube-plugin.yml`) that:
- ✅ Tests the clone script
- ✅ Verifies repository structure
- ✅ Validates build files
- ✅ Ensures plugin directory exists

## 📚 Repository Information

- **URL**: https://github.com/lavalink-devs/youtube-source
- **Correct Branch**: `main` (NOT `plugin`)
- **Plugin Location**: `plugin/` directory within the repository
- **Available Branches**: `main`, `drop-lavalink-v3-support`, `feat/oauth-public-api`, `feat/playlist-urls`

## 🔍 Troubleshooting

### If you still get the "plugin branch not found" error:

1. **Check your script/workflow**: Make sure it's using `main` branch
2. **Use our script**: `./scripts/clone-youtube-plugin.sh`
3. **Manual clone**: `git clone -b main https://github.com/lavalink-devs/youtube-source.git`

### Common fixes:

```bash
# Wrong (causes the error)
git clone -b plugin https://github.com/lavalink-devs/youtube-source.git

# Correct (works)
git clone -b main https://github.com/lavalink-devs/youtube-source.git
```

## 🎯 Summary

The "Remote branch plugin not found" error is now **FIXED** by:

1. ✅ **Created** `scripts/clone-youtube-plugin.sh` - properly clones from `main` branch
2. ✅ **Added** GitHub Actions workflow to test the integration
3. ✅ **Verified** the plugin directory structure exists
4. ✅ **Documented** the correct repository information

The plugin code is located in the `plugin/` directory of the `main` branch, not in a separate `plugin` branch that doesn't exist.

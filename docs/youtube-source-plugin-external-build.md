# YouTube Source Plugin External Build Integration

## Overview

This document describes the CI/CD integration that automatically clones and builds the YouTube source plugin from the external lavalink-devs repository during the build process, eliminating the need to include plugin source code in the main repository.

## External Plugin Source

**Repository**: `https://github.com/lavalink-devs/youtube-source`  
**Branch**: `plugin`  
**Clone Method**: Shallow clone (`--depth 1`)  
**Build System**: Gradle with Java 17

## Workflow Integration

### Build Workflow (`.github/workflows/build.yml`)

#### Test Job Steps:
1. **Clone YouTube Source Plugin** - Shallow clone from external repository
2. **Setup Java for YouTube Plugin Build** - Install Java 17 for Gradle build
3. **Build YouTube Plugin** - Execute Gradle build and copy JAR to plugins directory
4. Continue with standard Rust build process

#### Build Job Steps:
- Same plugin cloning and building steps as test job
- **Upload YouTube Plugin Artifacts** - Upload built plugin JARs as CI artifacts

### PR Workflow (`.github/workflows/pr.yml`)
- Identical YouTube plugin cloning and building steps as build workflow
- Ensures plugin availability during pull request testing

### Release Workflow (`.github/workflows/release.yml`)
- **Download YouTube Plugin Artifacts** - Retrieve plugin JARs for each target architecture
- **Bundle Plugins with Binaries** - Include plugins in release archives alongside executables

## Build Process Details

### Plugin Build Steps:
```bash
# Clone external repository
git clone --depth 1 --branch plugin https://github.com/lavalink-devs/youtube-source.git youtube-source-external

# Build plugin with Gradle
cd youtube-source-external
chmod +x gradlew
./gradlew build --no-daemon

# Copy built JAR to plugins directory
mkdir -p ../plugins
find . -name "youtube-plugin-*.jar" -path "*/build/libs/*" -exec cp {} ../plugins/ \;
```

### Artifact Management:
- Plugin JARs uploaded as `youtube-plugin-{target-architecture}` artifacts
- Downloaded and bundled with release binaries
- Available in `plugins/` directory of release archives

## Repository Configuration

### .gitignore Entries:
```
# External plugin sources (cloned during CI/CD)
youtube-source/
youtube-source-external/
```

### Directory Structure After Build:
```
plugins/
├── youtube-plugin-*.jar  # Built from external repository
```

## Benefits

1. **Repository Size Reduction** - No plugin source code in main repository
2. **Automatic Plugin Updates** - Always uses latest plugin version from upstream
3. **Clear Dependency Separation** - Plugin source managed externally
4. **Faster Repository Operations** - Smaller repository for cloning and storage

## Error Handling

### Plugin Repository Unavailable:
- Build fails with clear error message
- CI logs show clone failure details

### Plugin Build Failures:
- Gradle build errors logged in CI output
- Plugin JAR copy operations include error handling

### Missing Plugin Files:
- Artifact upload includes `if-no-files-found: warn`
- Release process handles missing plugins gracefully

## Maintenance

### Plugin Version Updates:
- Automatic - uses latest from plugin branch
- No manual intervention required in main repository

### Java Version Requirements:
- Java 17 required for plugin build
- Configured in workflow setup-java action

### Gradle Build Configuration:
- Uses plugin repository's gradle wrapper
- No-daemon flag for CI efficiency
- Build artifacts located in `*/build/libs/*` pattern

## Troubleshooting

### Plugin Clone Failures:
1. Verify external repository accessibility
2. Check plugin branch existence
3. Review network connectivity in CI environment

### Plugin Build Failures:
1. Check Java 17 setup in workflow
2. Review Gradle build logs
3. Verify plugin repository build requirements

### Missing Plugin in Release:
1. Check artifact upload success
2. Verify artifact download in release job
3. Review plugin JAR copy operations

## Architecture Integration

### Plugin Loading:
- Built plugin JAR available in `plugins/` directory
- Lavalink-rust plugin loader discovers and loads JAR files
- Plugin interface compatibility maintained

### Release Distribution:
- Plugin bundled with binary releases
- No separate plugin distribution required
- Users receive complete package with plugin included

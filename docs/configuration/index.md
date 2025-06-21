---
description: How to configure Lavalink Rust
---

# Configuration

Lavalink Rust uses the same configuration format as Java Lavalink, ensuring seamless migration while providing Rust-specific optimizations and features.

!!! rust "Configuration Compatibility"
    Your existing `application.yml` configuration file from Java Lavalink works without modification in Rust Lavalink. This ensures zero-downtime migration and easy switching between implementations.

## Configuration Methods

There are 3 main ways to configure Lavalink Rust:

### 1. Configuration File (Recommended)

The easiest and most common way to configure Lavalink Rust. Create a file called `application.yml` in the same directory as your Lavalink Rust binary.

```yaml
server:
  port: 2333
  address: 0.0.0.0

lavalink:
  server:
    password: "youshallnotpass"
    sources:
      youtube: true
      bandcamp: true
      soundcloud: true
      twitch: true
      vimeo: true
      http: true
      local: true
      fallback: true  # Rust-specific: enables Spotify/Apple Music/Deezer fallback
```

!!! rust "Rust-Specific Features"
    - **Fallback Source**: Enable automatic conversion of Spotify/Apple Music/Deezer URLs to YouTube searches
    - **Enhanced Performance**: Rust-specific optimizations are automatically applied
    - **Memory Safety**: No JVM-specific configuration needed

### 2. Environment Variables

Configure Lavalink Rust using environment variables. This is particularly useful for containerized deployments.

!!! performance "Performance Note"
    Environment variable parsing in Rust is faster and more memory-efficient than the Java implementation.

<details markdown="1">
<summary>Rust Lavalink Environment Variables</summary>

```bash
# Server Configuration
SERVER_PORT=2333
SERVER_ADDRESS=0.0.0.0

# Lavalink Authentication
LAVALINK_SERVER_PASSWORD=youshallnotpass

# Audio Sources
LAVALINK_SERVER_SOURCES_YOUTUBE=true
LAVALINK_SERVER_SOURCES_BANDCAMP=true
LAVALINK_SERVER_SOURCES_SOUNDCLOUD=true
LAVALINK_SERVER_SOURCES_TWITCH=true
LAVALINK_SERVER_SOURCES_VIMEO=true
LAVALINK_SERVER_SOURCES_HTTP=true
LAVALINK_SERVER_SOURCES_LOCAL=true
LAVALINK_SERVER_SOURCES_FALLBACK=true  # Rust-specific

# Audio Filters
LAVALINK_SERVER_FILTERS_VOLUME=true
LAVALINK_SERVER_FILTERS_EQUALIZER=true
LAVALINK_SERVER_FILTERS_KARAOKE=true
LAVALINK_SERVER_FILTERS_TIMESCALE=true
LAVALINK_SERVER_FILTERS_TREMOLO=true
LAVALINK_SERVER_FILTERS_VIBRATO=true
LAVALINK_SERVER_FILTERS_DISTORTION=true
LAVALINK_SERVER_FILTERS_ROTATION=true
LAVALINK_SERVER_FILTERS_CHANNEL_MIX=true
LAVALINK_SERVER_FILTERS_LOW_PASS=true

# Audio Engine (Rust-specific optimizations)
LAVALINK_SERVER_BUFFER_DURATION_MS=400
LAVALINK_SERVER_FRAME_BUFFER_DURATION_MS=5000
LAVALINK_SERVER_OPUS_ENCODING_QUALITY=10
LAVALINK_SERVER_TRACK_STUCK_THRESHOLD_MS=10000

# Player Configuration
LAVALINK_SERVER_PLAYER_UPDATE_INTERVAL=5
LAVALINK_SERVER_YOUTUBE_PLAYLIST_LOAD_LIMIT=6
LAVALINK_SERVER_YOUTUBE_SEARCH_ENABLED=true
LAVALINK_SERVER_SOUNDCLOUD_SEARCH_ENABLED=true

# Rate Limiting
LAVALINK_SERVER_RATELIMIT_IP_BLOCKS=
LAVALINK_SERVER_RATELIMIT_EXCLUDE_IPS=
LAVALINK_SERVER_RATELIMIT_STRATEGY=RotateOnBan
LAVALINK_SERVER_RATELIMIT_RETRY_LIMIT=-1

# HTTP Configuration
LAVALINK_SERVER_HTTP_CONFIG_PROXY_HOST=
LAVALINK_SERVER_HTTP_CONFIG_PROXY_PORT=
LAVALINK_SERVER_HTTP_CONFIG_PROXY_USER=
LAVALINK_SERVER_HTTP_CONFIG_PROXY_PASSWORD=

# Metrics (Rust-specific implementation)
METRICS_PROMETHEUS_ENABLED=false
METRICS_PROMETHEUS_ENDPOINT=/metrics

# Logging (Rust-specific)
RUST_LOG=info
LAVALINK_LOG_LEVEL=info
LOGGING_FILE_PATH=./logs/lavalink.log

# Plugin System (Rust-specific)
LAVALINK_PLUGINS_DIR=./plugins
LAVALINK_PLUGINS_ENABLED=true
```

</details>

### 3. Command Line Arguments

Lavalink Rust supports command line arguments for quick configuration overrides:

```bash
# Basic usage
./lavalink-rust

# Custom configuration file
./lavalink-rust --config /path/to/custom.yml

# Override specific settings
./lavalink-rust --port 2334 --password mypassword

# Enable debug logging
./lavalink-rust --log-level debug

# Show help
./lavalink-rust --help
```

## Configuration Sections

### Server Configuration

Basic server settings for network binding and HTTP configuration.

```yaml
server:
  port: 2333                    # Port to bind to
  address: 0.0.0.0             # Address to bind to (0.0.0.0 for all interfaces)
  http2:
    enabled: false             # HTTP/2 support (experimental in Rust)
```

!!! rust "Rust Advantages"
    - **Faster Startup**: Native binary starts in ~2 seconds vs 10-15 seconds for Java
    - **Lower Memory**: ~100MB baseline vs ~200MB for Java
    - **Better Concurrency**: Tokio async runtime handles more connections efficiently

### Lavalink Core Configuration

Core Lavalink settings including authentication and audio sources.

```yaml
lavalink:
  server:
    password: "youshallnotpass"  # Authentication password
    sources:
      youtube: true              # YouTube support via yt-dlp
      bandcamp: true             # Bandcamp support
      soundcloud: true           # SoundCloud support
      twitch: true               # Twitch streams and VODs
      vimeo: true                # Vimeo support
      http: true                 # Direct HTTP audio streams
      local: true                # Local file system access
      fallback: true             # Rust-specific: Spotify/Apple Music/Deezer fallback
    
    filters:                     # Audio filter support
      volume: true
      equalizer: true
      karaoke: true
      timescale: true
      tremolo: true
      vibrato: true
      distortion: true
      rotation: true
      channelMix: true
      lowPass: true
    
    bufferDurationMs: 400        # Audio buffer duration
    frameBufferDurationMs: 5000  # Frame buffer duration
    opusEncodingQuality: 10      # Opus encoding quality (0-10)
    trackStuckThresholdMs: 10000 # Track stuck detection threshold
    
    playerUpdateInterval: 5      # Player update interval in seconds
    youtubePlaylistLoadLimit: 6  # YouTube playlist load limit
    youtubeSearchEnabled: true   # Enable YouTube search
    soundcloudSearchEnabled: true # Enable SoundCloud search
```

### Rust-Specific Configuration

Configuration options unique to the Rust implementation.

```yaml
rust:
  # Audio Engine Configuration
  audio:
    sample_rate: 48000           # Audio sample rate
    channels: 2                  # Audio channels (1=mono, 2=stereo)
    bit_depth: 16               # Audio bit depth
    
  # Memory Management
  memory:
    max_track_cache: 1000       # Maximum cached tracks
    cleanup_interval: 300       # Cache cleanup interval (seconds)
    
  # Performance Tuning
  performance:
    worker_threads: 0           # Tokio worker threads (0 = auto)
    max_blocking_threads: 512   # Max blocking threads
    thread_stack_size: 2097152  # Thread stack size in bytes
    
  # Plugin System
  plugins:
    enabled: true               # Enable plugin system
    directory: "./plugins"      # Plugin directory
    hot_reload: true           # Enable hot reloading
    
  # Fallback System Configuration
  fallback:
    enabled: true               # Enable fallback system
    youtube_search_prefix: "ytsearch:" # Search prefix for fallback
    max_search_results: 5       # Maximum search results to consider
    cache_duration: 3600        # Cache duration for fallback results (seconds)
```

### Logging Configuration

Rust Lavalink uses the `tracing` crate for structured logging.

```yaml
logging:
  level:
    root: INFO                  # Root log level
    lavalink: INFO             # Lavalink-specific log level
    tokio: WARN                # Tokio runtime log level
    hyper: WARN                # HTTP server log level
    
  file:
    enabled: true              # Enable file logging
    path: "./logs/lavalink.log" # Log file path
    max_size: "10MB"           # Maximum log file size
    max_files: 5               # Maximum number of log files
    
  console:
    enabled: true              # Enable console logging
    format: "pretty"           # Log format: "pretty", "json", "compact"
    
  filters:
    - "hyper::proto::h1::io=off"     # Disable noisy HTTP logs
    - "tokio_util::codec::framed=off" # Disable codec logs
```

### Metrics Configuration

Prometheus metrics support for monitoring.

```yaml
metrics:
  prometheus:
    enabled: false             # Enable Prometheus metrics
    endpoint: "/metrics"       # Metrics endpoint path
    
  # Rust-specific metrics
  rust:
    memory_stats: true         # Include memory statistics
    tokio_stats: true          # Include Tokio runtime statistics
    audio_stats: true          # Include audio processing statistics
```

## Migration from Java Lavalink

### Compatible Settings

These settings work identically in both Java and Rust Lavalink:

- ✅ `server.port` and `server.address`
- ✅ `lavalink.server.password`
- ✅ All `lavalink.server.sources.*` settings
- ✅ All `lavalink.server.filters.*` settings
- ✅ Player and buffer configuration
- ✅ Rate limiting configuration
- ✅ HTTP proxy configuration

### Rust-Specific Additions

New configuration options available only in Rust Lavalink:

- 🆕 `lavalink.server.sources.fallback` - Spotify/Apple Music/Deezer fallback
- 🆕 `rust.*` section - Rust-specific optimizations
- 🆕 Enhanced logging with structured output
- 🆕 Plugin hot-reloading support
- 🆕 Advanced memory management options

### Removed/Changed Settings

Settings that don't apply to Rust Lavalink:

- ❌ JVM-specific settings (heap size, GC configuration)
- ❌ `lavalink.server.gc-warnings` (no garbage collection in Rust)
- ❌ Java-specific plugin configuration
- 🔄 Logging configuration uses Rust tracing instead of Logback

## Performance Tuning

### Memory Optimization

```yaml
rust:
  memory:
    max_track_cache: 500        # Reduce for lower memory usage
    cleanup_interval: 180       # More frequent cleanup
    
lavalink:
  server:
    bufferDurationMs: 200       # Smaller buffer for lower latency
    frameBufferDurationMs: 2000 # Reduce frame buffer
```

### High-Performance Configuration

```yaml
rust:
  performance:
    worker_threads: 8           # Match CPU cores
    max_blocking_threads: 1024  # Increase for high load
    
lavalink:
  server:
    playerUpdateInterval: 1     # More frequent updates
    bufferDurationMs: 800       # Larger buffer for stability
```

For more detailed configuration options, see:

- [Audio Sources Configuration](sources.md)
- [Audio Filters Configuration](filters.md)
- [Performance Tuning Guide](performance.md)
- [Plugin Configuration](../plugins/development.md)

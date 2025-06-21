---
description: Lavalink Rust frequently asked questions.
---

# FAQ

## General Questions

### What is Lavalink Rust?

Lavalink Rust is a high-performance, memory-safe implementation of Lavalink written in Rust. It provides a drop-in replacement for the original Java Lavalink server while leveraging Rust's performance and safety benefits.

### What is Lavalink used for?

Lavalink is used to stream music to Discord voice servers. It acts as an audio node that Discord bots connect to for playing music, handling the heavy lifting of audio processing and streaming while your bot focuses on Discord interactions.

### How does Lavalink Rust compare to Java Lavalink?

| Feature | Java Lavalink | Rust Lavalink |
|---------|---------------|---------------|
| **Memory Usage** | 1-6GB | 256-512MB |
| **Startup Time** | 10-15 seconds | 2-5 seconds |
| **CPU Usage** | Variable (GC spikes) | Consistent |
| **Dependencies** | JRE 17+ required | Native binary |
| **API Compatibility** | Full | 95%+ compatible |
| **Plugin System** | JAR-based | Dynamic libraries |

## Installation and Setup

### How do I install Lavalink Rust?

There are several installation methods:

1. **Download pre-built binary** (recommended):
   ```bash
   wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-linux-x64
   chmod +x lavalink-rust-linux-x64
   ```

2. **Build from source**:
   ```bash
   git clone https://github.com/lavalink-devs/lavalink-rust.git
   cd lavalink-rust
   cargo build --release
   ```

3. **Install via Cargo**:
   ```bash
   cargo install lavalink-rust
   ```

See the [Binary Installation Guide](binary.md) for detailed instructions.

### Do I need Java to run Lavalink Rust?

No! Unlike Java Lavalink, Rust Lavalink is a native binary that doesn't require a Java Runtime Environment (JRE). However, you do need `yt-dlp` for audio source support.

### How do I install yt-dlp?

```bash
# Using pip (recommended)
pip3 install yt-dlp

# Using package managers
sudo apt install yt-dlp  # Ubuntu/Debian
brew install yt-dlp      # macOS

# Verify installation
yt-dlp --version
```

### Can I use my existing Java Lavalink configuration?

Yes! Your existing `application.yml` configuration file should work without modification. Rust Lavalink maintains configuration compatibility with Java Lavalink.

### How do I update Lavalink Rust?

**Binary installation:**
```bash
# Download new binary
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-linux-x64
chmod +x lavalink-rust-linux-x64

# Replace old binary
sudo systemctl stop lavalink-rust
sudo cp lavalink-rust-linux-x64 /opt/lavalink-rust/bin/lavalink-rust
sudo systemctl start lavalink-rust
```

**Docker:**
```bash
docker compose pull
docker compose up -d
```

## Audio Sources and Compatibility

### What audio sources are supported?

✅ **Fully Supported:**
- YouTube (search and direct URLs)
- SoundCloud (search and direct URLs)
- Bandcamp
- Vimeo
- Twitch (streams, VODs, clips)
- HTTP direct audio URLs
- Local files (file:// URLs)

❌ **Not Supported:**
- Spotify (use YouTube fallback)
- Apple Music (use YouTube fallback)
- Deezer (use YouTube fallback)
- Niconico

### How does the Spotify fallback work?

When you provide a Spotify URL, Lavalink Rust automatically:
1. Extracts track metadata (title, artist)
2. Searches YouTube for the same track
3. Returns YouTube results instead

```bash
# Input: Spotify URL
https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh

# Output: YouTube search results for "Never Gonna Give You Up Rick Astley"
```

### Why aren't Spotify/Apple Music/Deezer directly supported?

These services require official API access and licensing agreements that aren't available for open-source projects. The YouTube fallback provides a legal alternative that works well in practice.

### Can I disable the YouTube fallback?

Yes, you can configure fallback behavior in your `application.yml`:

```yaml
lavalink:
  server:
    fallback:
      enabled: true
      youtube_search: true
      max_results: 5
```

## Performance and Resources

### How much memory does Lavalink Rust use?

Typically 256-512MB, which is 50-75% less than Java Lavalink. Memory usage scales with:
- Number of concurrent players
- Audio quality settings
- Cache size

### How much CPU does it use?

CPU usage is generally lower and more consistent than Java Lavalink due to:
- No garbage collection pauses
- Efficient async runtime (Tokio)
- Optimized audio processing

### Can I run multiple instances?

Yes! You can run multiple Lavalink Rust instances for load balancing:

```yaml
# Instance 1
server:
  port: 2333

# Instance 2  
server:
  port: 2334
```

### What are the minimum system requirements?

- **CPU**: 1 core (2+ recommended for production)
- **Memory**: 256MB (512MB+ recommended)
- **Storage**: 50MB for binary + logs
- **Network**: Stable internet connection

## API and Client Compatibility

### Are existing Lavalink clients compatible?

Yes! Lavalink Rust maintains 95%+ API compatibility with Java Lavalink. Most clients work without modification.

### What API endpoints are different?

**Missing endpoints:**
- `POST /v4/decodetracks` (planned for future release)

**Enhanced endpoints:**
- All endpoints have faster response times
- Better error messages and stack traces
- Additional metrics and monitoring data

### How do I connect my Discord bot?

Use any existing Lavalink client library. Connection details remain the same:

```javascript
// Example with discord.js and erela.js
const { Manager } = require("erela.js");

const manager = new Manager({
    nodes: [{
        host: "localhost",
        port: 2333,
        password: "youshallnotpass",
    }],
    // ... other options
});
```

## Docker and Deployment

### How do I run Lavalink Rust with Docker?

```yaml
# docker-compose.yml
services:
  lavalink-rust:
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    ports:
      - "2333:2333"
    environment:
      - LAVALINK_SERVER_PASSWORD=youshallnotpass
    volumes:
      - ./application.yml:/app/application.yml:ro
```

See the [Docker Guide](docker.md) for complete instructions.

### What's the difference between Docker image variants?

| Variant | Size | Base | Best For |
|---------|------|------|----------|
| `latest` | ~150MB | Debian | General use, debugging |
| `alpine` | ~80MB | Alpine | Production, size-conscious |
| `distroless` | ~50MB | Distroless | High security, minimal attack surface |

### How do I set up a systemd service?

```bash
# Automated setup
sudo ./deployment/scripts/deploy.sh

# Manual setup
sudo cp lavalink-rust.service /etc/systemd/system/
sudo systemctl enable lavalink-rust
sudo systemctl start lavalink-rust
```

See the [Systemd Guide](systemd.md) for detailed instructions.

## Troubleshooting

### Lavalink won't start

**Check the basics:**
```bash
# Verify binary is executable
chmod +x lavalink-rust

# Check dependencies
yt-dlp --version

# Test configuration
./lavalink-rust --config application.yml --validate
```

**Common issues:**
- Port 2333 already in use
- Missing yt-dlp dependency
- Invalid configuration file
- Insufficient permissions

### Audio playback issues

**No audio playing:**
1. Check if tracks are loading: `curl http://localhost:2333/v4/loadtracks?identifier=ytsearch:test`
2. Verify yt-dlp is working: `yt-dlp --version`
3. Check Discord bot permissions
4. Verify WebSocket connection

**Poor audio quality:**
```yaml
lavalink:
  server:
    filters:
      volume: true
      equalizer: true
    buffer_duration_ms: 400
    frame_buffer_duration_ms: 5000
```

### High memory usage

```bash
# Check actual usage
ps aux | grep lavalink-rust

# Monitor over time
top -p $(pgrep lavalink-rust)
```

**Optimization tips:**
- Reduce buffer sizes
- Limit concurrent players
- Disable unused filters
- Use appropriate Docker memory limits

### Connection issues

**WebSocket connection fails:**
1. Check firewall settings
2. Verify password matches
3. Test with curl: `curl http://localhost:2333/v4/info`
4. Check client library configuration

**Timeout errors:**
```yaml
server:
  http:
    timeout: 30s
    keep_alive: 60s
```

## Migration from Java Lavalink

### How do I migrate from Java Lavalink?

1. **Backup your configuration:**
   ```bash
   cp application.yml application.yml.backup
   ```

2. **Install Rust Lavalink:**
   ```bash
   # Download binary or use deployment script
   sudo ./deployment/scripts/deploy.sh
   ```

3. **Stop Java service and start Rust service:**
   ```bash
   sudo systemctl stop lavalink
   sudo systemctl start lavalink-rust
   ```

4. **Test functionality:**
   ```bash
   curl http://localhost:2333/v4/info
   ```

### Will my plugins work?

Java plugins are **not compatible** with Rust Lavalink due to different plugin architectures. However:

- Most plugin functionality is built into Rust Lavalink
- A new plugin system is in development
- Popular plugins are being ported

### Can I run both versions simultaneously?

Yes, for testing purposes:

```yaml
# Java Lavalink
server:
  port: 2333

# Rust Lavalink  
server:
  port: 2334
```

This allows gradual migration and A/B testing.

## Getting Help

### How do I report a bug?

1. **Check existing issues:** [GitHub Issues](https://github.com/lavalink-devs/lavalink-rust/issues)
2. **Gather information:**
   - Lavalink Rust version
   - Operating system
   - Configuration file (remove sensitive data)
   - Error logs
3. **Create detailed issue** with reproduction steps

### How do I get support?

- **GitHub Discussions:** [Q&A Section](https://github.com/lavalink-devs/lavalink-rust/discussions)
- **Discord:** [Lavalink Support Server](https://discord.gg/lavalink)
- **Documentation:** Check [troubleshooting guide](troubleshooting.md)

### How can I contribute?

- **Report bugs** and suggest features
- **Improve documentation**
- **Submit pull requests**
- **Help other users** in discussions

See the [Contributing Guide](https://github.com/lavalink-devs/lavalink-rust/blob/main/CONTRIBUTING.md) for details.

## Advanced Topics

### How do I enable metrics?

```yaml
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    port: 9090
```

Access metrics at `http://localhost:9090/metrics`

### How do I configure logging?

```yaml
logging:
  level:
    root: INFO
    lavalink: DEBUG
  file:
    enabled: true
    path: "./logs/lavalink.log"
    max_size: "100MB"
    max_files: 10
```

### How do I set up SSL/TLS?

```yaml
server:
  ssl:
    enabled: true
    key_store: "keystore.p12"
    key_store_password: "password"
    key_store_type: "PKCS12"
```

### How do I configure load balancing?

Use multiple Lavalink instances with a load balancer:

```yaml
# nginx.conf
upstream lavalink {
    server 127.0.0.1:2333;
    server 127.0.0.1:2334;
    server 127.0.0.1:2335;
}
```

For more advanced topics, see the [Advanced Configuration Guide](../advanced/architecture.md).

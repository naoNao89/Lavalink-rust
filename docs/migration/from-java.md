---
description: Complete guide for migrating from Java Lavalink to Rust Lavalink
---

# Migration from Java Lavalink

This guide provides step-by-step instructions for migrating from Java Lavalink to Rust Lavalink, ensuring a smooth transition with minimal downtime.

## Migration Overview

### Why Migrate?

| Benefit | Java Lavalink | Rust Lavalink | Improvement |
|---------|---------------|---------------|-------------|
| **Memory Usage** | 1-6 GB | 256-512 MB | 50-75% reduction |
| **Startup Time** | 10-15 seconds | 2-5 seconds | 75% faster |
| **CPU Efficiency** | Variable (GC spikes) | Consistent | Predictable performance |
| **Resource Usage** | High overhead | Minimal overhead | Better resource utilization |
| **Stability** | JVM dependent | Native binary | Reduced dependencies |

### Compatibility Matrix

| Feature | Java Lavalink | Rust Lavalink | Migration Required |
|---------|---------------|---------------|-------------------|
| **REST API** | ✅ Full | ✅ 95% Compatible | ❌ No |
| **WebSocket Protocol** | ✅ Full | ✅ Full | ❌ No |
| **Configuration** | ✅ application.yml | ✅ Same format | ❌ No |
| **Audio Sources** | ✅ 8 sources | ✅ 7 sources* | ⚠️ Partial |
| **Plugins** | ✅ JAR-based | ✅ Native libraries | ✅ Yes |
| **Client Libraries** | ✅ All supported | ✅ All supported | ❌ No |

*Spotify/Apple Music/Deezer use intelligent fallback to YouTube

## Pre-Migration Assessment

### 1. Inventory Current Setup

**Document your current configuration:**
```bash
# Backup current configuration
cp application.yml application.yml.backup

# Document Java version
java -version

# Check current memory usage
ps aux | grep java | grep -i lavalink

# List installed plugins
ls -la plugins/
```

**Identify dependencies:**
- Java Runtime Environment version
- Custom plugins in use
- Audio sources being used
- Client libraries and versions
- Monitoring and logging setup

### 2. Compatibility Check

**Audio Sources Assessment:**
```yaml
# Check your current sources configuration
lavalink:
  server:
    sources:
      youtube: true      # ✅ Fully supported
      bandcamp: true     # ✅ Fully supported  
      soundcloud: true   # ✅ Fully supported
      twitch: true       # ✅ Fully supported
      vimeo: true        # ✅ Fully supported
      http: true         # ✅ Fully supported
      local: true        # ✅ Fully supported
      nico: false        # ❌ Not supported yet
```

**Plugin Assessment:**
```bash
# List current plugins
find plugins/ -name "*.jar" -exec basename {} \;

# Common plugins and their Rust equivalents:
# - LavaSrc (Spotify/Apple Music) → Built-in fallback system
# - LavaSearch → Built-in search functionality  
# - Custom plugins → Need rewriting for Rust
```

### 3. Performance Baseline

**Measure current performance:**
```bash
# Memory usage
ps -o pid,vsz,rss,comm -p $(pgrep java)

# Startup time
time systemctl restart lavalink

# Response time
time curl http://localhost:2333/v4/info

# Connection count
netstat -an | grep :2333 | wc -l
```

## Migration Strategies

### Strategy 1: Direct Replacement (Recommended)

**Best for:**
- Standard setups without custom plugins
- Minimal Spotify/Apple Music usage
- Desire for immediate performance benefits

**Steps:**
1. Install Rust Lavalink alongside Java version
2. Test functionality with existing configuration
3. Switch traffic to Rust version
4. Monitor and validate
5. Decommission Java version

### Strategy 2: Gradual Migration

**Best for:**
- Complex setups with multiple instances
- Heavy plugin usage
- Risk-averse environments

**Steps:**
1. Deploy Rust Lavalink on separate port
2. Migrate subset of bots/users
3. Gradually increase Rust traffic
4. Monitor performance and stability
5. Complete migration when confident

### Strategy 3: Hybrid Approach

**Best for:**
- Heavy Spotify/Apple Music usage
- Custom plugins that can't be migrated immediately
- Specific Java-only requirements

**Steps:**
1. Keep Java Lavalink for Spotify/plugins
2. Use Rust Lavalink for YouTube/SoundCloud
3. Route traffic based on source type
4. Gradually migrate as features become available

## Step-by-Step Migration

### Phase 1: Preparation

**1. Install Prerequisites:**
```bash
# Install yt-dlp (required for Rust Lavalink)
pip3 install yt-dlp

# Verify installation
yt-dlp --version
```

**2. Download Rust Lavalink:**
```bash
# Download latest release
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-linux-x64
chmod +x lavalink-rust-linux-x64
mv lavalink-rust-linux-x64 lavalink-rust

# Or build from source
git clone https://github.com/lavalink-devs/lavalink-rust.git
cd lavalink-rust
cargo build --release
cp target/release/lavalink-rust ./
```

**3. Prepare Configuration:**
```bash
# Copy existing configuration
cp application.yml application-rust.yml

# Update for Rust-specific features (optional)
```

### Phase 2: Testing

**1. Test Rust Lavalink:**
```bash
# Start on different port for testing
./lavalink-rust --port 2334 --config application-rust.yml
```

**2. Validate Basic Functionality:**
```bash
# Test server info
curl http://localhost:2334/v4/info

# Test track loading
curl "http://localhost:2334/v4/loadtracks?identifier=ytsearch:test"

# Test WebSocket connection
wscat -c ws://localhost:2334
```

**3. Test Audio Sources:**
```bash
# YouTube
curl "http://localhost:2334/v4/loadtracks?identifier=https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# SoundCloud  
curl "http://localhost:2334/v4/loadtracks?identifier=https://soundcloud.com/artist/track"

# Spotify (fallback test)
curl "http://localhost:2334/v4/loadtracks?identifier=https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh"
```

**4. Performance Testing:**
```bash
# Load test
for i in {1..100}; do
  curl -s "http://localhost:2334/v4/loadtracks?identifier=ytsearch:test $i" &
done
wait

# Memory usage
ps aux | grep lavalink-rust
```

### Phase 3: Production Migration

**1. Prepare Production Environment:**
```bash
# Create backup
sudo systemctl stop lavalink
sudo cp -r /opt/lavalink /opt/lavalink-backup-$(date +%Y%m%d)

# Install Rust Lavalink
sudo mkdir -p /opt/lavalink-rust/{bin,config,logs,plugins}
sudo cp lavalink-rust /opt/lavalink-rust/bin/
sudo cp application.yml /opt/lavalink-rust/config/
sudo chown -R lavalink:lavalink /opt/lavalink-rust
```

**2. Update Service Configuration:**
```bash
# Create new systemd service
sudo tee /etc/systemd/system/lavalink-rust.service > /dev/null << 'EOF'
[Unit]
Description=Lavalink Rust Audio Node
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
User=lavalink
Group=lavalink
WorkingDirectory=/opt/lavalink-rust
ExecStart=/opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits (much lower than Java)
MemoryMax=1G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable lavalink-rust
```

**3. Execute Migration:**
```bash
# Stop Java Lavalink
sudo systemctl stop lavalink

# Start Rust Lavalink
sudo systemctl start lavalink-rust

# Verify startup
sudo systemctl status lavalink-rust
sudo journalctl -u lavalink-rust -f
```

**4. Validate Migration:**
```bash
# Test API endpoints
curl http://localhost:2333/v4/info
curl http://localhost:2333/v4/stats

# Test client connections
# (Use your existing bot/client to connect)

# Monitor performance
htop -p $(pgrep lavalink-rust)
```

### Phase 4: Post-Migration

**1. Monitor Performance:**
```bash
# Set up monitoring
curl http://localhost:2333/v4/stats | jq

# Check logs for errors
sudo journalctl -u lavalink-rust --since "1 hour ago"

# Monitor resource usage
watch 'ps aux | grep lavalink-rust'
```

**2. Update Documentation:**
```bash
# Document new setup
echo "Migrated to Rust Lavalink on $(date)" >> /opt/lavalink-rust/MIGRATION_LOG.md
echo "Memory usage: $(ps aux | grep lavalink-rust | awk '{print $6}')KB" >> /opt/lavalink-rust/MIGRATION_LOG.md
```

**3. Clean Up (After Validation):**
```bash
# Disable old service
sudo systemctl disable lavalink

# Archive old installation
sudo tar -czf /opt/lavalink-java-backup-$(date +%Y%m%d).tar.gz /opt/lavalink
sudo rm -rf /opt/lavalink
```

## Configuration Migration

### Basic Configuration

Your existing `application.yml` works without changes:

```yaml
# This configuration works identically in both versions
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
      local: false
    filters:
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

logging:
  level:
    root: INFO
    lavalink: INFO
```

### Rust-Specific Enhancements

Add Rust-specific optimizations:

```yaml
# Add to your existing application.yml
rust:
  # Fallback system for Spotify/Apple Music/Deezer
  fallback:
    enabled: true
    youtube_search_prefix: "ytsearch:"
    max_search_results: 5
    cache_duration: 3600
    
  # Performance tuning
  performance:
    worker_threads: 0              # Auto-detect CPU cores
    blocking_threads: 512
    
  # Memory optimization
  memory:
    track_cache_size: 1000
    player_buffer_size: 4096
```

## Plugin Migration

### Built-in Replacements

| Java Plugin | Rust Equivalent | Migration Action |
|-------------|-----------------|------------------|
| **LavaSrc** | Built-in fallback | Remove plugin, enable fallback |
| **LavaSearch** | Built-in search | Remove plugin, use native search |
| **LavalinkFilter** | Built-in filters | Remove plugin, use native filters |

### Custom Plugin Migration

**For custom plugins, you have options:**

1. **Rewrite for Rust:**
   ```rust
   // Example Rust plugin structure
   use std::ffi::{CStr, CString};
   use std::os::raw::c_char;
   
   #[no_mangle]
   pub extern "C" fn get_name() -> *const c_char {
       CString::new("my-plugin").unwrap().into_raw()
   }
   
   #[no_mangle]
   pub extern "C" fn initialize() -> i32 {
       // Plugin initialization
       0
   }
   ```

2. **Use Hybrid Approach:**
   - Keep Java Lavalink for plugins
   - Use Rust Lavalink for performance-critical tasks

3. **Request Feature:**
   - Submit feature request for built-in equivalent
   - Contribute to Rust Lavalink development

## Client Code Migration

### No Changes Required

**Existing client code works without modification:**

```javascript
// This code works with both Java and Rust Lavalink
const { Manager } = require("erela.js");

const manager = new Manager({
    nodes: [{
        host: "localhost",
        port: 2333,
        password: "youshallnotpass",
    }],
});

// All existing functionality works
await manager.search("ytsearch:never gonna give you up");
await manager.search("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
await manager.search("https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh"); // Now uses fallback
```

### Optional Optimizations

**Take advantage of Rust-specific features:**

```javascript
// Monitor improved performance
manager.on("nodeConnect", (node) => {
    console.log(`Connected to ${node.options.identifier}`);
    console.log(`Memory usage: ${node.stats.memory.used}MB`); // Much lower than Java
});

// Handle fallback notifications (if implemented)
manager.on("trackStart", (player, track) => {
    if (track.sourceName === "youtube" && track.originalUrl?.includes("spotify")) {
        console.log("Playing Spotify track via YouTube fallback");
    }
});
```

## Troubleshooting Migration Issues

### Common Problems

**1. Service Won't Start:**
```bash
# Check dependencies
which yt-dlp
yt-dlp --version

# Check permissions
ls -la /opt/lavalink-rust/bin/lavalink-rust
sudo chmod +x /opt/lavalink-rust/bin/lavalink-rust

# Check logs
sudo journalctl -u lavalink-rust -n 50
```

**2. Audio Sources Not Working:**
```bash
# Test yt-dlp directly
yt-dlp --extract-flat "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Check configuration
grep -A 10 "sources:" /opt/lavalink-rust/config/application.yml
```

**3. Performance Issues:**
```bash
# Check resource usage
htop -p $(pgrep lavalink-rust)

# Optimize configuration
# See: docs/advanced/performance.md
```

**4. Client Connection Issues:**
```bash
# Test connectivity
curl http://localhost:2333/v4/info

# Check WebSocket
wscat -c ws://localhost:2333

# Verify password
grep password /opt/lavalink-rust/config/application.yml
```

### Rollback Procedure

**If migration fails:**

```bash
# Stop Rust Lavalink
sudo systemctl stop lavalink-rust

# Restore Java Lavalink
sudo systemctl start lavalink

# Verify restoration
curl http://localhost:2333/v4/info

# Investigate issues
sudo journalctl -u lavalink-rust --since "1 hour ago"
```

## Performance Validation

### Before/After Comparison

**Memory Usage:**
```bash
# Java Lavalink
ps aux | grep java | grep -i lavalink | awk '{print $6}' # Typically 1-6GB

# Rust Lavalink  
ps aux | grep lavalink-rust | awk '{print $6}' # Typically 256-512MB
```

**Startup Time:**
```bash
# Java Lavalink
time systemctl restart lavalink # Typically 10-15 seconds

# Rust Lavalink
time systemctl restart lavalink-rust # Typically 2-5 seconds
```

**Response Time:**
```bash
# Test API response time
time curl http://localhost:2333/v4/info
```

### Success Metrics

**Migration is successful when:**
- ✅ All API endpoints respond correctly
- ✅ WebSocket connections work
- ✅ Audio playback functions normally
- ✅ Memory usage is significantly reduced
- ✅ Startup time is faster
- ✅ No increase in error rates
- ✅ Client applications work without changes

## Post-Migration Optimization

### Performance Tuning

**Optimize for your workload:**
```yaml
# High-traffic optimization
rust:
  performance:
    worker_threads: 8
    blocking_threads: 1024
  memory:
    track_cache_size: 5000
    player_buffer_size: 8192
```

### Monitoring Setup

**Set up comprehensive monitoring:**
```yaml
# Enable metrics
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    port: 9090
```

### Documentation Updates

**Update your documentation:**
- Installation procedures
- Configuration management
- Troubleshooting guides
- Performance baselines
- Monitoring dashboards

## Getting Help

### Support Resources

- **GitHub Issues:** [Report bugs](https://github.com/lavalink-devs/lavalink-rust/issues)
- **GitHub Discussions:** [Ask questions](https://github.com/lavalink-devs/lavalink-rust/discussions)
- **Discord:** [Community support](https://discord.gg/lavalink)
- **Documentation:** [Complete guides](../index.md)

### Migration Assistance

**For complex migrations:**
1. Review [troubleshooting guide](../getting-started/troubleshooting.md)
2. Check [performance guide](../advanced/performance.md)
3. Join community discussions
4. Consider professional support services

**Remember:** Migration is a process, not an event. Take time to validate each step and don't hesitate to ask for help when needed.

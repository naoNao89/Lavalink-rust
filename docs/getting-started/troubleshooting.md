---
description: Lavalink Rust troubleshooting guide and common solutions.
---

# Troubleshooting

This guide covers common issues and their solutions when running Lavalink Rust.

## Installation Issues

### Binary Won't Start

**Symptoms:**
- "Permission denied" error
- "No such file or directory" error
- Binary exits immediately

**Solutions:**

```bash
# Make binary executable
chmod +x lavalink-rust

# Check if binary is corrupted
file lavalink-rust
# Should show: ELF 64-bit LSB executable

# Verify architecture compatibility
uname -m  # Should match binary architecture (x86_64, aarch64, etc.)

# Check for missing libraries (Linux)
ldd lavalink-rust
```

### Missing Dependencies

**Symptoms:**
- "yt-dlp not found" error
- Audio sources not working
- "command not found" errors

**Solutions:**

```bash
# Install yt-dlp
pip3 install yt-dlp

# Verify installation
which yt-dlp
yt-dlp --version

# Add to PATH if needed
export PATH=$PATH:/usr/local/bin
echo 'export PATH=$PATH:/usr/local/bin' >> ~/.bashrc

# For system-wide installation
sudo pip3 install yt-dlp
```

### Configuration File Issues

**Symptoms:**
- "Configuration file not found"
- "Invalid YAML syntax"
- "Unknown configuration key"

**Solutions:**

```bash
# Verify file exists and is readable
ls -la application.yml
cat application.yml

# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('application.yml'))"

# Test configuration
./lavalink-rust --config application.yml --validate

# Use absolute path
./lavalink-rust --config /full/path/to/application.yml
```

## Connection Issues

### Port Already in Use

**Symptoms:**
- "Address already in use" error
- "Failed to bind to port 2333"

**Solutions:**

```bash
# Check what's using the port
sudo netstat -tulpn | grep 2333
sudo ss -tulpn | grep 2333

# Kill conflicting process
sudo kill $(sudo lsof -t -i:2333)

# Use different port
./lavalink-rust --port 2334

# Or in configuration:
# server:
#   port: 2334
```

### Firewall Blocking Connections

**Symptoms:**
- Clients can't connect
- Connection timeouts
- "Connection refused" errors

**Solutions:**

```bash
# Check firewall status
sudo ufw status  # Ubuntu/Debian
sudo firewall-cmd --list-all  # CentOS/RHEL

# Allow Lavalink port
sudo ufw allow 2333
sudo firewall-cmd --permanent --add-port=2333/tcp
sudo firewall-cmd --reload

# Test connectivity
telnet localhost 2333
curl http://localhost:2333/v4/info
```

### WebSocket Connection Failures

**Symptoms:**
- "WebSocket connection failed"
- "Handshake failed"
- Clients can't establish WebSocket connection

**Solutions:**

```bash
# Test WebSocket connection
wscat -c ws://localhost:2333

# Check authentication
curl -H "Authorization: youshallnotpass" http://localhost:2333/v4/info

# Verify client configuration
# Ensure password matches between client and server
```

## Audio Playback Issues

### No Audio Playing

**Symptoms:**
- Tracks load but don't play
- Silent playback
- "Player not found" errors

**Diagnostic Steps:**

```bash
# Test track loading
curl "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test"

# Check player status
curl -H "Authorization: youshallnotpass" \
     "http://localhost:2333/v4/sessions/SESSION_ID/players/GUILD_ID"

# Verify yt-dlp functionality
yt-dlp --extract-flat "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

**Common Solutions:**

1. **Check Discord bot permissions:**
   - Connect to voice channel
   - Speak permission
   - Use voice activity

2. **Verify audio source:**
   ```yaml
   lavalink:
     server:
       sources:
         youtube: true
         soundcloud: true
         http: true
   ```

3. **Check player state:**
   ```bash
   # Resume paused player
   curl -X PATCH \
        -H "Authorization: youshallnotpass" \
        -H "Content-Type: application/json" \
        -d '{"paused": false}' \
        "http://localhost:2333/v4/sessions/SESSION_ID/players/GUILD_ID"
   ```

### Poor Audio Quality

**Symptoms:**
- Crackling or distorted audio
- Frequent buffering
- Audio cuts out

**Solutions:**

```yaml
# Increase buffer sizes
lavalink:
  server:
    buffer_duration_ms: 400
    frame_buffer_duration_ms: 5000
    track_stuck_threshold_ms: 10000

# Adjust audio quality
    filters:
      volume: true
      equalizer: true
    
# Optimize network settings
server:
  http:
    timeout: 30s
    keep_alive: 60s
```

### YouTube/Audio Source Errors

**Symptoms:**
- "Video unavailable"
- "Age-restricted content"
- "Geo-blocked content"

**Solutions:**

```bash
# Update yt-dlp
pip3 install --upgrade yt-dlp

# Test specific URL
yt-dlp --verbose "https://www.youtube.com/watch?v=VIDEO_ID"

# Check for geo-restrictions
yt-dlp --geo-bypass "https://www.youtube.com/watch?v=VIDEO_ID"

# Use different audio source
curl "http://localhost:2333/v4/loadtracks?identifier=scsearch:your query"
```

## Performance Issues

### High Memory Usage

**Symptoms:**
- Memory usage > 1GB
- Out of memory errors
- System slowdown

**Diagnostic:**

```bash
# Check memory usage
ps aux | grep lavalink-rust
top -p $(pgrep lavalink-rust)

# Monitor memory over time
while true; do
    ps -o pid,vsz,rss,comm -p $(pgrep lavalink-rust)
    sleep 5
done
```

**Solutions:**

```yaml
# Reduce buffer sizes
lavalink:
  server:
    buffer_duration_ms: 200
    frame_buffer_duration_ms: 2000

# Limit concurrent players
    player_update_interval: 5
    
# Disable unused features
    filters:
      volume: true
      equalizer: false
      karaoke: false
```

### High CPU Usage

**Symptoms:**
- CPU usage > 100%
- System lag
- Slow response times

**Solutions:**

```yaml
# Reduce audio processing
lavalink:
  server:
    frame_buffer_duration_ms: 5000
    track_stuck_threshold_ms: 10000

# Optimize filters
    filters:
      # Disable expensive filters
      equalizer: false
      karaoke: false
      timescale: false
```

### Slow Startup

**Symptoms:**
- Takes > 10 seconds to start
- "Server not ready" messages

**Solutions:**

```bash
# Check disk I/O
iostat -x 1

# Verify no competing processes
ps aux | grep -E "(java|lavalink)"

# Use SSD storage if possible
# Ensure sufficient RAM available
```

## Docker Issues

### Container Won't Start

**Symptoms:**
- Container exits immediately
- "exec format error"
- Permission denied in container

**Solutions:**

```bash
# Check container logs
docker logs lavalink-rust

# Verify image architecture
docker inspect ghcr.io/lavalink-devs/lavalink-rust:latest | grep Architecture

# Fix permissions
docker run --user 322:322 ghcr.io/lavalink-devs/lavalink-rust:latest

# Test with interactive shell
docker run -it --entrypoint /bin/bash ghcr.io/lavalink-devs/lavalink-rust:latest
```

### Volume Mount Issues

**Symptoms:**
- Configuration not loading
- "File not found" in container
- Permission errors

**Solutions:**

```bash
# Check file permissions
ls -la application.yml

# Fix ownership
sudo chown 322:322 application.yml

# Use absolute paths
docker run -v $(pwd)/application.yml:/app/application.yml:ro

# Verify mount
docker exec lavalink-rust ls -la /app/
```

### Network Issues in Docker

**Symptoms:**
- Can't connect to container
- Port not accessible
- Container can't reach internet

**Solutions:**

```yaml
# Expose ports correctly
ports:
  - "2333:2333"

# Use host networking (if needed)
network_mode: host

# Check Docker networks
docker network ls
docker network inspect bridge
```

## Systemd Service Issues

### Service Won't Start

**Symptoms:**
- "Failed to start" error
- Service exits immediately
- "Unit not found"

**Solutions:**

```bash
# Check service status
sudo systemctl status lavalink-rust

# View detailed logs
sudo journalctl -u lavalink-rust -n 50

# Validate service file
sudo systemd-analyze verify /etc/systemd/system/lavalink-rust.service

# Reload systemd
sudo systemctl daemon-reload
```

### Permission Issues

**Symptoms:**
- "Permission denied" in logs
- Can't write to log files
- Configuration file not readable

**Solutions:**

```bash
# Fix ownership
sudo chown -R lavalink:lavalink /opt/lavalink-rust

# Fix permissions
sudo chmod +x /opt/lavalink-rust/bin/lavalink-rust
sudo chmod 644 /opt/lavalink-rust/config/application.yml
sudo chmod 755 /opt/lavalink-rust/logs

# Test as service user
sudo -u lavalink /opt/lavalink-rust/bin/lavalink-rust --version
```

### Service Keeps Restarting

**Symptoms:**
- Constant restart loops
- "Start request repeated too quickly"

**Solutions:**

```bash
# Check restart settings
sudo systemctl show lavalink-rust | grep Restart

# Increase restart delay
sudo systemctl edit lavalink-rust
```

Add:
```ini
[Service]
RestartSec=30
```

## Logging and Debugging

### Enable Debug Logging

```yaml
# In application.yml
logging:
  level:
    root: DEBUG
    lavalink: TRACE

# Or via environment
RUST_LOG=debug ./lavalink-rust

# Or via CLI flags
./lavalink-rust --debug              # Debug level
./lavalink-rust --trace              # Trace level (very verbose)
./lavalink-rust --log-level trace    # Custom level
./lavalink-rust --debug --json-logs  # Debug with JSON output
```

### Capture Network Traffic

```bash
# Monitor HTTP requests
sudo tcpdump -i any -A 'port 2333'

# Monitor WebSocket traffic
sudo tcpdump -i any -A 'port 2333 and tcp'
```

### Performance Profiling

```bash
# CPU profiling
perf record -g ./lavalink-rust
perf report

# Memory profiling
valgrind --tool=massif ./lavalink-rust
```

## Getting Help

### Gathering Information

When reporting issues, include:

1. **System Information:**
   ```bash
   uname -a
   ./lavalink-rust --version
   yt-dlp --version
   ```

2. **Configuration:**
   ```bash
   # Remove sensitive data like passwords
   cat application.yml
   ```

3. **Logs:**
   ```bash
   # Last 100 lines
   sudo journalctl -u lavalink-rust -n 100
   
   # Or application logs
   tail -n 100 logs/lavalink.log
   ```

4. **Error Details:**
   - Exact error messages
   - Steps to reproduce
   - Expected vs actual behavior

### Support Channels

- **GitHub Issues:** [Bug Reports](https://github.com/lavalink-devs/lavalink-rust/issues)
- **GitHub Discussions:** [Q&A](https://github.com/lavalink-devs/lavalink-rust/discussions)
- **Discord:** [Lavalink Support Server](https://discord.gg/lavalink)

### Before Reporting

1. **Search existing issues** for similar problems
2. **Try latest version** to see if issue is fixed
3. **Test with minimal configuration** to isolate the problem
4. **Check FAQ** for common solutions

## Quick Reference

### Common Commands

```bash
# Test connectivity
curl http://localhost:2333/v4/info

# Load a track
curl "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test"

# Check service status
sudo systemctl status lavalink-rust

# View logs
sudo journalctl -u lavalink-rust -f

# Restart service
sudo systemctl restart lavalink-rust

# Test configuration
./lavalink-rust --config application.yml --validate
```

### Emergency Recovery

```bash
# Stop all Lavalink processes
sudo pkill -f lavalink

# Reset to default configuration
cp application.yml.example application.yml

# Start with minimal config
./lavalink-rust --port 2334 --password emergency

# Check system resources
free -h
df -h
top
```

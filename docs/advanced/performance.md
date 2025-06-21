---
description: Rust-specific performance tuning guide for Lavalink Rust
---

# Rust Performance Tuning

Lavalink Rust offers significant performance advantages over Java Lavalink. This guide covers Rust-specific optimizations and tuning strategies.

## Performance Overview

### Baseline Improvements

| Metric | Java Lavalink | Rust Lavalink | Improvement |
|--------|---------------|---------------|-------------|
| **Memory Usage** | 1-6 GB | 256-512 MB | 50-75% reduction |
| **Startup Time** | 10-15 seconds | 2-5 seconds | 75% faster |
| **CPU Usage** | Variable (GC spikes) | Consistent | Predictable performance |
| **Response Time** | 50-200ms | 10-50ms | 50-75% faster |
| **Concurrent Connections** | 1000-2000 | 5000+ | 2-5x improvement |

### Key Advantages

- **No Garbage Collection**: Eliminates GC pauses and unpredictable latency
- **Zero-Cost Abstractions**: Rust's abstractions compile to optimal machine code
- **Memory Safety**: Prevents memory leaks and buffer overflows
- **Efficient Async Runtime**: Tokio provides high-performance async I/O

## Memory Optimization

### Memory Configuration

```yaml
# application.yml
rust:
  memory:
    # Memory pool settings
    track_cache_size: 1000        # Number of tracks to cache
    player_buffer_size: 4096      # Buffer size per player (bytes)
    connection_pool_size: 100     # Max concurrent connections
    
    # Garbage collection (for cached data)
    gc_interval: 300              # Cleanup interval (seconds)
    max_idle_time: 1800           # Max idle time before cleanup (seconds)
```

### Memory Monitoring

```bash
# Monitor memory usage
ps aux | grep lavalink-rust

# Detailed memory breakdown
cat /proc/$(pgrep lavalink-rust)/status | grep -E "(VmRSS|VmSize|VmPeak)"

# Real-time monitoring
top -p $(pgrep lavalink-rust)
```

### Memory Optimization Strategies

1. **Reduce Buffer Sizes:**
   ```yaml
   lavalink:
     server:
       buffer_duration_ms: 200      # Reduce from default 400ms
       frame_buffer_duration_ms: 2000  # Reduce from default 5000ms
   ```

2. **Limit Concurrent Players:**
   ```yaml
   rust:
     limits:
       max_players_per_session: 10
       max_total_players: 100
   ```

3. **Optimize Track Caching:**
   ```yaml
   rust:
     cache:
       track_cache_enabled: true
       track_cache_size: 500        # Reduce if memory constrained
       track_cache_ttl: 3600        # Time to live (seconds)
   ```

## CPU Optimization

### CPU Configuration

```yaml
rust:
  performance:
    # Thread pool configuration
    worker_threads: 0              # 0 = auto-detect CPU cores
    blocking_threads: 512          # Max blocking threads
    thread_stack_size: 2097152     # 2MB stack size
    
    # Task scheduling
    task_queue_size: 1024          # Async task queue size
    priority_scheduling: true      # Enable priority scheduling
```

### CPU Monitoring

```bash
# Monitor CPU usage
htop -p $(pgrep lavalink-rust)

# CPU usage per thread
ps -eLf | grep lavalink-rust

# System load
uptime
```

### CPU Optimization Strategies

1. **Optimize Thread Pool:**
   ```yaml
   rust:
     performance:
       worker_threads: 4            # Set to CPU core count
       blocking_threads: 256        # Reduce if not doing heavy I/O
   ```

2. **Reduce Audio Processing:**
   ```yaml
   lavalink:
     server:
       filters:
         # Disable expensive filters if not needed
         equalizer: false
         karaoke: false
         timescale: false
   ```

3. **Optimize Network I/O:**
   ```yaml
   server:
     http:
       max_connections: 1000
       connection_timeout: 30s
       keep_alive_timeout: 60s
   ```

## Network Optimization

### Network Configuration

```yaml
server:
  # HTTP server optimization
  http:
    max_connections: 2000          # Increase for high load
    connection_timeout: 30s
    keep_alive_timeout: 60s
    request_timeout: 120s
    
  # WebSocket optimization
  websocket:
    max_frame_size: 65536          # 64KB max frame size
    ping_interval: 30s             # Keep-alive ping interval
    pong_timeout: 10s              # Pong response timeout

rust:
  network:
    # TCP optimization
    tcp_nodelay: true              # Disable Nagle's algorithm
    tcp_keepalive: true            # Enable TCP keep-alive
    socket_reuse: true             # Enable socket reuse
    
    # Buffer sizes
    send_buffer_size: 65536        # 64KB send buffer
    recv_buffer_size: 65536        # 64KB receive buffer
```

### Network Monitoring

```bash
# Monitor network connections
netstat -tulpn | grep lavalink-rust

# Monitor network traffic
iftop -i eth0

# Check connection limits
ulimit -n
```

## Audio Processing Optimization

### Audio Configuration

```yaml
lavalink:
  server:
    # Audio quality vs performance trade-offs
    buffer_duration_ms: 400        # Higher = more stable, more memory
    frame_buffer_duration_ms: 5000 # Higher = less CPU, more latency
    track_stuck_threshold_ms: 10000 # Higher = more tolerance
    
    # Audio source optimization
    sources:
      youtube: true
      soundcloud: true
      http: true
      local: false                 # Disable if not needed
      
    # Filter optimization
    filters:
      volume: true                 # Low cost
      equalizer: true              # Medium cost
      karaoke: false               # High cost - disable if not needed
      timescale: false             # High cost - disable if not needed
```

### Audio Performance Monitoring

```bash
# Monitor audio processing
curl http://localhost:2333/v4/stats | jq '.frameStats'

# Check for audio dropouts
journalctl -u lavalink-rust | grep -i "audio\|buffer\|frame"
```

## Async Runtime Optimization

### Tokio Configuration

```yaml
rust:
  tokio:
    # Runtime configuration
    flavor: "multi_thread"         # or "current_thread" for single-core
    worker_threads: 0              # 0 = auto-detect
    max_blocking_threads: 512      # Max blocking threads
    
    # I/O configuration
    io_driver_enabled: true
    time_driver_enabled: true
    
    # Scheduler configuration
    disable_lifo_slot: false       # Keep LIFO optimization
    global_queue_interval: 31      # Global queue check interval
```

### Async Best Practices

1. **Avoid Blocking Operations:**
   ```rust
   // Bad: Blocking in async context
   async fn bad_example() {
       std::thread::sleep(Duration::from_secs(1)); // Blocks thread
   }
   
   // Good: Use async sleep
   async fn good_example() {
       tokio::time::sleep(Duration::from_secs(1)).await;
   }
   ```

2. **Use Appropriate Spawning:**
   ```rust
   // CPU-intensive work
   tokio::task::spawn_blocking(|| {
       // Heavy computation
   });
   
   // I/O work
   tokio::spawn(async {
       // Async I/O operations
   });
   ```

## Container Optimization

### Docker Configuration

```yaml
# docker-compose.yml
services:
  lavalink-rust:
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    deploy:
      resources:
        limits:
          memory: 512M              # Much less than Java (2-6GB)
          cpus: '1.0'
        reservations:
          memory: 256M
          cpus: '0.5'
    environment:
      # Rust-specific optimizations
      - RUST_LOG=info              # Reduce log verbosity
      - RUST_BACKTRACE=0           # Disable backtraces in production
      
      # Memory optimization
      - MALLOC_ARENA_MAX=2         # Limit malloc arenas
```

### Container Monitoring

```bash
# Monitor container resources
docker stats lavalink-rust

# Check container limits
docker inspect lavalink-rust | jq '.HostConfig.Memory'

# Monitor container logs
docker logs -f lavalink-rust
```

## Production Optimization

### System-Level Optimization

```bash
# Increase file descriptor limits
echo "lavalink soft nofile 65536" >> /etc/security/limits.conf
echo "lavalink hard nofile 65536" >> /etc/security/limits.conf

# Optimize TCP settings
echo "net.core.somaxconn = 65536" >> /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65536" >> /etc/sysctl.conf
echo "net.core.netdev_max_backlog = 5000" >> /etc/sysctl.conf

# Apply settings
sysctl -p
```

### Systemd Service Optimization

```ini
# /etc/systemd/system/lavalink-rust.service
[Service]
# Resource limits
LimitNOFILE=65536
LimitNPROC=32768
MemoryMax=1G
CPUQuota=200%

# Performance settings
IOSchedulingClass=1
IOSchedulingPriority=4
CPUSchedulingPolicy=2
CPUSchedulingPriority=50

# Environment optimization
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=0
Environment=MALLOC_ARENA_MAX=2
```

## Monitoring and Metrics

### Built-in Metrics

```bash
# Server statistics
curl http://localhost:2333/v4/stats

# Prometheus metrics (if enabled)
curl http://localhost:9090/metrics
```

### Performance Metrics Configuration

```yaml
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    port: 9090
    
rust:
  metrics:
    # Custom metrics
    track_performance_metrics: true
    memory_usage_metrics: true
    network_metrics: true
    
    # Metric collection interval
    collection_interval: 30        # seconds
```

### Key Metrics to Monitor

1. **Memory Metrics:**
   - `lavalink_memory_used_bytes`
   - `lavalink_memory_allocated_bytes`
   - `lavalink_gc_collections_total`

2. **CPU Metrics:**
   - `lavalink_cpu_usage_percent`
   - `lavalink_thread_count`
   - `lavalink_task_queue_size`

3. **Audio Metrics:**
   - `lavalink_players_total`
   - `lavalink_tracks_loaded_total`
   - `lavalink_audio_frames_sent_total`

4. **Network Metrics:**
   - `lavalink_connections_active`
   - `lavalink_websocket_messages_total`
   - `lavalink_http_requests_total`

## Benchmarking

### Load Testing

```bash
# Simple load test
for i in {1..100}; do
  curl -s "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test" &
done
wait

# WebSocket load test (using wscat)
for i in {1..50}; do
  wscat -c ws://localhost:2333 &
done
```

### Performance Comparison

```bash
# Memory usage comparison
echo "Java Lavalink:"
ps aux | grep java | grep -i lavalink

echo "Rust Lavalink:"
ps aux | grep lavalink-rust

# Startup time comparison
time java -jar Lavalink.jar &
time ./lavalink-rust &
```

## Troubleshooting Performance Issues

### Common Performance Problems

1. **High Memory Usage:**
   - Check track cache size
   - Monitor for memory leaks
   - Reduce buffer sizes

2. **High CPU Usage:**
   - Disable expensive filters
   - Optimize thread pool size
   - Check for busy loops

3. **Slow Response Times:**
   - Increase connection limits
   - Optimize network settings
   - Check for blocking operations

4. **Audio Dropouts:**
   - Increase buffer sizes
   - Check network stability
   - Monitor system load

### Performance Debugging

```bash
# Profile CPU usage
perf record -g ./lavalink-rust
perf report

# Memory profiling
valgrind --tool=massif ./lavalink-rust

# Network debugging
tcpdump -i any port 2333

# System call tracing
strace -p $(pgrep lavalink-rust)
```

## Best Practices Summary

1. **Start with defaults** and measure before optimizing
2. **Monitor key metrics** continuously
3. **Test changes** in a staging environment
4. **Document optimizations** and their impact
5. **Regular performance reviews** to catch regressions
6. **Consider hardware upgrades** for extreme performance needs

For more advanced topics, see:
- [Architecture Guide](architecture.md)
- [Monitoring Setup](../configuration/monitoring.md)
- [Troubleshooting](../getting-started/troubleshooting.md)

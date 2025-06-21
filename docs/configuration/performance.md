---
description: Performance tuning guide for Lavalink Rust
---

# Performance Tuning

Lavalink Rust provides superior performance out of the box, but can be further optimized for specific use cases and hardware configurations.

!!! performance "Performance Baseline"
    **Default Performance Benefits:**
    
    - 🚀 **50% less memory usage** compared to Java Lavalink
    - ⚡ **80% faster startup time** (2s vs 10-15s)
    - 🔄 **20-30% better CPU efficiency** with no GC pauses
    - 📊 **More predictable performance** under load

## Performance Profiles

### Low-Resource Configuration

Optimized for minimal resource usage (VPS, containers with limited resources):

```yaml
server:
  port: 2333
  address: 0.0.0.0

lavalink:
  server:
    password: "youshallnotpass"
    bufferDurationMs: 200        # Smaller buffer
    frameBufferDurationMs: 2000  # Reduced frame buffer
    playerUpdateInterval: 10     # Less frequent updates
    
rust:
  performance:
    worker_threads: 2            # Limit worker threads
    max_blocking_threads: 32     # Reduce blocking threads
    thread_stack_size: 1048576   # Smaller stack size (1MB)
    
  memory:
    max_track_cache: 100         # Smaller cache
    cleanup_interval: 120        # More frequent cleanup
    
  audio:
    sample_rate: 44100           # Lower sample rate
    bit_depth: 16               # Lower bit depth
    
  filters:
    performance:
      parallel_processing: false # Disable parallel processing
      buffer_size: 512          # Smaller buffer
      thread_pool_size: 1       # Single thread
```

**Expected Resource Usage:**
- Memory: ~50-80MB
- CPU: 1-2 cores
- Suitable for: Small Discord bots, development environments

### Balanced Configuration

Optimized for typical production use (dedicated servers, good hardware):

```yaml
server:
  port: 2333
  address: 0.0.0.0

lavalink:
  server:
    password: "youshallnotpass"
    bufferDurationMs: 400        # Default buffer
    frameBufferDurationMs: 5000  # Default frame buffer
    playerUpdateInterval: 5      # Default updates
    
rust:
  performance:
    worker_threads: 0            # Auto-detect (recommended)
    max_blocking_threads: 512    # Default
    thread_stack_size: 2097152   # Default (2MB)
    
  memory:
    max_track_cache: 1000        # Good cache size
    cleanup_interval: 300        # Default cleanup
    
  audio:
    sample_rate: 48000           # High quality
    bit_depth: 16               # Standard bit depth
    
  filters:
    performance:
      parallel_processing: true  # Enable parallel processing
      buffer_size: 1024         # Balanced buffer
      thread_pool_size: 4       # Multi-threaded
```

**Expected Resource Usage:**
- Memory: ~100-150MB
- CPU: 2-4 cores
- Suitable for: Most Discord bots, production environments

### High-Performance Configuration

Optimized for maximum performance (dedicated servers, high-end hardware):

```yaml
server:
  port: 2333
  address: 0.0.0.0

lavalink:
  server:
    password: "youshallnotpass"
    bufferDurationMs: 800        # Larger buffer for stability
    frameBufferDurationMs: 10000 # Large frame buffer
    playerUpdateInterval: 1      # Frequent updates
    
rust:
  performance:
    worker_threads: 8            # Match CPU cores
    max_blocking_threads: 1024   # High concurrency
    thread_stack_size: 4194304   # Larger stack (4MB)
    
  memory:
    max_track_cache: 5000        # Large cache
    cleanup_interval: 600        # Less frequent cleanup
    
  audio:
    sample_rate: 48000           # High quality
    bit_depth: 24               # High bit depth
    
  filters:
    performance:
      parallel_processing: true  # Enable parallel processing
      simd_optimization: true    # Use SIMD instructions
      buffer_size: 2048         # Large buffer
      thread_pool_size: 8       # Many threads
      
    quality:
      sample_rate: 96000        # Very high internal sample rate
      bit_depth: 32             # High internal bit depth
      dithering: true           # High-quality dithering
```

**Expected Resource Usage:**
- Memory: ~200-400MB
- CPU: 4-8+ cores
- Suitable for: Large Discord bots, high-traffic applications

## CPU Optimization

### Thread Configuration

```yaml
rust:
  performance:
    # Worker threads handle async tasks (networking, I/O)
    worker_threads: 0            # 0 = auto-detect (recommended)
    
    # Blocking threads handle CPU-intensive tasks
    max_blocking_threads: 512    # Increase for high concurrency
    
    # Thread stack size affects memory usage per thread
    thread_stack_size: 2097152   # 2MB default, reduce for memory savings
    
    # Enable thread parking for better CPU efficiency
    thread_parking: true
    
    # CPU affinity (Linux only)
    cpu_affinity: []             # Pin threads to specific CPU cores
```

### Audio Processing Optimization

```yaml
rust:
  audio:
    # Processing configuration
    processing_threads: 0        # 0 = auto-detect
    processing_priority: "high"  # "low", "normal", "high"
    
    # Buffer configuration
    input_buffer_size: 4096      # Input audio buffer
    output_buffer_size: 4096     # Output audio buffer
    
    # Quality vs performance trade-offs
    resampling_quality: "medium" # "low", "medium", "high", "very_high"
    interpolation: "linear"      # "linear", "cubic", "sinc"
```

### Filter Optimization

```yaml
rust:
  filters:
    performance:
      # Parallel processing
      parallel_processing: true  # Process filters in parallel
      thread_pool_size: 4        # Number of filter processing threads
      
      # SIMD optimization (x86_64 only)
      simd_optimization: true    # Use AVX/SSE instructions
      
      # Buffer configuration
      buffer_size: 1024          # Filter processing buffer size
      lookahead_samples: 256     # Lookahead for better quality
      
      # Quality settings
      precision: "single"        # "single", "double" (float precision)
      dithering: false          # Enable for higher quality
```

## Memory Optimization

### Cache Configuration

```yaml
rust:
  memory:
    # Track cache
    max_track_cache: 1000        # Maximum cached tracks
    cache_ttl: 3600             # Cache time-to-live (seconds)
    
    # Cleanup configuration
    cleanup_interval: 300        # Cleanup interval (seconds)
    cleanup_threshold: 0.8       # Cleanup when 80% full
    
    # Memory pools
    audio_buffer_pool_size: 100  # Pre-allocated audio buffers
    metadata_cache_size: 500     # Metadata cache size
    
    # Garbage collection (Rust doesn't have GC, but manages allocations)
    allocation_strategy: "pool"  # "pool", "direct"
    deallocation_batch_size: 50  # Batch deallocations
```

### Memory Monitoring

```yaml
rust:
  monitoring:
    memory:
      enabled: true
      log_usage: true            # Log memory usage
      warning_threshold: 0.8     # Warn at 80% of limit
      critical_threshold: 0.95   # Critical at 95% of limit
      
      # Memory limits (optional)
      max_heap_size: "1GB"       # Maximum memory usage
      max_cache_size: "256MB"    # Maximum cache size
```

## Network Optimization

### Connection Configuration

```yaml
server:
  # HTTP/2 configuration (experimental)
  http2:
    enabled: false             # Enable HTTP/2 support
    max_concurrent_streams: 100 # Max concurrent streams
    
rust:
  network:
    # Connection pooling
    connection_pool_size: 100   # HTTP connection pool size
    connection_timeout: 30      # Connection timeout (seconds)
    request_timeout: 60         # Request timeout (seconds)
    
    # Keep-alive configuration
    keep_alive: true           # Enable keep-alive
    keep_alive_timeout: 60     # Keep-alive timeout (seconds)
    
    # Buffer sizes
    socket_buffer_size: 65536  # Socket buffer size
    tcp_nodelay: true          # Disable Nagle's algorithm
```

### Rate Limiting Optimization

```yaml
lavalink:
  server:
    ratelimit:
      strategy: "RotateOnBan"    # Efficient rate limiting
      retryLimit: 3              # Limit retries
      
rust:
  ratelimit:
    # Efficient rate limiting implementation
    algorithm: "token_bucket"    # "token_bucket", "sliding_window"
    burst_allowance: 1.5        # Allow 150% burst
    recovery_time: 60           # Recovery time (seconds)
```

## Disk I/O Optimization

### Logging Configuration

```yaml
logging:
  # Async logging for better performance
  async: true
  buffer_size: 8192            # Log buffer size
  
  file:
    enabled: true
    path: "./logs/lavalink.log"
    # Rotation for performance
    max_size: "100MB"          # Rotate at 100MB
    max_files: 5               # Keep 5 files
    compression: true          # Compress old logs
    
  # Reduce log verbosity for performance
  level:
    root: "INFO"               # Reduce from DEBUG
    tokio: "WARN"              # Reduce tokio logs
    hyper: "WARN"              # Reduce HTTP logs
```

### Cache Storage

```yaml
rust:
  storage:
    # Cache storage configuration
    cache_directory: "./cache"  # Cache directory
    cache_compression: true     # Compress cached data
    cache_encryption: false     # Encrypt cached data (adds overhead)
    
    # I/O configuration
    io_threads: 2              # Dedicated I/O threads
    sync_interval: 30          # Sync to disk interval (seconds)
    write_buffer_size: 1048576 # Write buffer size (1MB)
```

## Monitoring and Profiling

### Performance Metrics

```yaml
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    
rust:
  metrics:
    # Detailed metrics
    detailed_metrics: true      # Enable detailed metrics
    histogram_buckets: [0.1, 0.5, 1.0, 2.5, 5.0, 10.0] # Custom buckets
    
    # Performance tracking
    track_allocations: false    # Track memory allocations (overhead)
    track_cpu_usage: true       # Track CPU usage per component
    track_latency: true         # Track request latency
```

### Debug Configuration

```yaml
rust:
  debug:
    # Performance debugging
    profile_audio_processing: false # Profile audio processing
    profile_network_requests: false # Profile network requests
    profile_memory_usage: false     # Profile memory usage
    
    # Tracing (development only)
    enable_tracing: false       # Enable detailed tracing
    trace_filters: []           # Trace specific components
```

## Platform-Specific Optimizations

### Linux Optimizations

```yaml
rust:
  platform:
    linux:
      # Use io_uring for better I/O performance (Linux 5.1+)
      use_io_uring: true
      
      # CPU governor
      cpu_governor: "performance" # Set CPU governor
      
      # Memory management
      use_huge_pages: false      # Use huge pages (requires setup)
      memory_overcommit: 1       # Memory overcommit strategy
```

### Windows Optimizations

```yaml
rust:
  platform:
    windows:
      # Windows-specific optimizations
      high_precision_timer: true # Use high precision timer
      process_priority: "high"   # Set process priority
      
      # Memory management
      working_set_size: "auto"   # Working set size
```

### Docker Optimizations

```yaml
rust:
  docker:
    # Container-specific optimizations
    detect_container: true      # Auto-detect container environment
    container_memory_limit: "auto" # Use container memory limits
    
    # Resource constraints
    respect_cgroup_limits: true # Respect cgroup limits
    adaptive_threading: true   # Adapt to container resources
```

## Benchmarking and Testing

### Performance Testing

```bash
# CPU benchmark
./lavalink-rust --benchmark cpu

# Memory benchmark  
./lavalink-rust --benchmark memory

# Network benchmark
./lavalink-rust --benchmark network

# Full benchmark suite
./lavalink-rust --benchmark all --output benchmark-results.json
```

### Load Testing

```yaml
rust:
  testing:
    # Load testing configuration
    max_concurrent_players: 1000 # Maximum players for testing
    test_duration: 300           # Test duration (seconds)
    ramp_up_time: 60            # Ramp up time (seconds)
    
    # Synthetic load
    synthetic_tracks: true       # Use synthetic audio for testing
    track_duration: 180         # Synthetic track duration (seconds)
```

For more information, see:
- [Configuration Overview](index.md)
- [Audio Sources Configuration](sources.md)
- [Audio Filters Configuration](filters.md)
- [Monitoring Guide](../advanced/monitoring.md)

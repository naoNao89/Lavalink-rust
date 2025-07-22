# ARM/IoT Support for Lavalink-rust

This document describes the ARM and IoT device support implementation for Lavalink-rust.

## Overview

Lavalink-rust now supports ARM-based IoT devices including TV boxes, embedded systems, and single-board computers running 32-bit ARM Debian and similar distributions.

## Supported ARM Targets

- `armv7-unknown-linux-gnueabihf` - ARMv7 with hard-float ABI (recommended for most ARM devices)
- `armv7-unknown-linux-musleabihf` - ARMv7 with musl libc (for static linking)
- `arm-unknown-linux-gnueabihf` - ARMv6 with hard-float ABI (for older ARM devices)

## Feature Sets

### ARM-specific Features

- `arm-optimizations` - Enable ARM-specific performance optimizations
- `neon-simd` - Use NEON SIMD instructions for audio processing
- `alsa-backend` - ALSA audio backend for ARM Linux systems
- `resource-monitoring` - Monitor ARM device resources (CPU, memory, temperature)
- `adaptive-quality` - Automatically adjust quality based on device capabilities
- `low-memory` - Optimizations for memory-constrained devices
- `no-std-compat` - Compatibility layer for embedded environments

### Feature Bundles

- `arm-iot` - Basic IoT device support with optimizations
- `arm-tv-box` - Optimized for ARM TV boxes with ALSA audio
- `arm-embedded` - Minimal feature set for embedded systems

## Building for ARM

### Prerequisites

1. Install Rust ARM targets:
```bash
rustup target add armv7-unknown-linux-gnueabihf
rustup target add armv7-unknown-linux-musleabihf
rustup target add arm-unknown-linux-gnueabihf
```

2. Install ARM cross-compilation toolchain:
```bash
# On Ubuntu/Debian
sudo apt-get install gcc-arm-linux-gnueabihf

# On macOS with Homebrew
brew install arm-linux-gnueabihf-binutils
```

### Build Commands

#### Using the build script (recommended):
```bash
# Build for ARM TV box
./scripts/build-arm.sh --target armv7-unknown-linux-gnueabihf --features tv-box --release

# Build all ARM targets
./scripts/build-arm.sh --clean --package
```

#### Using Cargo directly:
```bash
# ARM TV box build
cargo build --target armv7-unknown-linux-gnueabihf --no-default-features --features "arm-tv-box" --release

# ARM IoT device build
cargo build --target armv7-unknown-linux-gnueabihf --no-default-features --features "arm-iot" --release

# Minimal embedded build
cargo build --target armv7-unknown-linux-gnueabihf --no-default-features --features "arm-embedded" --release
```

#### Using Docker:
```bash
# Build ARM binaries in Docker
docker build -f docker/Dockerfile.arm-cross --target builder .

# Run ARM container
docker build -f docker/Dockerfile.arm-cross --target runtime-arm .
```

## Configuration

### ARM-specific Configuration

Create an `application.yml` with ARM optimizations:

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
      local: false

# ARM-specific settings
arm:
  # Audio backend configuration
  audio:
    backend: "alsa"
    device: "default"
    sample_rate: 44100
    channels: 2
    buffer_size: 1024
    period_size: 256
    low_latency: false
  
  # Resource monitoring
  monitoring:
    enabled: true
    cpu_threshold: 80.0
    memory_threshold: 85.0
    temperature_threshold: 70.0
  
  # Performance optimizations
  optimizations:
    use_neon: true
    low_memory_mode: false
    adaptive_quality: true
    max_concurrent_tracks: 10
```

## Performance Characteristics

### Typical ARM Device Performance

| Device Type | CPU | RAM | Expected Performance |
|-------------|-----|-----|---------------------|
| ARM TV Box | Cortex-A53 1.5GHz | 2GB | 5-10 concurrent tracks |
| Raspberry Pi 4 | Cortex-A72 1.5GHz | 4GB | 10-15 concurrent tracks |
| Embedded ARM | Cortex-A7 1.0GHz | 512MB | 2-5 concurrent tracks |

### Memory Usage

- **Minimal build**: ~50MB RAM
- **IoT build**: ~80MB RAM  
- **TV box build**: ~120MB RAM

### Audio Latency

- **Standard mode**: 50-100ms
- **Low latency mode**: 20-50ms (higher CPU usage)

## Troubleshooting

### Common Issues

1. **Cross-compilation failures**
   - Ensure ARM toolchain is properly installed
   - Check `.cargo/config.toml` linker configuration
   - Verify target is installed: `rustup target list --installed`

2. **Audio playback issues**
   - Check ALSA configuration: `aplay -l`
   - Verify audio device permissions
   - Test with: `speaker-test -c 2 -t wav`

3. **Performance issues**
   - Enable ARM optimizations: `--features "arm-optimizations"`
   - Reduce concurrent tracks in configuration
   - Monitor resource usage with built-in monitoring

4. **Memory constraints**
   - Use `--features "low-memory"` for constrained devices
   - Reduce buffer sizes in configuration
   - Consider using musl target for smaller binaries

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug ./lavalink-rust --config application.yml
```

Monitor system resources:
```bash
# CPU and memory usage
htop

# Audio system status
cat /proc/asound/cards
```

## Deployment

### Systemd Service

Create `/etc/systemd/system/lavalink-rust.service`:

```ini
[Unit]
Description=Lavalink Rust Audio Server
After=network.target

[Service]
Type=simple
User=lavalink
Group=lavalink
WorkingDirectory=/opt/lavalink
ExecStart=/opt/lavalink/bin/lavalink-rust --config /opt/lavalink/config/application.yml
Restart=always
RestartSec=10

# ARM-specific optimizations
Environment=RUST_LOG=info
Environment=MALLOC_ARENA_MAX=2
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

### Docker Deployment

```bash
# Pull ARM image
docker pull lavalink-rust:arm-latest

# Run container
docker run -d \
  --name lavalink-rust \
  -p 2333:2333 \
  -v /path/to/config:/opt/lavalink/config \
  lavalink-rust:arm-latest
```

## Contributing

When contributing ARM-related code:

1. Test on actual ARM hardware when possible
2. Use conditional compilation for ARM-specific features
3. Consider memory and CPU constraints
4. Update documentation and tests
5. Verify cross-compilation works

## Support

For ARM/IoT specific issues:
- Check the troubleshooting section above
- Review system requirements
- Test with minimal configuration first
- Provide device specifications when reporting issues

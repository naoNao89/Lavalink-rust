---
description: How to run Lavalink Rust as a standalone binary
---

# Standalone Binary

## Prerequisites

Unlike Java Lavalink, Rust Lavalink does not require a Java Runtime Environment (JRE). However, you will need:

- **yt-dlp**: Required for YouTube, SoundCloud, and other audio source support
- **Operating System**: Linux (x64, ARM64), macOS (x64, ARM64), or Windows (x64)
- **Memory**: Minimum 256MB RAM (significantly less than Java Lavalink's 1GB+ requirement)

### Installing yt-dlp

```bash
# Using pip (recommended)
pip3 install yt-dlp

# Using package managers
# Ubuntu/Debian
sudo apt install yt-dlp

# macOS with Homebrew
brew install yt-dlp

# Verify installation
yt-dlp --version
```

## Installation

### Option 1: Download Pre-built Binary (Recommended)

Download the latest binary for your platform from [GitHub Releases](https://github.com/lavalink-devs/lavalink-rust/releases/latest):

```bash
# Linux x64
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-linux-x64
chmod +x lavalink-rust-linux-x64
mv lavalink-rust-linux-x64 lavalink-rust

# Linux ARM64
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-linux-arm64
chmod +x lavalink-rust-linux-arm64
mv lavalink-rust-linux-arm64 lavalink-rust

# macOS x64
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-macos-x64
chmod +x lavalink-rust-macos-x64
mv lavalink-rust-macos-x64 lavalink-rust

# macOS ARM64 (Apple Silicon)
wget https://github.com/lavalink-devs/lavalink-rust/releases/latest/download/lavalink-rust-macos-arm64
chmod +x lavalink-rust-macos-arm64
mv lavalink-rust-macos-arm64 lavalink-rust

# Windows x64
# Download lavalink-rust-windows-x64.exe from the releases page
```

### Option 2: Build from Source

If you prefer to build from source or need the latest development features:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/lavalink-devs/lavalink-rust.git
cd lavalink-rust
cargo build --release

# The binary will be available at target/release/lavalink-rust
cp target/release/lavalink-rust ./lavalink-rust
```

### Option 3: Install via Cargo

```bash
# Install directly from crates.io
cargo install lavalink-rust

# The binary will be installed to ~/.cargo/bin/lavalink-rust
```

## Directory Setup

Create a dedicated directory for your Lavalink installation:

```bash
# Create installation directory
mkdir -p ~/lavalink-rust
cd ~/lavalink-rust

# Copy the binary
cp /path/to/lavalink-rust ./

# Create necessary directories
mkdir -p logs plugins
```

## Configuration

Create an `application.yml` configuration file in your installation directory. See the [Configuration Guide](../configuration/index.md) for detailed options.

### Basic Configuration Example

```yaml title="application.yml"
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

## Running Lavalink

### Basic Usage

```bash
# Run with default configuration (application.yml in current directory)
./lavalink-rust

# Run with custom configuration file
./lavalink-rust --config /path/to/config.yml

# Run with verbose logging
./lavalink-rust --verbose

# Run with custom log level
RUST_LOG=debug ./lavalink-rust
```

### Command Line Options

```bash
# View all available options
./lavalink-rust --help

# Common options:
./lavalink-rust \
  --config /path/to/application.yml \
  --verbose

# Debug options:
./lavalink-rust --debug              # Enable debug logging
./lavalink-rust --trace              # Enable trace logging (very verbose)
./lavalink-rust --log-level warn     # Set custom log level
./lavalink-rust --json-logs          # Output logs in JSON format
./lavalink-rust --no-color           # Disable colored output
./lavalink-rust --timestamps         # Show timestamps in logs

# Combined example:
./lavalink-rust \
  --config /path/to/application.yml \
  --debug \
  --json-logs \
  --timestamps
```

### Environment Variables

You can also configure Lavalink using environment variables:

```bash
# Server configuration
export LAVALINK_SERVER_PORT=2333
export LAVALINK_SERVER_ADDRESS=0.0.0.0
export LAVALINK_SERVER_PASSWORD=youshallnotpass

# Logging
export RUST_LOG=info
export RUST_BACKTRACE=1

# Run with environment variables
./lavalink-rust
```

## Running in Background

### Using Screen (Simple)

```bash
# Start a new screen session
screen -S lavalink

# Run Lavalink
./lavalink-rust

# Detach from screen (Ctrl+A, then D)
# Reattach later with: screen -r lavalink
```

### Using nohup

```bash
# Run in background with nohup
nohup ./lavalink-rust > lavalink.log 2>&1 &

# Check if running
ps aux | grep lavalink-rust

# View logs
tail -f lavalink.log
```

### Using systemd (Recommended for Linux)

For production deployments, we recommend using systemd. See the [Systemd Service Guide](systemd.md) for detailed instructions.

## Verification

Once Lavalink is running, verify it's working correctly:

```bash
# Check server info
curl http://localhost:2333/v4/info

# Check server stats
curl http://localhost:2333/v4/stats

# Expected response should include version info and server statistics
```

## Performance Comparison

Rust Lavalink offers significant performance improvements over Java Lavalink:

| Metric | Java Lavalink | Rust Lavalink | Improvement |
|--------|---------------|---------------|-------------|
| **Startup Time** | 10-15 seconds | 2-5 seconds | 75% faster |
| **Memory Usage** | 1-2 GB | 256-512 MB | 50-75% less |
| **CPU Usage** | Variable (GC spikes) | Consistent | More predictable |
| **Binary Size** | 50+ MB (+ JRE) | 15-25 MB | Smaller footprint |

## Troubleshooting

### Common Issues

**Binary won't start:**
```bash
# Check if binary is executable
chmod +x lavalink-rust

# Check dependencies
ldd lavalink-rust  # Linux
otool -L lavalink-rust  # macOS
```

**yt-dlp not found:**
```bash
# Verify yt-dlp installation
which yt-dlp
yt-dlp --version

# Install if missing
pip3 install yt-dlp
```

**Permission denied:**
```bash
# Ensure proper permissions
chmod +x lavalink-rust
chown $USER:$USER lavalink-rust
```

For more troubleshooting help, see the [Troubleshooting Guide](troubleshooting.md).

## Next Steps

- **Production Deployment**: Set up [Systemd Service](systemd.md) for automatic startup
- **Containerization**: Use [Docker](docker.md) for isolated deployment
- **Configuration**: Explore [Configuration Options](../configuration/index.md)
- **API Usage**: Check the [REST API Documentation](../api/rest.md)

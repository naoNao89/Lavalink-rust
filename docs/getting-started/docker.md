---
description: How to run Lavalink Rust as a Docker container
---

# Docker

Docker images for Lavalink Rust are available on [GitHub Container Registry](https://github.com/lavalink-devs/lavalink-rust/pkgs/container/lavalink-rust).

## Prerequisites

Install [Docker](https://docs.docker.com/engine/install/) & [Docker Compose](https://docs.docker.com/compose/install/)

## Docker Image Variants

| Variant | Description | Base Image | Size | User | Group | Example |
|---------|-------------|------------|------|------|-------|---------|
| `latest` | Default variant, Debian-based | debian:bookworm-slim | ~150MB | 322 | 322 | `ghcr.io/lavalink-devs/lavalink-rust:latest` |
| `alpine` | Smaller image, Alpine-based | alpine:latest | ~80MB | 322 | 322 | `ghcr.io/lavalink-devs/lavalink-rust:alpine` |
| `distroless` | Minimal security-focused image | gcr.io/distroless/cc | ~50MB | 65534 | 65534 | `ghcr.io/lavalink-devs/lavalink-rust:distroless` |

## Quick Start with Docker Compose

Create a `compose.yml` file:

```yaml title="compose.yml"
services:
  lavalink-rust:
    # Use the latest stable version
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    container_name: lavalink-rust
    restart: unless-stopped
    environment:
      # Rust-specific environment variables
      - RUST_LOG=info
      - RUST_BACKTRACE=1
      # Lavalink server configuration
      - LAVALINK_SERVER_PASSWORD=youshallnotpass
      - LAVALINK_SERVER_PORT=2333
      - LAVALINK_SERVER_ADDRESS=0.0.0.0
    volumes:
      # Mount configuration file (optional if using environment variables)
      - ./application.yml:/app/application.yml:ro
      # Persist logs
      - ./logs:/app/logs
      # Optional: Mount plugins directory for future plugin support
      - ./plugins:/app/plugins:ro
    networks:
      - lavalink
    expose:
      # Lavalink API port
      - 2333
    ports:
      # Expose Lavalink to host (remove if only used by other containers)
      - "2333:2333"
      # Optional: Expose metrics port for monitoring
      - "9090:9090"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:2333/v4/info"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        limits:
          # Rust uses significantly less memory than Java Lavalink
          memory: 512M
          cpus: '1.0'
        reservations:
          memory: 256M
          cpus: '0.5'

networks:
  # Create a lavalink network for other containers to join
  lavalink:
    name: lavalink
```

### Start the Container

```bash
# Start in background
docker compose up -d

# View logs
docker compose logs -f lavalink-rust

# Stop the container
docker compose down
```

## Configuration Options

### Option 1: Environment Variables (Recommended)

Configure entirely through environment variables:

```yaml title="compose.yml (environment variables only)"
services:
  lavalink-rust:
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    container_name: lavalink-rust
    restart: unless-stopped
    environment:
      # Rust runtime
      - RUST_LOG=info
      - RUST_BACKTRACE=1
      
      # Server configuration
      - LAVALINK_SERVER_PORT=2333
      - LAVALINK_SERVER_ADDRESS=0.0.0.0
      - LAVALINK_SERVER_PASSWORD=youshallnotpass
      
      # Audio sources
      - LAVALINK_SERVER_SOURCES_YOUTUBE=true
      - LAVALINK_SERVER_SOURCES_SOUNDCLOUD=true
      - LAVALINK_SERVER_SOURCES_BANDCAMP=true
      - LAVALINK_SERVER_SOURCES_TWITCH=true
      - LAVALINK_SERVER_SOURCES_VIMEO=true
      - LAVALINK_SERVER_SOURCES_HTTP=true
      - LAVALINK_SERVER_SOURCES_LOCAL=false
      
      # Audio filters
      - LAVALINK_SERVER_FILTERS_VOLUME=true
      - LAVALINK_SERVER_FILTERS_EQUALIZER=true
      - LAVALINK_SERVER_FILTERS_KARAOKE=true
      - LAVALINK_SERVER_FILTERS_TIMESCALE=true
      - LAVALINK_SERVER_FILTERS_TREMOLO=true
      - LAVALINK_SERVER_FILTERS_VIBRATO=true
      - LAVALINK_SERVER_FILTERS_DISTORTION=true
      - LAVALINK_SERVER_FILTERS_ROTATION=true
      - LAVALINK_SERVER_FILTERS_CHANNELMIX=true
      - LAVALINK_SERVER_FILTERS_LOWPASS=true
    ports:
      - "2333:2333"
    networks:
      - lavalink

networks:
  lavalink:
    name: lavalink
```

### Option 2: Configuration File

Create an `application.yml` file and mount it:

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

## Running with Docker CLI

### Basic Usage

```bash
# Run with environment variables
docker run -d \
  --name lavalink-rust \
  --restart unless-stopped \
  -p 2333:2333 \
  -e RUST_LOG=info \
  -e LAVALINK_SERVER_PASSWORD=youshallnotpass \
  ghcr.io/lavalink-devs/lavalink-rust:latest

# Run with configuration file
docker run -d \
  --name lavalink-rust \
  --restart unless-stopped \
  -p 2333:2333 \
  -v $(pwd)/application.yml:/app/application.yml:ro \
  -v $(pwd)/logs:/app/logs \
  ghcr.io/lavalink-devs/lavalink-rust:latest
```

### Advanced Usage

```bash
# Run with custom memory limits and network
docker run -d \
  --name lavalink-rust \
  --restart unless-stopped \
  --memory=512m \
  --cpus=1.0 \
  --network lavalink \
  -p 2333:2333 \
  -p 9090:9090 \
  -e RUST_LOG=debug \
  -e RUST_BACKTRACE=1 \
  -e LAVALINK_SERVER_PASSWORD=youshallnotpass \
  -v $(pwd)/application.yml:/app/application.yml:ro \
  -v $(pwd)/logs:/app/logs \
  -v $(pwd)/plugins:/app/plugins:ro \
  ghcr.io/lavalink-devs/lavalink-rust:latest
```

## Image Variants Comparison

### Debian (Default)
- **Best for**: General use, development, debugging
- **Pros**: Full package manager, debugging tools available
- **Cons**: Larger size (~150MB)

```yaml
image: ghcr.io/lavalink-devs/lavalink-rust:latest
```

### Alpine
- **Best for**: Production with size constraints
- **Pros**: Smaller size (~80MB), security-focused
- **Cons**: Different libc (musl), potential compatibility issues

```yaml
image: ghcr.io/lavalink-devs/lavalink-rust:alpine
```

### Distroless
- **Best for**: High-security production environments
- **Pros**: Minimal attack surface (~50MB), no shell/package manager
- **Cons**: Difficult to debug, no shell access

```yaml
image: ghcr.io/lavalink-devs/lavalink-rust:distroless
```

## Networking

### Container-to-Container Communication

If your Discord bot also runs in Docker:

```yaml title="bot-compose.yml"
services:
  discord-bot:
    image: your-bot:latest
    environment:
      - LAVALINK_HOST=lavalink-rust  # Use service name as hostname
      - LAVALINK_PORT=2333
      - LAVALINK_PASSWORD=youshallnotpass
    networks:
      - lavalink  # Join the same network

  lavalink-rust:
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    # ... other configuration
    networks:
      - lavalink

networks:
  lavalink:
    name: lavalink
```

### External Access

```yaml
# Expose only to localhost
ports:
  - "127.0.0.1:2333:2333"

# Expose to all interfaces (be careful with security)
ports:
  - "2333:2333"
```

## Monitoring and Health Checks

### Built-in Health Check

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:2333/v4/info"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

### Metrics Endpoint

```yaml
ports:
  - "9090:9090"  # Prometheus metrics endpoint
```

## Performance Tuning

### Memory Optimization

```yaml
deploy:
  resources:
    limits:
      memory: 512M  # Much less than Java Lavalink's 2-6GB
    reservations:
      memory: 256M
```

### CPU Optimization

```yaml
deploy:
  resources:
    limits:
      cpus: '1.0'
    reservations:
      cpus: '0.5'
```

## Troubleshooting

### Common Issues

**Container won't start:**
```bash
# Check logs
docker logs lavalink-rust

# Check if port is already in use
netstat -tulpn | grep 2333
```

**Permission issues:**
```bash
# Fix volume permissions
sudo chown -R 322:322 ./logs ./plugins
```

**Health check failing:**
```bash
# Test manually
docker exec lavalink-rust curl -f http://localhost:2333/v4/info
```

### Debugging

```bash
# Access container shell (Debian/Alpine only)
docker exec -it lavalink-rust /bin/bash

# View real-time logs
docker logs -f lavalink-rust

# Check container stats
docker stats lavalink-rust
```

## Migration from Java Lavalink Docker

### Key Differences

| Aspect | Java Lavalink | Rust Lavalink |
|--------|---------------|---------------|
| **Base Image** | `eclipse-temurin:18-jre` | `debian:bookworm-slim` |
| **Memory Usage** | 2-6GB typical | 256-512MB typical |
| **Startup Time** | 10-15 seconds | 2-5 seconds |
| **Image Size** | 200-300MB | 50-150MB |
| **Dependencies** | JRE included | yt-dlp included |

### Migration Steps

1. **Update image reference:**
   ```yaml
   # Old
   image: ghcr.io/lavalink-devs/lavalink:4-alpine
   
   # New
   image: ghcr.io/lavalink-devs/lavalink-rust:alpine
   ```

2. **Reduce memory limits:**
   ```yaml
   deploy:
     resources:
       limits:
         memory: 512M  # Down from 2-6GB
   ```

3. **Update environment variables:**
   ```yaml
   environment:
     # Remove Java-specific variables
     # - _JAVA_OPTIONS=-Xmx6G
     
     # Add Rust-specific variables
     - RUST_LOG=info
     - RUST_BACKTRACE=1
   ```

4. **Test and deploy:**
   ```bash
   docker compose up -d
   docker compose logs -f lavalink-rust
   ```

## Next Steps

- **Production Setup**: Configure [monitoring and logging](../configuration/performance.md)
- **Security**: Set up [proper authentication](../configuration/index.md#security)
- **Scaling**: Learn about [load balancing](../advanced/architecture.md)
- **Integration**: Connect your [Discord bot clients](../api/rest.md)

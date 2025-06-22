---
description: Advanced Docker deployment guide for Lavalink Rust in production environments
---

# Docker Deployment Guide

This guide covers advanced Docker deployment strategies for Lavalink Rust in production environments, including orchestration, scaling, monitoring, and security considerations.

## Production Docker Setup

### Multi-Stage Production Dockerfile

```dockerfile
# Build stage
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libopus-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy dependency files for caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source and build
COPY src ./src
COPY tests ./tests
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libopus0 \
    libavcodec59 \
    libavformat59 \
    libavutil57 \
    libavfilter8 \
    libavdevice59 \
    python3 \
    python3-pip \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp
# Note: --break-system-packages is needed for Debian Bookworm (PEP 668)
RUN pip3 install --no-cache-dir --break-system-packages yt-dlp

# Create non-root user
RUN groupadd -g 322 lavalink && \
    useradd -r -u 322 -g lavalink -d /app -s /bin/bash lavalink

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/lavalink-rust /app/lavalink-rust
RUN chmod +x /app/lavalink-rust

# Create directories and set permissions
RUN mkdir -p /app/{logs,plugins,config} && \
    chown -R lavalink:lavalink /app

# Copy default configuration
COPY application.yml /app/config/application.yml.example

# Switch to non-root user
USER lavalink

# Expose ports
EXPOSE 2333 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:2333/v4/info || exit 1

# Use tini as init system
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/lavalink-rust", "--config", "/app/config/application.yml"]
```

### Production Docker Compose

```yaml
version: '3.8'

services:
  lavalink-rust:
    image: ghcr.io/lavalink-devs/lavalink-rust:latest
    container_name: lavalink-rust
    restart: unless-stopped
    
    # Resource limits (much lower than Java Lavalink)
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '2.0'
        reservations:
          memory: 512M
          cpus: '1.0'
    
    # Environment configuration
    environment:
      # Rust runtime optimization
      - RUST_LOG=info
      - RUST_BACKTRACE=0
      - MALLOC_ARENA_MAX=2
      
      # Lavalink configuration
      - LAVALINK_SERVER_PASSWORD=${LAVALINK_PASSWORD:-youshallnotpass}
      - LAVALINK_SERVER_PORT=2333
      - LAVALINK_SERVER_ADDRESS=0.0.0.0
      
      # Performance tuning
      - LAVALINK_SERVER_BUFFER_DURATION_MS=400
      - LAVALINK_SERVER_FRAME_BUFFER_DURATION_MS=5000
      
      # Metrics
      - METRICS_PROMETHEUS_ENABLED=true
      - METRICS_PROMETHEUS_ENDPOINT=/metrics
    
    # Volume mounts
    volumes:
      - ./config/application.yml:/app/config/application.yml:ro
      - ./logs:/app/logs
      - ./plugins:/app/plugins:ro
      - lavalink_cache:/app/cache
    
    # Network configuration
    networks:
      - lavalink
      - monitoring
    
    # Port exposure
    ports:
      - "2333:2333"
      - "9090:9090"
    
    # Health check
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:2333/v4/info"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    
    # Logging configuration
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "5"
        labels: "service=lavalink-rust"

  # Load balancer for multiple instances
  nginx:
    image: nginx:alpine
    container_name: lavalink-nginx
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    networks:
      - lavalink
    depends_on:
      - lavalink-rust

  # Monitoring stack
  prometheus:
    image: prom/prometheus:latest
    container_name: lavalink-prometheus
    restart: unless-stopped
    ports:
      - "9091:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./monitoring/rules:/etc/prometheus/rules:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
      - '--web.enable-admin-api'
    networks:
      - monitoring

  grafana:
    image: grafana/grafana:latest
    container_name: lavalink-grafana
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_INSTALL_PLUGINS=grafana-piechart-panel
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/provisioning:/etc/grafana/provisioning:ro
      - ./monitoring/grafana/dashboards:/var/lib/grafana/dashboards:ro
    networks:
      - monitoring

  # Log aggregation
  loki:
    image: grafana/loki:latest
    container_name: lavalink-loki
    restart: unless-stopped
    ports:
      - "3100:3100"
    volumes:
      - ./monitoring/loki.yml:/etc/loki/local-config.yaml:ro
      - loki_data:/loki
    networks:
      - monitoring

  promtail:
    image: grafana/promtail:latest
    container_name: lavalink-promtail
    restart: unless-stopped
    volumes:
      - ./monitoring/promtail.yml:/etc/promtail/config.yml:ro
      - /var/log:/var/log:ro
      - /var/lib/docker/containers:/var/lib/docker/containers:ro
    networks:
      - monitoring

networks:
  lavalink:
    name: lavalink
    driver: bridge
  monitoring:
    name: monitoring
    driver: bridge

volumes:
  lavalink_cache:
  prometheus_data:
  grafana_data:
  loki_data:
```

## Scaling and Load Balancing

### Horizontal Scaling

```yaml
# docker-compose.scale.yml
version: '3.8'

services:
  lavalink-rust-1:
    extends:
      file: docker-compose.yml
      service: lavalink-rust
    container_name: lavalink-rust-1
    environment:
      - LAVALINK_SERVER_PORT=2333
    ports:
      - "2333:2333"
      - "9090:9090"

  lavalink-rust-2:
    extends:
      file: docker-compose.yml
      service: lavalink-rust
    container_name: lavalink-rust-2
    environment:
      - LAVALINK_SERVER_PORT=2333
    ports:
      - "2334:2333"
      - "9091:9090"

  lavalink-rust-3:
    extends:
      file: docker-compose.yml
      service: lavalink-rust
    container_name: lavalink-rust-3
    environment:
      - LAVALINK_SERVER_PORT=2333
    ports:
      - "2335:2333"
      - "9092:9090"
```

### Nginx Load Balancer Configuration

```nginx
# nginx/nginx.conf
upstream lavalink_backend {
    least_conn;
    server lavalink-rust-1:2333 max_fails=3 fail_timeout=30s;
    server lavalink-rust-2:2333 max_fails=3 fail_timeout=30s;
    server lavalink-rust-3:2333 max_fails=3 fail_timeout=30s;
}

upstream lavalink_metrics {
    server lavalink-rust-1:9090;
    server lavalink-rust-2:9090;
    server lavalink-rust-3:9090;
}

server {
    listen 80;
    server_name lavalink.example.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name lavalink.example.com;
    
    # SSL configuration
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    
    # WebSocket support
    location / {
        proxy_pass http://lavalink_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
    
    # Metrics endpoint
    location /metrics {
        proxy_pass http://lavalink_metrics;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        
        # Restrict access to metrics
        allow 10.0.0.0/8;
        allow 172.16.0.0/12;
        allow 192.168.0.0/16;
        deny all;
    }
}
```

## Security Hardening

### Secure Docker Configuration

```yaml
# Security-focused docker-compose override
version: '3.8'

services:
  lavalink-rust:
    # Security options
    security_opt:
      - no-new-privileges:true
    
    # Read-only root filesystem
    read_only: true
    
    # Temporary filesystems for writable directories
    tmpfs:
      - /tmp:noexec,nosuid,size=100m
      - /app/logs:noexec,nosuid,size=500m
    
    # Capabilities
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE
    
    # User namespace remapping
    user: "322:322"
    
    # Resource limits
    ulimits:
      nproc: 65535
      nofile:
        soft: 65535
        hard: 65535
    
    # Additional environment variables
    environment:
      - RUST_BACKTRACE=0  # Disable backtraces in production
```

### Secrets Management

```yaml
# Using Docker secrets
version: '3.8'

services:
  lavalink-rust:
    secrets:
      - lavalink_password
      - ssl_cert
      - ssl_key
    environment:
      - LAVALINK_SERVER_PASSWORD_FILE=/run/secrets/lavalink_password

secrets:
  lavalink_password:
    file: ./secrets/lavalink_password.txt
  ssl_cert:
    file: ./secrets/ssl_cert.pem
  ssl_key:
    file: ./secrets/ssl_key.pem
```

## Monitoring Integration

### Prometheus Configuration

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  # Lavalink Rust instances
  - job_name: 'lavalink-rust'
    static_configs:
      - targets: 
        - 'lavalink-rust-1:9090'
        - 'lavalink-rust-2:9090'
        - 'lavalink-rust-3:9090'
    scrape_interval: 10s
    metrics_path: /metrics
    
  # Docker container metrics
  - job_name: 'docker'
    static_configs:
      - targets: ['cadvisor:8080']
    
  # System metrics
  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']
```

### Grafana Dashboard Configuration

```json
{
  "dashboard": {
    "title": "Lavalink Rust Production Dashboard",
    "panels": [
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "process_resident_memory_bytes{job=\"lavalink-rust\"} / 1024 / 1024",
            "legendFormat": "{{instance}} Memory (MB)"
          }
        ]
      },
      {
        "title": "Active Players",
        "type": "graph",
        "targets": [
          {
            "expr": "lavalink_players_total{job=\"lavalink-rust\"}",
            "legendFormat": "{{instance}} Players"
          }
        ]
      },
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(lavalink_http_requests_total{job=\"lavalink-rust\"}[5m])",
            "legendFormat": "{{instance}} Requests/sec"
          }
        ]
      }
    ]
  }
}
```

## Deployment Automation

### CI/CD Pipeline Example

```yaml
# .github/workflows/deploy.yml
name: Deploy Lavalink Rust

on:
  push:
    tags:
      - 'v*'

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: |
          docker build -t lavalink-rust:${{ github.ref_name }} .
          docker tag lavalink-rust:${{ github.ref_name }} lavalink-rust:latest
      
      - name: Push to registry
        run: |
          echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
          docker push lavalink-rust:${{ github.ref_name }}
          docker push lavalink-rust:latest
      
      - name: Deploy to production
        run: |
          ssh ${{ secrets.DEPLOY_USER }}@${{ secrets.DEPLOY_HOST }} '
            cd /opt/lavalink-rust &&
            docker-compose pull &&
            docker-compose up -d --remove-orphans &&
            docker system prune -f
          '
```

### Deployment Scripts

```bash
#!/bin/bash
# deploy.sh - Production deployment script

set -euo pipefail

COMPOSE_FILE="docker-compose.yml"
BACKUP_DIR="/opt/lavalink-rust/backups"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Create backup
echo "Creating backup..."
mkdir -p "$BACKUP_DIR"
docker-compose config > "$BACKUP_DIR/docker-compose-$TIMESTAMP.yml"

# Pull latest images
echo "Pulling latest images..."
docker-compose pull

# Rolling update
echo "Performing rolling update..."
for service in lavalink-rust-1 lavalink-rust-2 lavalink-rust-3; do
    echo "Updating $service..."
    docker-compose up -d --no-deps $service
    
    # Wait for health check
    echo "Waiting for $service to be healthy..."
    timeout 60 bash -c "until docker-compose ps $service | grep -q 'healthy'; do sleep 5; done"
    
    echo "$service updated successfully"
done

# Cleanup
echo "Cleaning up old images..."
docker system prune -f

echo "Deployment completed successfully!"
```

## Troubleshooting

### Common Issues

**1. Container Memory Issues:**
```bash
# Check memory usage
docker stats lavalink-rust

# Adjust memory limits
docker-compose up -d --scale lavalink-rust=1 \
  --memory=1g --memory-swap=1g
```

**2. Network Connectivity:**
```bash
# Test container networking
docker exec lavalink-rust curl -f http://localhost:2333/v4/info

# Check port bindings
docker port lavalink-rust
```

**3. Volume Permissions:**
```bash
# Fix volume permissions
sudo chown -R 322:322 ./logs ./plugins
sudo chmod -R 755 ./logs ./plugins
```

### Health Monitoring

```bash
# Check all container health
docker-compose ps

# View container logs
docker-compose logs -f lavalink-rust

# Monitor resource usage
docker stats --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}"

# Test load balancer
for i in {1..10}; do
  curl -s http://localhost/v4/info | jq .version
done
```

## Best Practices

### Resource Management

1. **Memory Limits**: Set appropriate memory limits (512MB-1GB)
2. **CPU Limits**: Use CPU quotas to prevent resource starvation
3. **Storage**: Use volumes for persistent data
4. **Network**: Isolate services using Docker networks

### Security

1. **Non-root User**: Always run containers as non-root
2. **Read-only Filesystem**: Use read-only root filesystem where possible
3. **Secrets Management**: Use Docker secrets for sensitive data
4. **Network Isolation**: Separate application and monitoring networks

### Monitoring

1. **Health Checks**: Implement comprehensive health checks
2. **Metrics Collection**: Enable Prometheus metrics
3. **Log Aggregation**: Use centralized logging
4. **Alerting**: Set up alerts for critical metrics

### Deployment

1. **Rolling Updates**: Use rolling updates for zero-downtime deployments
2. **Backup Strategy**: Always backup before deployments
3. **Testing**: Test deployments in staging environment
4. **Rollback Plan**: Have a rollback strategy ready

For more information, see:
- [Performance Tuning](performance.md)
- [Monitoring Setup](../configuration/monitoring.md)
- [Security Guide](../configuration/security.md)

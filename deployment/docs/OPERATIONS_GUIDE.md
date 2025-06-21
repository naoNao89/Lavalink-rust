# Lavalink Rust Operations Guide

## Overview

This guide covers the operational aspects of running Lavalink Rust in production, including deployment, monitoring, troubleshooting, and maintenance procedures.

## System Requirements

### Minimum Requirements
- **CPU**: 1 core (2+ cores recommended)
- **Memory**: 256MB RAM (512MB+ recommended)
- **Storage**: 1GB free space
- **OS**: Linux (Ubuntu 20.04+, CentOS 8+, or similar)
- **Network**: Stable internet connection for audio source access

### Dependencies
- **yt-dlp**: For YouTube/SoundCloud audio extraction
- **curl**: For health checks and API testing
- **systemd**: For service management (optional but recommended)

## Installation Methods

### Method 1: Automated Deployment Script
```bash
# Run as root
sudo ./deployment/scripts/deploy.sh
```

### Method 2: Manual Installation
```bash
# Create user and directories
sudo groupadd -g 322 lavalink
sudo useradd -r -u 322 -g lavalink lavalink
sudo mkdir -p /opt/lavalink-rust/{bin,config,logs,plugins,backups}

# Build and install binary
cargo build --release
sudo cp target/release/lavalink-rust /opt/lavalink-rust/bin/
sudo cp application.yml /opt/lavalink-rust/config/
sudo chown -R lavalink:lavalink /opt/lavalink-rust

# Install systemd service
sudo cp deployment/systemd/lavalink-rust.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable lavalink-rust
sudo systemctl start lavalink-rust
```

### Method 3: Docker Deployment
```bash
# Using docker-compose
cd deployment
docker-compose up -d

# Or standalone Docker
docker build -t lavalink-rust -f deployment/Dockerfile .
docker run -d --name lavalink-rust -p 2333:2333 -v ./application.yml:/app/application.yml lavalink-rust
```

## Service Management

### Systemd Commands
```bash
# Start service
sudo systemctl start lavalink-rust

# Stop service
sudo systemctl stop lavalink-rust

# Restart service
sudo systemctl restart lavalink-rust

# Check status
sudo systemctl status lavalink-rust

# Enable auto-start
sudo systemctl enable lavalink-rust

# View logs
sudo journalctl -u lavalink-rust -f
```

### Docker Commands
```bash
# Start containers
docker-compose up -d

# Stop containers
docker-compose down

# View logs
docker-compose logs -f lavalink-rust

# Restart specific service
docker-compose restart lavalink-rust
```

## Configuration Management

### Configuration Files
- **Primary Config**: `/opt/lavalink-rust/config/application.yml`
- **Service Config**: `/etc/systemd/system/lavalink-rust.service`
- **Docker Config**: `deployment/docker-compose.yml`

### Key Configuration Parameters
```yaml
server:
  port: 2333                    # API port
  address: "0.0.0.0"           # Bind address
  password: "youshallnotpass"   # Authentication password

lavalink:
  server:
    sources:
      youtube: true             # Enable YouTube
      soundcloud: true          # Enable SoundCloud
      http: true               # Enable HTTP URLs
    bufferDurationMs: 400       # Audio buffer size
    playerUpdateInterval: 5     # Player update frequency
```

### Environment Variables
```bash
# Logging level
RUST_LOG=info

# Enable backtraces for debugging
RUST_BACKTRACE=1

# Override config values
LAVALINK_SERVER_PASSWORD=newsecretpassword
LAVALINK_SERVER_PORT=2333
```

## Monitoring and Health Checks

### Health Check Endpoints
```bash
# Basic info
curl http://localhost:2333/v4/info

# Server statistics
curl http://localhost:2333/v4/stats

# Metrics (if enabled)
curl http://localhost:9090/metrics
```

### Key Metrics to Monitor
- **Memory Usage**: Should be <512MB under normal load
- **CPU Usage**: Should be <50% under normal load
- **Active Players**: Monitor for unusual spikes
- **WebSocket Connections**: Track connection stability
- **Track Loading Success Rate**: Should be >95%

### Log Locations
- **Systemd**: `journalctl -u lavalink-rust`
- **Docker**: `docker-compose logs lavalink-rust`
- **File Logs**: `/opt/lavalink-rust/logs/` (if configured)

## Troubleshooting

### Common Issues

#### Service Won't Start
```bash
# Check service status
sudo systemctl status lavalink-rust

# Check logs for errors
sudo journalctl -u lavalink-rust -n 50

# Verify binary permissions
ls -la /opt/lavalink-rust/bin/lavalink-rust

# Test configuration
/opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml --help
```

#### High Memory Usage
```bash
# Check current memory usage
ps aux | grep lavalink-rust

# Monitor memory over time
watch -n 5 'ps aux | grep lavalink-rust'

# Check for memory leaks in logs
sudo journalctl -u lavalink-rust | grep -i "memory\|oom"
```

#### Audio Playback Issues
```bash
# Test yt-dlp functionality
yt-dlp --version
yt-dlp -f bestaudio "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Check audio source configuration
curl -H "Authorization: youshallnotpass" \
     "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test"
```

#### WebSocket Connection Problems
```bash
# Check WebSocket endpoint
curl -H "Authorization: youshallnotpass" \
     -H "Upgrade: websocket" \
     -H "Connection: Upgrade" \
     "http://localhost:2333/v4/websocket"

# Monitor WebSocket connections
sudo netstat -tulpn | grep :2333
```

### Performance Tuning

#### Memory Optimization
- Monitor RSS memory usage
- Adjust buffer sizes if needed
- Consider memory limits in systemd service

#### CPU Optimization
- Monitor CPU usage patterns
- Adjust player update intervals
- Consider CPU limits in systemd service

#### Network Optimization
- Ensure stable internet connection
- Monitor bandwidth usage
- Consider CDN for audio sources if applicable

## Backup and Recovery

### Configuration Backup
```bash
# Backup configuration
sudo cp -r /opt/lavalink-rust/config /opt/lavalink-rust/backups/config-$(date +%Y%m%d)

# Backup service file
sudo cp /etc/systemd/system/lavalink-rust.service /opt/lavalink-rust/backups/
```

### Recovery Procedures
```bash
# Restore from backup
sudo systemctl stop lavalink-rust
sudo cp /opt/lavalink-rust/backups/config-YYYYMMDD/* /opt/lavalink-rust/config/
sudo systemctl start lavalink-rust
```

## Security Considerations

### Access Control
- Change default password in configuration
- Use firewall to restrict access to port 2333
- Consider using reverse proxy with SSL/TLS

### Updates and Patches
- Regularly update yt-dlp: `pip3 install --upgrade yt-dlp`
- Monitor for Rust security updates
- Keep system packages updated

### Monitoring Access
- Monitor failed authentication attempts
- Log all API access
- Set up alerts for unusual activity

## Maintenance Schedule

### Daily
- Check service status
- Monitor resource usage
- Review error logs

### Weekly
- Update yt-dlp
- Review performance metrics
- Check disk space

### Monthly
- Update system packages
- Review and rotate logs
- Test backup/recovery procedures
- Review security configurations

## Support and Escalation

### Log Collection for Support
```bash
# Collect system info
uname -a > support-info.txt
cat /etc/os-release >> support-info.txt

# Collect service logs
sudo journalctl -u lavalink-rust --since "1 hour ago" > lavalink-logs.txt

# Collect configuration (remove sensitive data)
sudo cp /opt/lavalink-rust/config/application.yml config-sanitized.yml
# Edit config-sanitized.yml to remove passwords

# Package for support
tar -czf lavalink-support-$(date +%Y%m%d-%H%M).tar.gz support-info.txt lavalink-logs.txt config-sanitized.yml
```

### Emergency Contacts
- **Primary**: System Administrator
- **Secondary**: Development Team
- **Escalation**: Infrastructure Team

---

*Last Updated: $(date +%Y-%m-%d)*
*Version: 1.0*

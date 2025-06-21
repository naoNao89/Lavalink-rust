---
description: How to run Lavalink Rust as a systemd service
---

# Systemd Service

Running Lavalink Rust as a systemd service is the recommended approach for production Linux deployments. This ensures automatic startup, restart on failure, and proper system integration.

## Prerequisites

- Linux system with systemd (most modern distributions)
- Lavalink Rust binary installed (see [Binary Installation](binary.md))
- yt-dlp installed and available in PATH
- Root or sudo access for service installation

## Quick Setup (Automated)

The easiest way to set up Lavalink Rust as a systemd service is using the provided deployment script:

```bash
# Clone the repository (if not already done)
git clone https://github.com/lavalink-devs/lavalink-rust.git
cd lavalink-rust

# Run the automated deployment script
sudo ./deployment/scripts/deploy.sh
```

This script will:
- Create the `lavalink` user and group
- Build the binary (if needed)
- Install files to `/opt/lavalink-rust/`
- Create and enable the systemd service
- Start the service

## Manual Setup

### Step 1: Create User and Directories

```bash
# Create dedicated user and group
sudo groupadd -g 322 lavalink
sudo useradd -r -u 322 -g lavalink -d /opt/lavalink-rust -s /bin/bash lavalink

# Create directory structure
sudo mkdir -p /opt/lavalink-rust/{bin,config,logs,plugins,backups}
sudo chown -R lavalink:lavalink /opt/lavalink-rust
sudo chmod 755 /opt/lavalink-rust
sudo chmod 750 /opt/lavalink-rust/{logs,backups}
```

### Step 2: Install Binary and Configuration

```bash
# Copy the binary
sudo cp lavalink-rust /opt/lavalink-rust/bin/
sudo chmod +x /opt/lavalink-rust/bin/lavalink-rust

# Copy configuration
sudo cp application.yml /opt/lavalink-rust/config/
sudo chown -R lavalink:lavalink /opt/lavalink-rust
```

### Step 3: Create Systemd Service File

Create the service file at `/etc/systemd/system/lavalink-rust.service`:

```ini title="/etc/systemd/system/lavalink-rust.service"
[Unit]
Description=Lavalink Rust Audio Node
Documentation=https://github.com/lavalink-devs/lavalink-rust
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
User=lavalink
Group=lavalink
WorkingDirectory=/opt/lavalink-rust
ExecStart=/opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=lavalink-rust

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/lavalink-rust/logs
ReadOnlyPaths=/opt/lavalink-rust

# Resource limits
LimitNOFILE=65536
MemoryMax=1G
CPUQuota=200%

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
Environment=PATH=/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=multi-user.target
```

### Step 4: Enable and Start Service

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable lavalink-rust

# Start the service
sudo systemctl start lavalink-rust

# Check status
sudo systemctl status lavalink-rust
```

## Service Management

### Basic Commands

```bash
# Start the service
sudo systemctl start lavalink-rust

# Stop the service
sudo systemctl stop lavalink-rust

# Restart the service
sudo systemctl restart lavalink-rust

# Reload configuration (graceful restart)
sudo systemctl reload lavalink-rust

# Check service status
sudo systemctl status lavalink-rust

# Enable auto-start on boot
sudo systemctl enable lavalink-rust

# Disable auto-start on boot
sudo systemctl disable lavalink-rust
```

### Monitoring and Logs

```bash
# View real-time logs
sudo journalctl -u lavalink-rust -f

# View recent logs
sudo journalctl -u lavalink-rust -n 100

# View logs since boot
sudo journalctl -u lavalink-rust -b

# View logs for specific time period
sudo journalctl -u lavalink-rust --since "2024-01-01 00:00:00" --until "2024-01-01 23:59:59"

# Export logs to file
sudo journalctl -u lavalink-rust > lavalink-logs.txt
```

## Configuration

### Service Configuration

The systemd service can be customized by editing `/etc/systemd/system/lavalink-rust.service`:

```ini
[Service]
# Change working directory
WorkingDirectory=/opt/lavalink-rust

# Modify command line arguments
ExecStart=/opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml --verbose

# Set environment variables
Environment=RUST_LOG=debug
Environment=LAVALINK_SERVER_PASSWORD=youshallnotpass
Environment=LAVALINK_SERVER_PORT=2333

# Resource limits
MemoryMax=512M
CPUQuota=100%
```

After making changes:
```bash
sudo systemctl daemon-reload
sudo systemctl restart lavalink-rust
```

### Environment Variables

You can set environment variables in the service file or create a separate environment file:

```bash
# Create environment file
sudo tee /opt/lavalink-rust/config/environment > /dev/null << 'EOF'
RUST_LOG=info
RUST_BACKTRACE=1
LAVALINK_SERVER_PASSWORD=youshallnotpass
LAVALINK_SERVER_PORT=2333
LAVALINK_SERVER_ADDRESS=0.0.0.0
EOF

# Update service file to use environment file
sudo sed -i '/\[Service\]/a EnvironmentFile=/opt/lavalink-rust/config/environment' /etc/systemd/system/lavalink-rust.service

# Reload and restart
sudo systemctl daemon-reload
sudo systemctl restart lavalink-rust
```

## Security Hardening

### Enhanced Security Settings

For production deployments, consider these additional security settings:

```ini title="/etc/systemd/system/lavalink-rust.service (security hardened)"
[Service]
# ... existing configuration ...

# Additional security settings
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictSUIDSGID=true
RemoveIPC=true
RestrictNamespaces=true

# Network restrictions (if not needed)
# PrivateNetwork=true
# IPAddressDeny=any
# IPAddressAllow=localhost

# File system restrictions
ProtectProc=invisible
ProcSubset=pid
PrivateUsers=true
ProtectHostname=true
ProtectClock=true
ProtectKernelLogs=true
SystemCallArchitectures=native

# Capabilities (remove if not needed)
CapabilityBoundingSet=
AmbientCapabilities=
```

### File Permissions

```bash
# Secure configuration files
sudo chmod 600 /opt/lavalink-rust/config/application.yml
sudo chmod 700 /opt/lavalink-rust/config/

# Secure service file
sudo chmod 644 /etc/systemd/system/lavalink-rust.service
sudo chown root:root /etc/systemd/system/lavalink-rust.service
```

## Troubleshooting

### Service Won't Start

```bash
# Check service status
sudo systemctl status lavalink-rust

# View detailed logs
sudo journalctl -u lavalink-rust -n 50

# Check if binary exists and is executable
ls -la /opt/lavalink-rust/bin/lavalink-rust

# Check configuration file
sudo -u lavalink /opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml --check-config
```

### Permission Issues

```bash
# Fix ownership
sudo chown -R lavalink:lavalink /opt/lavalink-rust

# Fix permissions
sudo chmod +x /opt/lavalink-rust/bin/lavalink-rust
sudo chmod 644 /opt/lavalink-rust/config/application.yml
```

### Port Already in Use

```bash
# Check what's using port 2333
sudo netstat -tulpn | grep 2333
sudo ss -tulpn | grep 2333

# Kill conflicting process
sudo systemctl stop other-lavalink-service
```

### Memory Issues

```bash
# Check memory usage
sudo systemctl show lavalink-rust --property=MemoryCurrent

# Adjust memory limits in service file
sudo systemctl edit lavalink-rust
```

Add:
```ini
[Service]
MemoryMax=1G
```

### Configuration Validation

```bash
# Test configuration without starting service
sudo -u lavalink /opt/lavalink-rust/bin/lavalink-rust --config /opt/lavalink-rust/config/application.yml --validate

# Check yt-dlp availability
sudo -u lavalink which yt-dlp
sudo -u lavalink yt-dlp --version
```

## Performance Monitoring

### Resource Usage

```bash
# Monitor resource usage
sudo systemctl show lavalink-rust --property=MemoryCurrent,CPUUsageNSec

# Detailed process information
sudo ps aux | grep lavalink-rust

# Memory usage over time
sudo journalctl -u lavalink-rust | grep -i memory
```

### Service Health

```bash
# Check if service is active
sudo systemctl is-active lavalink-rust

# Check if service is enabled
sudo systemctl is-enabled lavalink-rust

# Service uptime
sudo systemctl show lavalink-rust --property=ActiveEnterTimestamp
```

## Backup and Recovery

### Configuration Backup

```bash
# Create backup script
sudo tee /opt/lavalink-rust/backup.sh > /dev/null << 'EOF'
#!/bin/bash
BACKUP_DIR="/opt/lavalink-rust/backups/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
cp -r /opt/lavalink-rust/config "$BACKUP_DIR/"
cp /etc/systemd/system/lavalink-rust.service "$BACKUP_DIR/"
echo "Backup created at $BACKUP_DIR"
EOF

sudo chmod +x /opt/lavalink-rust/backup.sh
```

### Automated Backups

```bash
# Create systemd timer for daily backups
sudo tee /etc/systemd/system/lavalink-backup.service > /dev/null << 'EOF'
[Unit]
Description=Backup Lavalink Rust Configuration

[Service]
Type=oneshot
User=lavalink
ExecStart=/opt/lavalink-rust/backup.sh
EOF

sudo tee /etc/systemd/system/lavalink-backup.timer > /dev/null << 'EOF'
[Unit]
Description=Daily Lavalink Backup
Requires=lavalink-backup.service

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Enable backup timer
sudo systemctl enable lavalink-backup.timer
sudo systemctl start lavalink-backup.timer
```

## Migration from Java Lavalink

### Service File Differences

| Aspect | Java Lavalink | Rust Lavalink |
|--------|---------------|---------------|
| **Binary Path** | `java -jar Lavalink.jar` | `/opt/lavalink-rust/bin/lavalink-rust` |
| **Memory Usage** | 2-6GB typical | 256-512MB typical |
| **Startup Time** | 10-15 seconds | 2-5 seconds |
| **Dependencies** | JRE required | Native binary |

### Migration Steps

1. **Stop Java service:**
   ```bash
   sudo systemctl stop lavalink
   sudo systemctl disable lavalink
   ```

2. **Install Rust version:**
   ```bash
   sudo ./deployment/scripts/deploy.sh
   ```

3. **Migrate configuration:**
   ```bash
   # Your existing application.yml should work without changes
   sudo cp /opt/lavalink/application.yml /opt/lavalink-rust/config/
   ```

4. **Start Rust service:**
   ```bash
   sudo systemctl start lavalink-rust
   sudo systemctl enable lavalink-rust
   ```

## Next Steps

- **Monitoring**: Set up [performance monitoring](../configuration/performance.md)
- **Security**: Configure [authentication and SSL](../configuration/index.md#security)
- **Scaling**: Learn about [load balancing](../advanced/architecture.md)
- **Maintenance**: Set up [log rotation and cleanup](../advanced/maintenance.md)

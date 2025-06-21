---
description: Comprehensive operational procedures and best practices for Lavalink Rust
---

# Operational Procedures

This guide covers day-to-day operational procedures, maintenance tasks, incident response, and best practices for running Lavalink Rust in production environments.

## Daily Operations

### Health Check Procedures

**Morning Health Check Routine:**
```bash
#!/bin/bash
# daily-health-check.sh

echo "=== Lavalink Rust Daily Health Check ==="
echo "Date: $(date)"
echo

# 1. Service Status
echo "1. Service Status:"
systemctl is-active lavalink-rust
systemctl status lavalink-rust --no-pager -l

# 2. Resource Usage
echo -e "\n2. Resource Usage:"
ps aux | grep lavalink-rust | grep -v grep
free -h
df -h /opt/lavalink-rust

# 3. API Health
echo -e "\n3. API Health:"
curl -s http://localhost:2333/v4/info | jq '.version, .buildTime'
curl -s http://localhost:2333/v4/stats | jq '.memory, .players, .playingPlayers'

# 4. Log Errors (last 24 hours)
echo -e "\n4. Recent Errors:"
journalctl -u lavalink-rust --since "24 hours ago" --grep="ERROR|WARN" --no-pager | tail -10

# 5. Network Connectivity
echo -e "\n5. Network Status:"
netstat -tulpn | grep :2333
ss -tulpn | grep :2333

echo -e "\n=== Health Check Complete ==="
```

**Automated Health Monitoring:**
```bash
#!/bin/bash
# health-monitor.sh - Run every 5 minutes via cron

HEALTH_URL="http://localhost:2333/v4/info"
ALERT_EMAIL="ops@example.com"
LOG_FILE="/var/log/lavalink-health.log"

# Check if service is responding
if ! curl -sf "$HEALTH_URL" > /dev/null; then
    echo "$(date): ALERT - Lavalink Rust not responding" >> "$LOG_FILE"
    echo "Lavalink Rust health check failed at $(date)" | mail -s "ALERT: Lavalink Down" "$ALERT_EMAIL"
    exit 1
fi

# Check memory usage
MEMORY_MB=$(ps aux | grep lavalink-rust | grep -v grep | awk '{print $6}')
if [ "$MEMORY_MB" -gt 1048576 ]; then  # 1GB in KB
    echo "$(date): WARNING - High memory usage: ${MEMORY_MB}KB" >> "$LOG_FILE"
fi

echo "$(date): Health check passed" >> "$LOG_FILE"
```

### Performance Monitoring

**Resource Usage Tracking:**
```bash
#!/bin/bash
# performance-monitor.sh

METRICS_FILE="/var/log/lavalink-metrics.csv"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# Get process info
PID=$(pgrep lavalink-rust)
if [ -z "$PID" ]; then
    echo "$TIMESTAMP,SERVICE_DOWN,0,0,0,0" >> "$METRICS_FILE"
    exit 1
fi

# Collect metrics
CPU_PERCENT=$(ps -p "$PID" -o %cpu --no-headers)
MEMORY_KB=$(ps -p "$PID" -o rss --no-headers)
MEMORY_MB=$((MEMORY_KB / 1024))

# Network connections
CONNECTIONS=$(netstat -an | grep :2333 | grep ESTABLISHED | wc -l)

# API response time
RESPONSE_TIME=$(curl -w "%{time_total}" -s -o /dev/null http://localhost:2333/v4/info)

# Log metrics
echo "$TIMESTAMP,$CPU_PERCENT,$MEMORY_MB,$CONNECTIONS,$RESPONSE_TIME" >> "$METRICS_FILE"

# Check thresholds
if (( $(echo "$CPU_PERCENT > 80" | bc -l) )); then
    echo "$(date): WARNING - High CPU usage: $CPU_PERCENT%" | logger -t lavalink-monitor
fi

if [ "$MEMORY_MB" -gt 800 ]; then
    echo "$(date): WARNING - High memory usage: ${MEMORY_MB}MB" | logger -t lavalink-monitor
fi
```

## Maintenance Procedures

### Routine Maintenance Tasks

**Weekly Maintenance Checklist:**
```bash
#!/bin/bash
# weekly-maintenance.sh

echo "=== Weekly Maintenance - $(date) ==="

# 1. Update yt-dlp
echo "1. Updating yt-dlp..."
pip3 install --upgrade yt-dlp
yt-dlp --version

# 2. Log rotation
echo "2. Rotating logs..."
journalctl --vacuum-time=30d
find /opt/lavalink-rust/logs -name "*.log" -mtime +30 -delete

# 3. System updates
echo "3. Checking system updates..."
apt list --upgradable 2>/dev/null | grep -v "WARNING"

# 4. Disk cleanup
echo "4. Cleaning up disk space..."
docker system prune -f
apt autoremove -y
apt autoclean

# 5. Configuration backup
echo "5. Backing up configuration..."
BACKUP_DIR="/opt/lavalink-rust/backups/weekly-$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"
cp -r /opt/lavalink-rust/config "$BACKUP_DIR/"
cp /etc/systemd/system/lavalink-rust.service "$BACKUP_DIR/"

# 6. Performance report
echo "6. Generating performance report..."
./performance-report.sh > "$BACKUP_DIR/performance-report.txt"

echo "=== Weekly Maintenance Complete ==="
```

**Monthly Maintenance Tasks:**
```bash
#!/bin/bash
# monthly-maintenance.sh

echo "=== Monthly Maintenance - $(date) ==="

# 1. Full system backup
echo "1. Creating full system backup..."
tar -czf "/backup/lavalink-full-$(date +%Y%m%d).tar.gz" \
    /opt/lavalink-rust \
    /etc/systemd/system/lavalink-rust.service

# 2. Security updates
echo "2. Applying security updates..."
apt update && apt upgrade -y

# 3. Certificate renewal (if using SSL)
echo "3. Checking SSL certificates..."
if [ -f "/etc/ssl/certs/lavalink.crt" ]; then
    openssl x509 -in /etc/ssl/certs/lavalink.crt -noout -dates
fi

# 4. Performance analysis
echo "4. Running performance analysis..."
./performance-analysis.sh

# 5. Capacity planning
echo "5. Capacity planning review..."
./capacity-planning.sh

echo "=== Monthly Maintenance Complete ==="
```

### Update Procedures

**Binary Update Process:**
```bash
#!/bin/bash
# update-lavalink.sh

set -euo pipefail

NEW_VERSION="$1"
BACKUP_DIR="/opt/lavalink-rust/backups/update-$(date +%Y%m%d-%H%M%S)"
BINARY_PATH="/opt/lavalink-rust/bin/lavalink-rust"

echo "=== Updating Lavalink Rust to version $NEW_VERSION ==="

# 1. Pre-update backup
echo "1. Creating backup..."
mkdir -p "$BACKUP_DIR"
cp "$BINARY_PATH" "$BACKUP_DIR/lavalink-rust.backup"
cp -r /opt/lavalink-rust/config "$BACKUP_DIR/"

# 2. Download new version
echo "2. Downloading new version..."
wget -O "/tmp/lavalink-rust-$NEW_VERSION" \
    "https://github.com/lavalink-devs/lavalink-rust/releases/download/v$NEW_VERSION/lavalink-rust-linux-x64"
chmod +x "/tmp/lavalink-rust-$NEW_VERSION"

# 3. Validate new binary
echo "3. Validating new binary..."
if ! "/tmp/lavalink-rust-$NEW_VERSION" --version; then
    echo "ERROR: New binary validation failed"
    exit 1
fi

# 4. Stop service
echo "4. Stopping service..."
systemctl stop lavalink-rust

# 5. Replace binary
echo "5. Replacing binary..."
cp "/tmp/lavalink-rust-$NEW_VERSION" "$BINARY_PATH"
chown lavalink:lavalink "$BINARY_PATH"

# 6. Start service
echo "6. Starting service..."
systemctl start lavalink-rust

# 7. Verify update
echo "7. Verifying update..."
sleep 10
if systemctl is-active lavalink-rust && curl -sf http://localhost:2333/v4/info; then
    echo "Update successful!"
    rm "/tmp/lavalink-rust-$NEW_VERSION"
else
    echo "Update failed, rolling back..."
    systemctl stop lavalink-rust
    cp "$BACKUP_DIR/lavalink-rust.backup" "$BINARY_PATH"
    systemctl start lavalink-rust
    exit 1
fi

echo "=== Update Complete ==="
```

**Configuration Update Process:**
```bash
#!/bin/bash
# update-config.sh

CONFIG_FILE="/opt/lavalink-rust/config/application.yml"
BACKUP_DIR="/opt/lavalink-rust/backups/config-$(date +%Y%m%d-%H%M%S)"

echo "=== Updating Configuration ==="

# 1. Backup current config
echo "1. Backing up current configuration..."
mkdir -p "$BACKUP_DIR"
cp "$CONFIG_FILE" "$BACKUP_DIR/application.yml.backup"

# 2. Validate new configuration
echo "2. Validating new configuration..."
if ! /opt/lavalink-rust/bin/lavalink-rust --config "$1" --validate; then
    echo "ERROR: Configuration validation failed"
    exit 1
fi

# 3. Apply new configuration
echo "3. Applying new configuration..."
cp "$1" "$CONFIG_FILE"
chown lavalink:lavalink "$CONFIG_FILE"

# 4. Reload service
echo "4. Reloading service..."
systemctl reload lavalink-rust

# 5. Verify configuration
echo "5. Verifying configuration..."
sleep 5
if systemctl is-active lavalink-rust && curl -sf http://localhost:2333/v4/info; then
    echo "Configuration update successful!"
else
    echo "Configuration update failed, rolling back..."
    cp "$BACKUP_DIR/application.yml.backup" "$CONFIG_FILE"
    systemctl reload lavalink-rust
    exit 1
fi

echo "=== Configuration Update Complete ==="
```

## Incident Response

### Incident Response Procedures

**Service Down Response:**
```bash
#!/bin/bash
# incident-service-down.sh

echo "=== INCIDENT: Service Down Response ==="
echo "Incident started at: $(date)"

# 1. Immediate assessment
echo "1. Assessing service status..."
systemctl status lavalink-rust --no-pager
journalctl -u lavalink-rust --since "10 minutes ago" --no-pager

# 2. Quick restart attempt
echo "2. Attempting service restart..."
systemctl restart lavalink-rust
sleep 10

# 3. Verify restart
if systemctl is-active lavalink-rust; then
    echo "Service restart successful"
    curl http://localhost:2333/v4/info
    echo "Incident resolved at: $(date)"
    exit 0
fi

# 4. Deeper investigation
echo "3. Service restart failed, investigating..."
journalctl -u lavalink-rust --since "1 hour ago" --no-pager > /tmp/incident-logs.txt

# 5. Check system resources
echo "4. Checking system resources..."
free -h
df -h
ps aux | head -20

# 6. Check dependencies
echo "5. Checking dependencies..."
which yt-dlp
yt-dlp --version
netstat -tulpn | grep :2333

# 7. Escalate if needed
echo "6. Manual intervention required"
echo "Logs saved to: /tmp/incident-logs.txt"
echo "Contact on-call engineer"
```

**High Memory Usage Response:**
```bash
#!/bin/bash
# incident-high-memory.sh

MEMORY_THRESHOLD_MB=800
CURRENT_MEMORY_MB=$(ps aux | grep lavalink-rust | grep -v grep | awk '{print $6/1024}' | cut -d. -f1)

if [ "$CURRENT_MEMORY_MB" -gt "$MEMORY_THRESHOLD_MB" ]; then
    echo "=== INCIDENT: High Memory Usage ==="
    echo "Current memory usage: ${CURRENT_MEMORY_MB}MB"
    echo "Threshold: ${MEMORY_THRESHOLD_MB}MB"
    
    # 1. Collect memory information
    echo "1. Collecting memory information..."
    ps aux | grep lavalink-rust
    cat /proc/$(pgrep lavalink-rust)/status | grep -E "(VmRSS|VmSize|VmPeak)"
    
    # 2. Check for memory leaks
    echo "2. Checking for potential memory leaks..."
    curl -s http://localhost:2333/v4/stats | jq '.memory'
    
    # 3. Graceful restart if critical
    if [ "$CURRENT_MEMORY_MB" -gt 1000 ]; then
        echo "3. Critical memory usage, performing graceful restart..."
        systemctl restart lavalink-rust
    fi
    
    echo "=== Memory incident logged ==="
fi
```

### Disaster Recovery

**Complete System Recovery:**
```bash
#!/bin/bash
# disaster-recovery.sh

BACKUP_DATE="$1"
BACKUP_PATH="/backup/lavalink-full-$BACKUP_DATE.tar.gz"

echo "=== Disaster Recovery Procedure ==="
echo "Restoring from backup: $BACKUP_DATE"

# 1. Stop service
echo "1. Stopping service..."
systemctl stop lavalink-rust

# 2. Backup current state
echo "2. Backing up current state..."
tar -czf "/tmp/pre-recovery-$(date +%Y%m%d-%H%M%S).tar.gz" /opt/lavalink-rust

# 3. Restore from backup
echo "3. Restoring from backup..."
cd /
tar -xzf "$BACKUP_PATH"

# 4. Fix permissions
echo "4. Fixing permissions..."
chown -R lavalink:lavalink /opt/lavalink-rust
chmod +x /opt/lavalink-rust/bin/lavalink-rust

# 5. Reload systemd
echo "5. Reloading systemd..."
systemctl daemon-reload

# 6. Start service
echo "6. Starting service..."
systemctl start lavalink-rust

# 7. Verify recovery
echo "7. Verifying recovery..."
sleep 15
if systemctl is-active lavalink-rust && curl -sf http://localhost:2333/v4/info; then
    echo "Disaster recovery successful!"
else
    echo "Disaster recovery failed!"
    exit 1
fi

echo "=== Disaster Recovery Complete ==="
```

## Capacity Planning

### Resource Monitoring

**Capacity Analysis Script:**
```bash
#!/bin/bash
# capacity-analysis.sh

echo "=== Capacity Analysis Report ==="
echo "Generated: $(date)"
echo

# 1. Current resource usage
echo "1. Current Resource Usage:"
echo "Memory: $(ps aux | grep lavalink-rust | grep -v grep | awk '{print $6/1024}' | cut -d. -f1)MB"
echo "CPU: $(ps aux | grep lavalink-rust | grep -v grep | awk '{print $3}')%"
echo "Connections: $(netstat -an | grep :2333 | grep ESTABLISHED | wc -l)"

# 2. Historical trends (last 30 days)
echo -e "\n2. Historical Trends (30 days):"
if [ -f "/var/log/lavalink-metrics.csv" ]; then
    echo "Average Memory: $(tail -n 8640 /var/log/lavalink-metrics.csv | awk -F, '{sum+=$3} END {print sum/NR}')MB"
    echo "Peak Memory: $(tail -n 8640 /var/log/lavalink-metrics.csv | awk -F, 'BEGIN{max=0} {if($3>max) max=$3} END {print max}')MB"
    echo "Average CPU: $(tail -n 8640 /var/log/lavalink-metrics.csv | awk -F, '{sum+=$2} END {print sum/NR}')%"
fi

# 3. Growth projections
echo -e "\n3. Growth Projections:"
CURRENT_PLAYERS=$(curl -s http://localhost:2333/v4/stats | jq '.players')
echo "Current Players: $CURRENT_PLAYERS"
echo "Estimated capacity: $((CURRENT_PLAYERS * 3)) players (3x current load)"

# 4. Recommendations
echo -e "\n4. Recommendations:"
MEMORY_MB=$(ps aux | grep lavalink-rust | grep -v grep | awk '{print $6/1024}' | cut -d. -f1)
if [ "$MEMORY_MB" -gt 600 ]; then
    echo "- Consider memory optimization or scaling"
fi

if [ "$CURRENT_PLAYERS" -gt 500 ]; then
    echo "- Consider horizontal scaling"
fi

echo -e "\n=== Analysis Complete ==="
```

## Security Operations

### Security Monitoring

**Security Audit Script:**
```bash
#!/bin/bash
# security-audit.sh

echo "=== Security Audit ==="
echo "Date: $(date)"

# 1. Check file permissions
echo "1. File Permissions:"
ls -la /opt/lavalink-rust/bin/lavalink-rust
ls -la /opt/lavalink-rust/config/application.yml

# 2. Check for unauthorized access attempts
echo -e "\n2. Failed Authentication Attempts:"
journalctl -u lavalink-rust --since "24 hours ago" | grep -i "unauthorized\|forbidden\|401\|403" | tail -10

# 3. Check network exposure
echo -e "\n3. Network Exposure:"
netstat -tulpn | grep :2333
ss -tulpn | grep :2333

# 4. Check SSL/TLS configuration
echo -e "\n4. SSL/TLS Status:"
if curl -k https://localhost:2333/v4/info 2>/dev/null; then
    echo "HTTPS enabled"
    openssl s_client -connect localhost:2333 -servername localhost < /dev/null 2>/dev/null | openssl x509 -noout -dates
else
    echo "HTTPS not configured"
fi

# 5. Check for security updates
echo -e "\n5. Security Updates:"
apt list --upgradable 2>/dev/null | grep -i security

echo -e "\n=== Security Audit Complete ==="
```

## Best Practices

### Operational Excellence

1. **Automation**: Automate routine tasks and monitoring
2. **Documentation**: Keep operational procedures up-to-date
3. **Testing**: Regularly test backup and recovery procedures
4. **Monitoring**: Implement comprehensive monitoring and alerting
5. **Security**: Follow security best practices and regular audits

### Performance Optimization

1. **Resource Monitoring**: Continuously monitor resource usage
2. **Capacity Planning**: Plan for growth and scaling needs
3. **Optimization**: Regularly review and optimize configurations
4. **Benchmarking**: Establish performance baselines and track changes

### Incident Management

1. **Response Plans**: Have clear incident response procedures
2. **Escalation**: Define escalation paths and contact information
3. **Post-Mortems**: Conduct post-incident reviews and improvements
4. **Communication**: Maintain clear communication during incidents

### Change Management

1. **Testing**: Test all changes in staging environments
2. **Rollback Plans**: Always have rollback procedures ready
3. **Documentation**: Document all changes and their impact
4. **Approval Process**: Follow change approval processes

For more information, see:
- [Monitoring Setup](../configuration/monitoring.md)
- [Performance Tuning](performance.md)
- [Docker Deployment](docker-deployment.md)
- [Troubleshooting Guide](../getting-started/troubleshooting.md)

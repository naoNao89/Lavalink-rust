# Lavalink Rust Team Training Guide

## Training Overview

This guide provides comprehensive training materials for team members transitioning from Java Lavalink to Rust Lavalink. It covers key differences, new procedures, and hands-on exercises.

## Learning Objectives

By the end of this training, team members will be able to:
- Deploy and manage Lavalink Rust in production
- Troubleshoot common issues specific to the Rust implementation
- Monitor performance and identify optimization opportunities
- Execute rollback procedures if needed
- Understand the key differences from Java Lavalink

## Module 1: Key Differences from Java Lavalink

### Architecture Changes
| Aspect | Java Lavalink | Rust Lavalink |
|--------|---------------|---------------|
| **Runtime** | JRE 17+ required | Native binary, no runtime |
| **Memory Usage** | ~1GB typical | ~256-512MB typical |
| **Startup Time** | 10-15 seconds | 2-3 seconds |
| **Configuration** | Same application.yml | Same application.yml |
| **API Compatibility** | Full v4 API | 90%+ v4 API compatible |

### Performance Improvements
- **50% less memory usage** compared to Java version
- **Faster startup times** due to native compilation
- **No garbage collection pauses** - more predictable performance
- **Better resource utilization** under high load

### Feature Differences
✅ **Supported**: YouTube, SoundCloud, HTTP, Bandcamp, Vimeo, Twitch
❌ **Not Supported**: Spotify, Apple Music, Deezer (use YouTube fallback)
🔄 **In Progress**: Plugin system (Java plugins incompatible)

## Module 2: Installation and Deployment

### Hands-on Exercise 1: Manual Installation
```bash
# Step 1: Create user and directories
sudo groupadd -g 322 lavalink
sudo useradd -r -u 322 -g lavalink lavalink
sudo mkdir -p /opt/lavalink-rust/{bin,config,logs,plugins,backups}

# Step 2: Build binary (in project directory)
cargo build --release

# Step 3: Install files
sudo cp target/release/lavalink-rust /opt/lavalink-rust/bin/
sudo cp application.yml /opt/lavalink-rust/config/
sudo chown -R lavalink:lavalink /opt/lavalink-rust

# Step 4: Install systemd service
sudo cp deployment/systemd/lavalink-rust.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable lavalink-rust
sudo systemctl start lavalink-rust

# Step 5: Verify installation
curl http://localhost:2333/v4/info
```

### Hands-on Exercise 2: Automated Deployment
```bash
# Run the deployment script
sudo ./deployment/scripts/deploy.sh

# Verify deployment
systemctl status lavalink-rust
curl http://localhost:2333/v4/info
```

### Hands-on Exercise 3: Docker Deployment
```bash
# Using docker-compose
cd deployment
docker-compose up -d

# Verify containers
docker-compose ps
docker-compose logs lavalink-rust

# Test API
curl http://localhost:2333/v4/info
```

## Module 3: Configuration Management

### Configuration File Structure
```yaml
server:
  port: 2333
  address: "0.0.0.0"
  password: "youshallnotpass"

lavalink:
  server:
    password: "youshallnotpass"
    sources:
      youtube: true
      soundcloud: true
      http: true
    bufferDurationMs: 400
    playerUpdateInterval: 5
```

### Environment Variables
```bash
# Logging configuration
export RUST_LOG=info              # debug, info, warn, error
export RUST_BACKTRACE=1           # Enable stack traces

# Override config values
export LAVALINK_SERVER_PASSWORD=newsecret
export LAVALINK_SERVER_PORT=2333
```

### Hands-on Exercise 4: Configuration Changes
1. Change the server password
2. Modify the port number
3. Enable/disable audio sources
4. Restart service and verify changes

## Module 4: Monitoring and Troubleshooting

### Essential Commands
```bash
# Service management
sudo systemctl status lavalink-rust
sudo systemctl restart lavalink-rust
sudo journalctl -u lavalink-rust -f

# Health checks
curl http://localhost:2333/v4/info
curl http://localhost:2333/v4/stats

# Resource monitoring
ps aux | grep lavalink-rust
top -p $(pgrep lavalink-rust)
```

### Common Issues and Solutions

#### Issue 1: Service Won't Start
**Symptoms**: Service fails to start, exits immediately
**Diagnosis**:
```bash
sudo systemctl status lavalink-rust
sudo journalctl -u lavalink-rust -n 20
```
**Common Causes**:
- Configuration file syntax errors
- Port already in use
- Missing dependencies (yt-dlp)
- Permission issues

#### Issue 2: High Memory Usage
**Symptoms**: Memory usage above 512MB
**Diagnosis**:
```bash
ps aux | grep lavalink-rust
cat /proc/$(pgrep lavalink-rust)/status | grep VmRSS
```
**Solutions**:
- Check for memory leaks in logs
- Reduce buffer sizes
- Restart service if needed

#### Issue 3: Audio Playback Failures
**Symptoms**: Tracks fail to load or play
**Diagnosis**:
```bash
# Test yt-dlp directly
yt-dlp -f bestaudio "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Test API endpoint
curl -H "Authorization: youshallnotpass" \
     "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test"
```

### Hands-on Exercise 5: Troubleshooting Scenarios
Practice diagnosing and fixing these simulated issues:
1. Service fails to start due to port conflict
2. High memory usage investigation
3. Audio source configuration problems
4. WebSocket connection issues

## Module 5: Performance Monitoring

### Key Metrics to Track
- **Memory Usage**: Target <512MB
- **CPU Usage**: Target <50%
- **Active Players**: Monitor for spikes
- **Track Loading Success Rate**: Target >95%
- **WebSocket Connections**: Monitor stability

### Monitoring Tools
```bash
# Built-in metrics (if enabled)
curl http://localhost:9090/metrics

# System monitoring
htop
iotop
netstat -tulpn | grep :2333
```

### Setting Up Prometheus + Grafana
```bash
# Start monitoring stack
cd deployment
docker-compose up -d prometheus grafana

# Access Grafana
open http://localhost:3000
# Login: admin/admin
```

## Module 6: Backup and Recovery

### Backup Procedures
```bash
# Configuration backup
sudo cp -r /opt/lavalink-rust/config /opt/lavalink-rust/backups/config-$(date +%Y%m%d)

# Service file backup
sudo cp /etc/systemd/system/lavalink-rust.service /opt/lavalink-rust/backups/

# Create full backup
tar -czf lavalink-backup-$(date +%Y%m%d).tar.gz \
    /opt/lavalink-rust/config \
    /etc/systemd/system/lavalink-rust.service
```

### Recovery Procedures
```bash
# Stop service
sudo systemctl stop lavalink-rust

# Restore configuration
sudo cp /opt/lavalink-rust/backups/config-YYYYMMDD/* /opt/lavalink-rust/config/

# Restore service file
sudo cp /opt/lavalink-rust/backups/lavalink-rust.service /etc/systemd/system/
sudo systemctl daemon-reload

# Start service
sudo systemctl start lavalink-rust

# Verify recovery
curl http://localhost:2333/v4/info
```

### Hands-on Exercise 6: Backup and Recovery
1. Create a configuration backup
2. Make changes to the configuration
3. Simulate a failure scenario
4. Restore from backup
5. Verify service functionality

## Module 7: Rollback Procedures

### When to Rollback
- Critical functionality not working
- Performance significantly degraded
- Unresolvable compatibility issues
- Security vulnerabilities discovered

### Rollback Steps
```bash
# 1. Stop Rust service
sudo systemctl stop lavalink-rust
sudo systemctl disable lavalink-rust

# 2. Restore Java Lavalink
sudo systemctl enable lavalink-java
sudo systemctl start lavalink-java

# 3. Verify Java service
curl http://localhost:2333/version

# 4. Update monitoring (if needed)
# Update Prometheus targets, Grafana dashboards, etc.
```

### Hands-on Exercise 7: Rollback Simulation
Practice a complete rollback scenario:
1. Document current Rust state
2. Stop Rust service
3. Start Java service
4. Verify functionality
5. Update monitoring configurations

## Module 8: Security Best Practices

### Access Control
- Change default passwords
- Use firewall rules to restrict access
- Consider reverse proxy with SSL/TLS
- Monitor authentication attempts

### Updates and Maintenance
```bash
# Update yt-dlp regularly
pip3 install --upgrade yt-dlp

# Monitor for security updates
# Keep system packages updated
sudo apt update && sudo apt upgrade
```

### Security Checklist
- [ ] Default passwords changed
- [ ] Firewall configured
- [ ] Access logs monitored
- [ ] Regular updates scheduled
- [ ] Backup procedures tested

## Module 9: Practical Scenarios

### Scenario 1: High Load Event
**Situation**: Expecting 5x normal traffic
**Actions**:
1. Monitor resource usage closely
2. Scale horizontally if needed
3. Adjust buffer sizes for performance
4. Have rollback plan ready

### Scenario 2: Audio Source Outage
**Situation**: YouTube API issues
**Actions**:
1. Check service logs for errors
2. Test alternative sources (SoundCloud, HTTP)
3. Communicate with users about limitations
4. Monitor for service restoration

### Scenario 3: Memory Leak Detection
**Situation**: Memory usage growing over time
**Actions**:
1. Monitor memory usage patterns
2. Check logs for error patterns
3. Restart service as temporary fix
4. Investigate root cause
5. Consider rollback if severe

## Assessment and Certification

### Knowledge Check Questions
1. What are the main performance benefits of Rust Lavalink?
2. How do you check if the service is running properly?
3. What's the first step when troubleshooting startup issues?
4. When should you consider rolling back to Java Lavalink?
5. How do you create a configuration backup?

### Practical Assessment
Complete these tasks to demonstrate proficiency:
1. Deploy Lavalink Rust using the automated script
2. Configure monitoring with Prometheus
3. Troubleshoot a simulated service failure
4. Perform a backup and recovery operation
5. Execute a rollback to Java Lavalink

### Certification Criteria
- [ ] Successfully completed all hands-on exercises
- [ ] Passed knowledge check (80% minimum)
- [ ] Demonstrated troubleshooting skills
- [ ] Completed practical assessment

## Resources and References

### Documentation
- [Operations Guide](OPERATIONS_GUIDE.md)
- [Rollback Procedures](ROLLBACK_PROCEDURES.md)
- [Migration Guide](../MIGRATION_GUIDE.md)

### Quick Reference Cards
- [Common Commands Cheat Sheet](CHEAT_SHEET.md)
- [Troubleshooting Flowchart](TROUBLESHOOTING_FLOWCHART.md)

### Support Contacts
- **Technical Lead**: [Contact Info]
- **Infrastructure Team**: [Contact Info]
- **Emergency Escalation**: [Contact Info]

---

*Training Version: 1.0*
*Last Updated: $(date +%Y-%m-%d)*

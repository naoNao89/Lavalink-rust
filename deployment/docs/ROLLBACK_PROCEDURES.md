# Lavalink Rust Rollback Procedures

## Overview

This document outlines the procedures for rolling back from Rust Lavalink to Java Lavalink in case of critical issues or compatibility problems.

## When to Consider Rollback

### Critical Scenarios
- **Service Instability**: Frequent crashes or unrecoverable errors
- **Performance Degradation**: Significantly worse performance than Java version
- **API Compatibility Issues**: Critical endpoints not working correctly
- **Audio Source Failures**: Major audio sources completely non-functional
- **Security Vulnerabilities**: Discovered security issues without immediate fix

### Non-Critical Scenarios (Consider Fixes First)
- Minor performance issues
- Single audio source problems
- Configuration-related issues
- Monitoring/logging problems

## Pre-Rollback Checklist

### Prerequisites
- [ ] Java 17+ installed and verified
- [ ] Java Lavalink backup available (`lavalink-java/` directory)
- [ ] Current configuration backed up
- [ ] Rollback window scheduled (if production)
- [ ] Team notified of rollback procedure
- [ ] Monitoring systems prepared for service change

### Risk Assessment
- [ ] Impact on active users assessed
- [ ] Client applications compatibility verified
- [ ] Rollback time window estimated
- [ ] Communication plan prepared

## Rollback Methods

### Method 1: Automated Rollback Script (Recommended)
```bash
# Run the automated rollback script
sudo ./deployment/scripts/rollback.sh

# Monitor the process
tail -f /var/log/lavalink-rollback.log
```

### Method 2: Manual Rollback
```bash
# Step 1: Stop Rust service
sudo systemctl stop lavalink-rust
sudo systemctl disable lavalink-rust

# Step 2: Verify Java Lavalink is available
ls -la /opt/lavalink-java/Lavalink.jar
java -version

# Step 3: Create Java service (if not exists)
sudo cp deployment/systemd/lavalink-java.service /etc/systemd/system/
sudo systemctl daemon-reload

# Step 4: Start Java service
sudo systemctl enable lavalink-java
sudo systemctl start lavalink-java

# Step 5: Verify rollback
curl http://localhost:2333/version
sudo systemctl status lavalink-java
```

### Method 3: Docker Rollback
```bash
# Stop Rust containers
docker-compose -f deployment/docker-compose.yml down

# Switch to Java compose file
docker-compose -f deployment/docker-compose-java.yml up -d

# Verify services
docker-compose -f deployment/docker-compose-java.yml ps
curl http://localhost:2333/version
```

## Detailed Rollback Steps

### Phase 1: Preparation (5-10 minutes)
1. **Create Emergency Backup**
   ```bash
   sudo mkdir -p /opt/lavalink-rust/backups/emergency-$(date +%Y%m%d-%H%M)
   sudo cp -r /opt/lavalink-rust/config /opt/lavalink-rust/backups/emergency-$(date +%Y%m%d-%H%M)/
   sudo cp /etc/systemd/system/lavalink-rust.service /opt/lavalink-rust/backups/emergency-$(date +%Y%m%d-%H%M)/
   ```

2. **Document Current State**
   ```bash
   # Service status
   sudo systemctl status lavalink-rust > rollback-state.txt
   
   # Resource usage
   ps aux | grep lavalink-rust >> rollback-state.txt
   
   # API status
   curl http://localhost:2333/v4/info >> rollback-state.txt 2>&1
   ```

3. **Notify Stakeholders**
   - Send rollback initiation notice
   - Update status page if applicable
   - Inform development team

### Phase 2: Service Transition (2-5 minutes)
1. **Stop Rust Service**
   ```bash
   sudo systemctl stop lavalink-rust
   sudo systemctl disable lavalink-rust
   ```

2. **Verify Java Prerequisites**
   ```bash
   java -version
   ls -la /opt/lavalink-java/Lavalink.jar
   ```

3. **Configure Java Service**
   ```bash
   # Ensure configuration is available
   sudo cp /opt/lavalink-rust/config/application.yml /opt/lavalink-java/ 2>/dev/null || true
   
   # Install Java service
   sudo systemctl enable lavalink-java
   ```

4. **Start Java Service**
   ```bash
   sudo systemctl start lavalink-java
   
   # Wait for startup
   sleep 15
   
   # Verify startup
   sudo systemctl status lavalink-java
   ```

### Phase 3: Verification (5-10 minutes)
1. **Service Health Check**
   ```bash
   # Check service status
   sudo systemctl is-active lavalink-java
   
   # Check process
   ps aux | grep java | grep Lavalink
   
   # Check port binding
   sudo netstat -tulpn | grep :2333
   ```

2. **API Functionality Test**
   ```bash
   # Basic info endpoint
   curl http://localhost:2333/version
   curl http://localhost:2333/v4/info
   curl http://localhost:2333/v4/stats
   
   # Test track loading
   curl -H "Authorization: youshallnotpass" \
        "http://localhost:2333/v4/loadtracks?identifier=ytsearch:test"
   ```

3. **Performance Verification**
   ```bash
   # Memory usage
   ps aux | grep java | grep Lavalink
   
   # Response time test
   time curl http://localhost:2333/v4/info
   ```

### Phase 4: Post-Rollback Tasks (10-15 minutes)
1. **Update Monitoring**
   - Update Prometheus targets
   - Adjust Grafana dashboards
   - Update alerting rules
   - Restart monitoring services if needed

2. **Client Application Updates**
   - Verify client connections work
   - Test audio playback functionality
   - Monitor for any client-side errors

3. **Documentation Updates**
   - Update operational documentation
   - Record rollback reason and lessons learned
   - Update deployment status

## Rollback Verification Checklist

### Service Status
- [ ] Java Lavalink service is running (`systemctl is-active lavalink-java`)
- [ ] Rust Lavalink service is stopped (`systemctl is-active lavalink-rust`)
- [ ] Port 2333 is bound to Java process
- [ ] No error messages in service logs

### API Functionality
- [ ] `/version` endpoint returns Java version
- [ ] `/v4/info` endpoint responds correctly
- [ ] `/v4/stats` endpoint shows reasonable values
- [ ] Track loading works for major sources (YouTube, SoundCloud)
- [ ] WebSocket connections can be established

### Performance Metrics
- [ ] Memory usage is within expected range (< 2GB)
- [ ] CPU usage is reasonable (< 50% under normal load)
- [ ] Response times are acceptable (< 500ms for basic endpoints)
- [ ] No memory leaks detected over 30 minutes

### Client Integration
- [ ] Existing client connections work
- [ ] New client connections can be established
- [ ] Audio playback functions correctly
- [ ] No client-side errors reported

## Troubleshooting Rollback Issues

### Java Service Won't Start
**Symptoms**: Service fails to start or exits immediately
**Diagnosis**:
```bash
sudo journalctl -u lavalink-java -n 50
sudo systemctl status lavalink-java
```
**Solutions**:
- Check Java version compatibility
- Verify Lavalink.jar file integrity
- Check configuration file syntax
- Ensure port 2333 is available

### API Not Responding
**Symptoms**: Curl requests timeout or return errors
**Diagnosis**:
```bash
sudo netstat -tulpn | grep :2333
curl -v http://localhost:2333/version
```
**Solutions**:
- Wait longer for service startup (Java takes 10-15 seconds)
- Check firewall rules
- Verify service is actually running
- Check application.yml configuration

### High Memory Usage
**Symptoms**: Java process using excessive memory
**Diagnosis**:
```bash
ps aux | grep java | grep Lavalink
jstat -gc $(pgrep -f Lavalink.jar)
```
**Solutions**:
- Adjust JVM heap size in service file
- Monitor for memory leaks
- Restart service if needed

### Client Connection Issues
**Symptoms**: Clients can't connect or authenticate
**Diagnosis**:
```bash
sudo journalctl -u lavalink-java | grep -i "websocket\|auth"
```
**Solutions**:
- Verify password configuration
- Check WebSocket endpoint availability
- Test with curl WebSocket client

## Post-Rollback Analysis

### Data Collection
1. **Performance Metrics**
   - Memory usage comparison
   - CPU usage patterns
   - Response time measurements
   - Error rates

2. **Issue Documentation**
   - Root cause of rollback
   - Timeline of events
   - Impact assessment
   - Resolution attempts made

3. **Lessons Learned**
   - What went wrong with Rust implementation
   - What could be improved in rollback process
   - Recommendations for future migration attempts

### Reporting Template
```
ROLLBACK REPORT
===============
Date: [DATE]
Duration: [ROLLBACK_DURATION]
Initiated by: [PERSON]
Reason: [REASON]

TIMELINE:
- [TIME] Issue detected
- [TIME] Rollback decision made
- [TIME] Rollback initiated
- [TIME] Java service started
- [TIME] Verification completed

IMPACT:
- Downtime: [DURATION]
- Users affected: [NUMBER]
- Services impacted: [LIST]

ROOT CAUSE:
[DETAILED_DESCRIPTION]

RESOLUTION:
[STEPS_TAKEN]

LESSONS LEARNED:
[IMPROVEMENTS_FOR_FUTURE]

NEXT STEPS:
[ACTION_ITEMS]
```

## Recovery Planning

### Short-term Actions (24-48 hours)
- Monitor Java service stability
- Address any immediate issues
- Communicate status to stakeholders
- Document rollback experience

### Medium-term Actions (1-2 weeks)
- Analyze root cause of Rust issues
- Plan fixes for identified problems
- Update migration strategy
- Improve rollback procedures

### Long-term Actions (1+ months)
- Consider second migration attempt
- Implement additional safeguards
- Enhance testing procedures
- Update team training

## Emergency Contacts

### Primary Contacts
- **System Administrator**: [CONTACT_INFO]
- **Technical Lead**: [CONTACT_INFO]
- **Infrastructure Team**: [CONTACT_INFO]

### Escalation Path
1. On-call engineer
2. Technical lead
3. Infrastructure manager
4. CTO/Engineering director

### Communication Channels
- **Slack**: #infrastructure-alerts
- **Email**: infrastructure-team@company.com
- **Phone**: [EMERGENCY_NUMBER]

---

*Document Version: 1.0*
*Last Updated: $(date +%Y-%m-%d)*
*Next Review: $(date -d '+3 months' +%Y-%m-%d)*

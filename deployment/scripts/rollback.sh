#!/bin/bash

# Lavalink Rust Rollback Script
# This script handles rollback from Rust Lavalink to Java Lavalink

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUST_SERVICE="lavalink-rust"
JAVA_SERVICE="lavalink-java"
RUST_INSTALL_DIR="/opt/lavalink-rust"
JAVA_INSTALL_DIR="/opt/lavalink-java"
BACKUP_DIR="/opt/lavalink-rust/backups"
ROLLBACK_LOG="/var/log/lavalink-rollback.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$ROLLBACK_LOG"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$ROLLBACK_LOG"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$ROLLBACK_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$ROLLBACK_LOG"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# Initialize logging
init_logging() {
    echo "=== Lavalink Rollback Started at $(date) ===" >> "$ROLLBACK_LOG"
    log_info "Rollback process initiated"
}

# Pre-rollback checks
pre_rollback_checks() {
    log_info "Performing pre-rollback checks..."
    
    # Check if Java Lavalink is available
    if [[ ! -f "$JAVA_INSTALL_DIR/Lavalink.jar" ]]; then
        log_error "Java Lavalink not found at $JAVA_INSTALL_DIR/Lavalink.jar"
        log_error "Cannot proceed with rollback without Java Lavalink backup"
        exit 1
    fi
    
    # Check if Java is installed
    if ! command -v java &> /dev/null; then
        log_error "Java is not installed. Please install Java 17+ before rollback"
        exit 1
    fi
    
    # Check Java version
    java_version=$(java -version 2>&1 | head -n1 | cut -d'"' -f2 | cut -d'.' -f1)
    if [[ "$java_version" -lt 17 ]]; then
        log_error "Java version $java_version is too old. Java 17+ required"
        exit 1
    fi
    
    log_success "Pre-rollback checks passed"
}

# Create rollback backup
create_rollback_backup() {
    local backup_name="rollback-backup-$(date +%Y%m%d-%H%M%S)"
    log_info "Creating rollback backup: $backup_name"
    
    mkdir -p "$BACKUP_DIR/$backup_name"
    
    # Backup Rust configuration
    if [[ -d "$RUST_INSTALL_DIR/config" ]]; then
        cp -r "$RUST_INSTALL_DIR/config" "$BACKUP_DIR/$backup_name/rust-config"
    fi
    
    # Backup service files
    if [[ -f "/etc/systemd/system/$RUST_SERVICE.service" ]]; then
        cp "/etc/systemd/system/$RUST_SERVICE.service" "$BACKUP_DIR/$backup_name/"
    fi
    
    # Save current service status
    systemctl is-active "$RUST_SERVICE" > "$BACKUP_DIR/$backup_name/rust-service-status.txt" 2>&1 || true
    systemctl is-enabled "$RUST_SERVICE" > "$BACKUP_DIR/$backup_name/rust-service-enabled.txt" 2>&1 || true
    
    log_success "Rollback backup created at $BACKUP_DIR/$backup_name"
}

# Stop Rust Lavalink service
stop_rust_service() {
    log_info "Stopping Rust Lavalink service..."
    
    if systemctl is-active --quiet "$RUST_SERVICE"; then
        systemctl stop "$RUST_SERVICE"
        log_success "Rust service stopped"
    else
        log_info "Rust service was not running"
    fi
    
    # Disable auto-start
    if systemctl is-enabled --quiet "$RUST_SERVICE"; then
        systemctl disable "$RUST_SERVICE"
        log_success "Rust service disabled"
    fi
}

# Setup Java Lavalink service
setup_java_service() {
    log_info "Setting up Java Lavalink service..."
    
    # Create Java service file if it doesn't exist
    if [[ ! -f "/etc/systemd/system/$JAVA_SERVICE.service" ]]; then
        log_info "Creating Java Lavalink systemd service..."
        cat > "/etc/systemd/system/$JAVA_SERVICE.service" << EOF
[Unit]
Description=Lavalink Java Service - Audio sending node for Discord
Documentation=https://github.com/lavalink-devs/Lavalink
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=lavalink
Group=lavalink
WorkingDirectory=$JAVA_INSTALL_DIR
ExecStart=java -Xmx1G -jar $JAVA_INSTALL_DIR/Lavalink.jar
Restart=on-failure
RestartSec=5s
StartLimitInterval=60s
StartLimitBurst=3

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$JAVA_INSTALL_DIR/logs
ReadOnlyPaths=$JAVA_INSTALL_DIR

# Resource limits
MemoryMax=2G
CPUQuota=200%

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=lavalink-java

[Install]
WantedBy=multi-user.target
EOF
        log_success "Java service file created"
    fi
    
    # Reload systemd
    systemctl daemon-reload
    
    # Enable Java service
    systemctl enable "$JAVA_SERVICE"
    log_success "Java service enabled"
}

# Migrate configuration
migrate_configuration() {
    log_info "Migrating configuration from Rust to Java..."
    
    # Ensure Java config directory exists
    mkdir -p "$JAVA_INSTALL_DIR"
    
    # Copy configuration if Rust config exists and Java config doesn't
    if [[ -f "$RUST_INSTALL_DIR/config/application.yml" ]] && [[ ! -f "$JAVA_INSTALL_DIR/application.yml" ]]; then
        cp "$RUST_INSTALL_DIR/config/application.yml" "$JAVA_INSTALL_DIR/application.yml"
        chown lavalink:lavalink "$JAVA_INSTALL_DIR/application.yml"
        log_success "Configuration migrated to Java Lavalink"
    elif [[ -f "$JAVA_INSTALL_DIR/application.yml" ]]; then
        log_info "Java configuration already exists, not overwriting"
    else
        log_warning "No configuration found to migrate"
    fi
}

# Start Java Lavalink service
start_java_service() {
    log_info "Starting Java Lavalink service..."
    
    systemctl start "$JAVA_SERVICE"
    
    # Wait for service to start
    sleep 10
    
    if systemctl is-active --quiet "$JAVA_SERVICE"; then
        log_success "Java service started successfully"
        
        # Test API availability
        local max_attempts=12
        local attempt=1
        
        while [[ $attempt -le $max_attempts ]]; do
            if curl -f -s http://localhost:2333/version > /dev/null 2>&1; then
                log_success "Java Lavalink API is responding"
                break
            else
                log_info "Waiting for API to be ready... (attempt $attempt/$max_attempts)"
                sleep 5
                ((attempt++))
            fi
        done
        
        if [[ $attempt -gt $max_attempts ]]; then
            log_warning "API not responding after $max_attempts attempts"
            log_info "Check service logs: journalctl -u $JAVA_SERVICE -f"
        fi
    else
        log_error "Failed to start Java service"
        log_info "Check logs with: journalctl -u $JAVA_SERVICE -f"
        exit 1
    fi
}

# Update monitoring configuration
update_monitoring() {
    log_info "Updating monitoring configuration..."
    
    # Update Prometheus configuration if it exists
    local prometheus_config="$PROJECT_ROOT/deployment/monitoring/prometheus.yml"
    if [[ -f "$prometheus_config" ]]; then
        # Create backup of current config
        cp "$prometheus_config" "$prometheus_config.rollback-backup"
        
        # Update target (this is a simple example - adjust based on your setup)
        sed -i 's/lavalink-rust:9090/localhost:2333/g' "$prometheus_config" || true
        log_info "Prometheus configuration updated"
    fi
    
    # Restart monitoring stack if running
    if command -v docker-compose &> /dev/null; then
        cd "$PROJECT_ROOT/deployment" 2>/dev/null || true
        if [[ -f "docker-compose.yml" ]] && docker-compose ps | grep -q prometheus; then
            log_info "Restarting monitoring stack..."
            docker-compose restart prometheus || log_warning "Failed to restart Prometheus"
        fi
    fi
}

# Verify rollback success
verify_rollback() {
    log_info "Verifying rollback success..."
    
    # Check Java service status
    if systemctl is-active --quiet "$JAVA_SERVICE"; then
        log_success "✓ Java Lavalink service is running"
    else
        log_error "✗ Java Lavalink service is not running"
        return 1
    fi
    
    # Check API response
    if curl -f -s http://localhost:2333/version > /dev/null; then
        local version=$(curl -s http://localhost:2333/version 2>/dev/null || echo "unknown")
        log_success "✓ Java Lavalink API is responding (version: $version)"
    else
        log_error "✗ Java Lavalink API is not responding"
        return 1
    fi
    
    # Check Rust service is stopped
    if ! systemctl is-active --quiet "$RUST_SERVICE"; then
        log_success "✓ Rust Lavalink service is stopped"
    else
        log_warning "⚠ Rust Lavalink service is still running"
    fi
    
    log_success "Rollback verification completed successfully"
}

# Generate rollback report
generate_report() {
    local report_file="$BACKUP_DIR/rollback-report-$(date +%Y%m%d-%H%M%S).txt"
    
    cat > "$report_file" << EOF
=== LAVALINK ROLLBACK REPORT ===
Date: $(date)
Rollback Duration: $SECONDS seconds

SERVICES STATUS:
- Java Lavalink: $(systemctl is-active $JAVA_SERVICE 2>/dev/null || echo "unknown")
- Rust Lavalink: $(systemctl is-active $RUST_SERVICE 2>/dev/null || echo "unknown")

API STATUS:
- Endpoint: http://localhost:2333/version
- Response: $(curl -s http://localhost:2333/version 2>/dev/null || echo "No response")

CONFIGURATION:
- Java Config: $(test -f "$JAVA_INSTALL_DIR/application.yml" && echo "Present" || echo "Missing")
- Rust Config Backup: $(test -d "$BACKUP_DIR" && echo "Available" || echo "Missing")

NEXT STEPS:
1. Monitor Java Lavalink performance
2. Update client applications if needed
3. Review rollback logs: $ROLLBACK_LOG
4. Consider investigating Rust issues for future migration

ROLLBACK LOG LOCATION: $ROLLBACK_LOG
BACKUP LOCATION: $BACKUP_DIR
EOF

    log_info "Rollback report generated: $report_file"
}

# Main rollback function
main() {
    local start_time=$(date +%s)
    
    log_info "=== Starting Lavalink Rollback Process ==="
    
    check_root
    init_logging
    pre_rollback_checks
    create_rollback_backup
    stop_rust_service
    setup_java_service
    migrate_configuration
    start_java_service
    update_monitoring
    verify_rollback
    generate_report
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    log_success "=== Rollback completed successfully in ${duration} seconds ==="
    log_info "Java Lavalink is now running on http://localhost:2333"
    log_info "Monitor with: journalctl -u $JAVA_SERVICE -f"
    log_info "Rollback report and backups available in: $BACKUP_DIR"
}

# Handle script interruption
trap 'log_error "Rollback interrupted! Check logs and service status."; exit 1' INT TERM

# Run main function
main "$@"

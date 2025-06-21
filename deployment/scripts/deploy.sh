#!/bin/bash

# Lavalink Rust Deployment Script
# This script handles the deployment of Lavalink Rust to production

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_USER="lavalink"
DEPLOY_GROUP="lavalink"
INSTALL_DIR="/opt/lavalink-rust"
SERVICE_NAME="lavalink-rust"
BACKUP_DIR="/opt/lavalink-rust/backups"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# Create user and group if they don't exist
create_user() {
    if ! getent group "$DEPLOY_GROUP" > /dev/null 2>&1; then
        log_info "Creating group: $DEPLOY_GROUP"
        groupadd -g 322 "$DEPLOY_GROUP"
    fi

    if ! getent passwd "$DEPLOY_USER" > /dev/null 2>&1; then
        log_info "Creating user: $DEPLOY_USER"
        useradd -r -u 322 -g "$DEPLOY_GROUP" -d "$INSTALL_DIR" -s /bin/bash "$DEPLOY_USER"
    fi
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    mkdir -p "$INSTALL_DIR"/{bin,config,logs,plugins,backups}
    chown -R "$DEPLOY_USER:$DEPLOY_GROUP" "$INSTALL_DIR"
    chmod 755 "$INSTALL_DIR"
    chmod 750 "$INSTALL_DIR"/{logs,backups}
}

# Build the Rust binary
build_binary() {
    log_info "Building Lavalink Rust binary..."
    cd "$PROJECT_ROOT"
    
    # Install yt-dlp if not present
    if ! command -v yt-dlp &> /dev/null; then
        log_info "Installing yt-dlp..."
        pip3 install yt-dlp
    fi
    
    # Build release binary
    cargo build --release
    
    if [[ ! -f "target/release/lavalink-rust" ]]; then
        log_error "Failed to build binary"
        exit 1
    fi
    
    log_success "Binary built successfully"
}

# Stop existing service
stop_service() {
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        log_info "Stopping existing service..."
        systemctl stop "$SERVICE_NAME"
        log_success "Service stopped"
    else
        log_info "Service is not running"
    fi
}

# Backup current installation
backup_current() {
    if [[ -f "$INSTALL_DIR/bin/lavalink-rust" ]]; then
        local backup_name="backup-$(date +%Y%m%d-%H%M%S)"
        log_info "Creating backup: $backup_name"
        
        mkdir -p "$BACKUP_DIR/$backup_name"
        cp -r "$INSTALL_DIR"/{bin,config} "$BACKUP_DIR/$backup_name/" 2>/dev/null || true
        
        log_success "Backup created at $BACKUP_DIR/$backup_name"
    else
        log_info "No existing installation to backup"
    fi
}

# Install binary and configuration
install_files() {
    log_info "Installing binary and configuration..."
    
    # Install binary
    cp "$PROJECT_ROOT/target/release/lavalink-rust" "$INSTALL_DIR/bin/"
    chmod +x "$INSTALL_DIR/bin/lavalink-rust"
    
    # Install configuration if it doesn't exist
    if [[ ! -f "$INSTALL_DIR/config/application.yml" ]]; then
        cp "$PROJECT_ROOT/application.yml" "$INSTALL_DIR/config/"
    else
        log_warning "Configuration exists, not overwriting. Check for updates manually."
    fi
    
    # Set ownership
    chown -R "$DEPLOY_USER:$DEPLOY_GROUP" "$INSTALL_DIR"
    
    log_success "Files installed successfully"
}

# Install systemd service
install_service() {
    log_info "Installing systemd service..."
    
    # Update service file with correct paths
    sed "s|/opt/lavalink-rust/lavalink-rust|$INSTALL_DIR/bin/lavalink-rust|g; \
         s|/opt/lavalink-rust/application.yml|$INSTALL_DIR/config/application.yml|g; \
         s|ReadWritePaths=/opt/lavalink-rust/logs|ReadWritePaths=$INSTALL_DIR/logs|g; \
         s|ReadOnlyPaths=/opt/lavalink-rust|ReadOnlyPaths=$INSTALL_DIR|g" \
         "$PROJECT_ROOT/deployment/systemd/lavalink-rust.service" > /etc/systemd/system/lavalink-rust.service
    
    systemctl daemon-reload
    systemctl enable "$SERVICE_NAME"
    
    log_success "Systemd service installed and enabled"
}

# Start service and verify
start_service() {
    log_info "Starting service..."
    systemctl start "$SERVICE_NAME"
    
    # Wait a moment for startup
    sleep 5
    
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        log_success "Service started successfully"
        
        # Check if API is responding
        if curl -f -s http://localhost:2333/v4/info > /dev/null; then
            log_success "API is responding correctly"
        else
            log_warning "Service started but API may not be ready yet"
        fi
    else
        log_error "Failed to start service"
        log_info "Check logs with: journalctl -u $SERVICE_NAME -f"
        exit 1
    fi
}

# Main deployment function
main() {
    log_info "Starting Lavalink Rust deployment..."
    
    check_root
    create_user
    create_directories
    build_binary
    stop_service
    backup_current
    install_files
    install_service
    start_service
    
    log_success "Deployment completed successfully!"
    log_info "Service status: $(systemctl is-active $SERVICE_NAME)"
    log_info "View logs with: journalctl -u $SERVICE_NAME -f"
    log_info "Service info available at: http://localhost:2333/v4/info"
}

# Run main function
main "$@"

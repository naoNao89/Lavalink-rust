#!/usr/bin/env bash

# Nix-based deployment script for Lavalink Rust
set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

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

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] ENVIRONMENT

Deploy Lavalink Rust using Nix to the specified environment.

ENVIRONMENTS:
    development     Deploy to development environment
    staging         Deploy to staging environment  
    production      Deploy to production environment

OPTIONS:
    -h, --help      Show this help message
    -d, --dry-run   Show what would be deployed without actually deploying
    -v, --verbose   Enable verbose output
    --build-only    Only build, don't deploy
    --test          Run tests before deployment

EXAMPLES:
    $0 development                    # Deploy to development
    $0 --dry-run production          # Show what would be deployed to production
    $0 --test --verbose staging      # Test and deploy to staging with verbose output

PREREQUISITES:
    - Nix with flakes enabled
    - SSH access to target hosts (for remote deployment)
    - deploy-rs installed (for remote deployment)

EOF
}

# Parse command line arguments
parse_args() {
    DRY_RUN=false
    VERBOSE=false
    BUILD_ONLY=false
    RUN_TESTS=false
    ENVIRONMENT=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --build-only)
                BUILD_ONLY=true
                shift
                ;;
            --test)
                RUN_TESTS=true
                shift
                ;;
            development|staging|production)
                ENVIRONMENT="$1"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    if [[ -z "$ENVIRONMENT" ]]; then
        log_error "Environment must be specified"
        show_usage
        exit 1
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Nix
    if ! command -v nix &> /dev/null; then
        log_error "Nix is not installed"
        exit 1
    fi
    
    # Check if flakes are enabled
    if ! nix flake --help &> /dev/null; then
        log_error "Nix flakes are not enabled"
        log_info "Enable flakes by adding 'experimental-features = nix-command flakes' to nix.conf"
        exit 1
    fi
    
    # Check deploy-rs for remote deployment
    if [[ "$ENVIRONMENT" != "development" ]] && ! command -v deploy &> /dev/null; then
        log_warning "deploy-rs not found, will use nixos-rebuild instead"
    fi
    
    log_success "Prerequisites check passed"
}

# Run tests
run_tests() {
    if [[ "$RUN_TESTS" == true ]]; then
        log_info "Running tests..."
        
        cd "$PROJECT_ROOT"
        
        # Run Nix checks
        if nix flake check; then
            log_success "Nix flake checks passed"
        else
            log_error "Nix flake checks failed"
            exit 1
        fi
        
        # Run custom tests
        if nix run .#tests.runner; then
            log_success "Custom tests passed"
        else
            log_error "Custom tests failed"
            exit 1
        fi
    fi
}

# Build the system
build_system() {
    log_info "Building system for $ENVIRONMENT..."
    
    cd "$PROJECT_ROOT"
    
    local build_cmd="nix build .#nixosConfigurations.$ENVIRONMENT.config.system.build.toplevel"
    
    if [[ "$VERBOSE" == true ]]; then
        build_cmd="$build_cmd --verbose"
    fi
    
    if [[ "$DRY_RUN" == true ]]; then
        build_cmd="$build_cmd --dry-run"
        log_info "Would run: $build_cmd"
    else
        if eval "$build_cmd"; then
            log_success "Build completed successfully"
        else
            log_error "Build failed"
            exit 1
        fi
    fi
}

# Deploy to local system
deploy_local() {
    log_info "Deploying to local system..."
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "Would run: sudo nixos-rebuild switch --flake .#$ENVIRONMENT"
        return
    fi
    
    cd "$PROJECT_ROOT"
    
    local rebuild_cmd="sudo nixos-rebuild switch --flake .#$ENVIRONMENT"
    
    if [[ "$VERBOSE" == true ]]; then
        rebuild_cmd="$rebuild_cmd --verbose"
    fi
    
    if eval "$rebuild_cmd"; then
        log_success "Local deployment completed successfully"
    else
        log_error "Local deployment failed"
        exit 1
    fi
}

# Deploy to remote system
deploy_remote() {
    local target="$1"
    
    log_info "Deploying to remote system: $target"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "Would run: deploy --hostname $target .#$ENVIRONMENT"
        return
    fi
    
    cd "$PROJECT_ROOT"
    
    if command -v deploy &> /dev/null; then
        # Use deploy-rs
        local deploy_cmd="deploy --hostname $target .#$ENVIRONMENT"
        
        if [[ "$VERBOSE" == true ]]; then
            deploy_cmd="$deploy_cmd --verbose"
        fi
        
        if eval "$deploy_cmd"; then
            log_success "Remote deployment completed successfully"
        else
            log_error "Remote deployment failed"
            exit 1
        fi
    else
        # Fallback to nixos-rebuild
        log_warning "Using nixos-rebuild for remote deployment"
        
        local rebuild_cmd="nixos-rebuild switch --flake .#$ENVIRONMENT --target-host $target"
        
        if [[ "$VERBOSE" == true ]]; then
            rebuild_cmd="$rebuild_cmd --verbose"
        fi
        
        if eval "$rebuild_cmd"; then
            log_success "Remote deployment completed successfully"
        else
            log_error "Remote deployment failed"
            exit 1
        fi
    fi
}

# Main deployment function
deploy() {
    case "$ENVIRONMENT" in
        development)
            deploy_local
            ;;
        staging)
            deploy_remote "staging.example.com"
            ;;
        production)
            deploy_remote "lavalink.example.com"
            ;;
        *)
            log_error "Unknown environment: $ENVIRONMENT"
            exit 1
            ;;
    esac
}

# Verify deployment
verify_deployment() {
    if [[ "$DRY_RUN" == true ]]; then
        log_info "Skipping verification in dry-run mode"
        return
    fi
    
    log_info "Verifying deployment..."
    
    # Wait a moment for services to start
    sleep 5
    
    case "$ENVIRONMENT" in
        development)
            local target="localhost"
            ;;
        staging)
            local target="staging.example.com"
            ;;
        production)
            local target="lavalink.example.com"
            ;;
    esac
    
    # Check if service is running
    if ssh "$target" "systemctl is-active lavalink-rust" &> /dev/null; then
        log_success "Lavalink Rust service is running"
    else
        log_error "Lavalink Rust service is not running"
        exit 1
    fi
    
    # Check if API is responding
    if ssh "$target" "curl -f -s http://localhost:2333/v4/info" &> /dev/null; then
        log_success "Lavalink Rust API is responding"
    else
        log_error "Lavalink Rust API is not responding"
        exit 1
    fi
}

# Main function
main() {
    parse_args "$@"
    
    log_info "Starting Nix deployment for Lavalink Rust"
    log_info "Environment: $ENVIRONMENT"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "Running in dry-run mode"
    fi
    
    check_prerequisites
    run_tests
    build_system
    
    if [[ "$BUILD_ONLY" == true ]]; then
        log_success "Build-only mode completed"
        exit 0
    fi
    
    deploy
    verify_deployment
    
    log_success "Deployment completed successfully!"
    log_info "Lavalink Rust is now running on $ENVIRONMENT"
    
    case "$ENVIRONMENT" in
        development)
            log_info "Access the service at: http://localhost:2333"
            ;;
        staging)
            log_info "Access the service at: https://staging.example.com"
            ;;
        production)
            log_info "Access the service at: https://lavalink.example.com"
            ;;
    esac
}

# Run main function
main "$@"

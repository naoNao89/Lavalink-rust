#!/bin/bash

# Lavalink Rust Deployment Test Suite
# This script tests all deployment configurations and scripts

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_LOG="$SCRIPT_DIR/test-results.log"
TEMP_DIR="/tmp/lavalink-rust-test-$$"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_TOTAL=0
TESTS_PASSED=0
TESTS_FAILED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$TEST_LOG"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$TEST_LOG"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$TEST_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$TEST_LOG"
}

# Test framework functions
test_start() {
    local test_name="$1"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    log_info "Running test: $test_name"
}

test_pass() {
    local test_name="$1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_success "✓ $test_name"
}

test_fail() {
    local test_name="$1"
    local reason="$2"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ $test_name: $reason"
}

# Setup test environment
setup_test_env() {
    log_info "Setting up test environment..."
    
    # Create temporary directory
    mkdir -p "$TEMP_DIR"
    
    # Initialize test log
    echo "=== Lavalink Rust Deployment Test Results ===" > "$TEST_LOG"
    echo "Date: $(date)" >> "$TEST_LOG"
    echo "Project Root: $PROJECT_ROOT" >> "$TEST_LOG"
    echo "" >> "$TEST_LOG"
    
    log_success "Test environment ready"
}

# Cleanup test environment
cleanup_test_env() {
    log_info "Cleaning up test environment..."
    rm -rf "$TEMP_DIR"
    log_success "Cleanup completed"
}

# Test 1: Validate Docker Compose Configuration
test_docker_compose_config() {
    test_start "Docker Compose Configuration"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if [[ ! -f "$compose_file" ]]; then
        test_fail "Docker Compose Configuration" "docker-compose.yml not found"
        return
    fi
    
    # Check if docker-compose is available
    if ! command -v docker-compose &> /dev/null && ! command -v docker &> /dev/null; then
        test_fail "Docker Compose Configuration" "Docker/docker-compose not available"
        return
    fi
    
    # Validate compose file syntax
    if command -v docker-compose &> /dev/null; then
        if docker-compose -f "$compose_file" config > /dev/null 2>&1; then
            test_pass "Docker Compose Configuration"
        else
            test_fail "Docker Compose Configuration" "Invalid docker-compose.yml syntax"
        fi
    elif command -v docker &> /dev/null; then
        if docker compose -f "$compose_file" config > /dev/null 2>&1; then
            test_pass "Docker Compose Configuration"
        else
            test_fail "Docker Compose Configuration" "Invalid docker-compose.yml syntax"
        fi
    fi
}

# Test 2: Validate Dockerfile
test_dockerfile() {
    test_start "Dockerfile Validation"
    
    local dockerfile="$SCRIPT_DIR/Dockerfile"
    
    if [[ ! -f "$dockerfile" ]]; then
        test_fail "Dockerfile Validation" "Dockerfile not found"
        return
    fi
    
    # Check Dockerfile syntax
    if command -v docker &> /dev/null; then
        if docker build -f "$dockerfile" -t lavalink-rust-test --dry-run "$PROJECT_ROOT" > /dev/null 2>&1; then
            test_pass "Dockerfile Validation"
        else
            # Try without --dry-run flag (older Docker versions)
            log_warning "Docker --dry-run not supported, checking Dockerfile manually"
            if grep -q "FROM debian:bookworm-slim" "$dockerfile" && \
               grep -q "COPY lavalink-rust /app/lavalink-rust" "$dockerfile" && \
               grep -q "EXPOSE 2333" "$dockerfile"; then
                test_pass "Dockerfile Validation"
            else
                test_fail "Dockerfile Validation" "Missing required Dockerfile components"
            fi
        fi
    else
        log_warning "Docker not available, checking Dockerfile manually"
        if grep -q "FROM debian:bookworm-slim" "$dockerfile" && \
           grep -q "COPY lavalink-rust /app/lavalink-rust" "$dockerfile" && \
           grep -q "EXPOSE 2333" "$dockerfile"; then
            test_pass "Dockerfile Validation"
        else
            test_fail "Dockerfile Validation" "Missing required Dockerfile components"
        fi
    fi
}

# Test 3: Validate Alpine Dockerfile
test_dockerfile_alpine() {
    test_start "Alpine Dockerfile Validation"
    
    local dockerfile="$SCRIPT_DIR/Dockerfile.alpine"
    
    if [[ ! -f "$dockerfile" ]]; then
        test_fail "Alpine Dockerfile Validation" "Dockerfile.alpine not found"
        return
    fi
    
    # Check Alpine Dockerfile components
    if grep -q "FROM alpine:" "$dockerfile" && \
       grep -q "apk add --no-cache" "$dockerfile" && \
       grep -q "EXPOSE 2333" "$dockerfile"; then
        test_pass "Alpine Dockerfile Validation"
    else
        test_fail "Alpine Dockerfile Validation" "Missing required Alpine Dockerfile components"
    fi
}

# Test 4: Validate Deployment Script
test_deployment_script() {
    test_start "Deployment Script Validation"
    
    local deploy_script="$SCRIPT_DIR/scripts/deploy.sh"
    
    if [[ ! -f "$deploy_script" ]]; then
        test_fail "Deployment Script Validation" "deploy.sh not found"
        return
    fi
    
    # Check script permissions
    if [[ ! -x "$deploy_script" ]]; then
        test_fail "Deployment Script Validation" "deploy.sh is not executable"
        return
    fi
    
    # Check script syntax
    if bash -n "$deploy_script"; then
        test_pass "Deployment Script Validation"
    else
        test_fail "Deployment Script Validation" "Syntax errors in deploy.sh"
    fi
}

# Test 5: Validate Rollback Script
test_rollback_script() {
    test_start "Rollback Script Validation"
    
    local rollback_script="$SCRIPT_DIR/scripts/rollback.sh"
    
    if [[ ! -f "$rollback_script" ]]; then
        test_fail "Rollback Script Validation" "rollback.sh not found"
        return
    fi
    
    # Check script permissions
    if [[ ! -x "$rollback_script" ]]; then
        test_fail "Rollback Script Validation" "rollback.sh is not executable"
        return
    fi
    
    # Check script syntax
    if bash -n "$rollback_script"; then
        test_pass "Rollback Script Validation"
    else
        test_fail "Rollback Script Validation" "Syntax errors in rollback.sh"
    fi
}

# Test 6: Validate SystemD Service File
test_systemd_service() {
    test_start "SystemD Service File Validation"
    
    local service_file="$SCRIPT_DIR/systemd/lavalink-rust.service"
    
    if [[ ! -f "$service_file" ]]; then
        test_fail "SystemD Service File Validation" "lavalink-rust.service not found"
        return
    fi
    
    # Check required service file sections
    if grep -q "\[Unit\]" "$service_file" && \
       grep -q "\[Service\]" "$service_file" && \
       grep -q "\[Install\]" "$service_file" && \
       grep -q "ExecStart=" "$service_file"; then
        test_pass "SystemD Service File Validation"
    else
        test_fail "SystemD Service File Validation" "Missing required service file sections"
    fi
}

# Test 7: Validate Monitoring Configuration
test_monitoring_config() {
    test_start "Monitoring Configuration Validation"
    
    local prometheus_config="$SCRIPT_DIR/monitoring/prometheus.yml"
    
    if [[ ! -f "$prometheus_config" ]]; then
        test_fail "Monitoring Configuration Validation" "prometheus.yml not found"
        return
    fi
    
    # Check Prometheus config syntax (basic YAML validation)
    if command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$prometheus_config'))" 2>/dev/null; then
            test_pass "Monitoring Configuration Validation"
        else
            test_fail "Monitoring Configuration Validation" "Invalid prometheus.yml YAML syntax"
        fi
    else
        # Basic check without YAML parser
        if grep -q "global:" "$prometheus_config" && \
           grep -q "scrape_configs:" "$prometheus_config"; then
            test_pass "Monitoring Configuration Validation"
        else
            test_fail "Monitoring Configuration Validation" "Missing required Prometheus config sections"
        fi
    fi
}

# Test 8: Check Required Dependencies
test_dependencies() {
    test_start "Required Dependencies Check"
    
    local missing_deps=()
    
    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi
    
    # Check for curl (used in health checks)
    if ! command -v curl &> /dev/null; then
        missing_deps+=("curl")
    fi
    
    if [[ ${#missing_deps[@]} -eq 0 ]]; then
        test_pass "Required Dependencies Check"
    else
        test_fail "Required Dependencies Check" "Missing dependencies: ${missing_deps[*]}"
    fi
}

# Test 9: Validate Configuration Files
test_config_files() {
    test_start "Configuration Files Validation"
    
    local app_config="$PROJECT_ROOT/application.yml"
    
    if [[ ! -f "$app_config" ]]; then
        test_fail "Configuration Files Validation" "application.yml not found"
        return
    fi
    
    # Basic YAML validation
    if command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$app_config'))" 2>/dev/null; then
            test_pass "Configuration Files Validation"
        else
            test_fail "Configuration Files Validation" "Invalid application.yml YAML syntax"
        fi
    else
        # Basic check for required sections
        if grep -q "server:" "$app_config" && \
           grep -q "lavalink:" "$app_config"; then
            test_pass "Configuration Files Validation"
        else
            test_fail "Configuration Files Validation" "Missing required config sections"
        fi
    fi
}

# Test 10: Directory Structure Validation
test_directory_structure() {
    test_start "Directory Structure Validation"
    
    local required_dirs=(
        "scripts"
        "systemd"
        "monitoring"
        "docs"
    )
    
    local missing_dirs=()
    
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "$SCRIPT_DIR/$dir" ]]; then
            missing_dirs+=("$dir")
        fi
    done
    
    if [[ ${#missing_dirs[@]} -eq 0 ]]; then
        test_pass "Directory Structure Validation"
    else
        test_fail "Directory Structure Validation" "Missing directories: ${missing_dirs[*]}"
    fi
}

# Main test runner
run_all_tests() {
    log_info "Starting Lavalink Rust Deployment Test Suite"
    
    setup_test_env
    
    # Run all tests
    test_docker_compose_config
    test_dockerfile
    test_dockerfile_alpine
    test_deployment_script
    test_rollback_script
    test_systemd_service
    test_monitoring_config
    test_dependencies
    test_config_files
    test_directory_structure
    
    cleanup_test_env
    
    # Print summary
    echo ""
    log_info "=== Test Summary ==="
    log_info "Total tests: $TESTS_TOTAL"
    log_success "Passed: $TESTS_PASSED"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Failed: $TESTS_FAILED"
    else
        log_success "Failed: $TESTS_FAILED"
    fi
    
    echo ""
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "All deployment tests passed! ✓"
        echo "Test results saved to: $TEST_LOG"
        exit 0
    else
        log_error "Some deployment tests failed! ✗"
        echo "Check test results in: $TEST_LOG"
        exit 1
    fi
}

# Run tests
run_all_tests

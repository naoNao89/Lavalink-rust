#!/bin/bash

# Lavalink Rust Docker Test Suite
# This script tests Docker configurations and builds

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_LOG="$SCRIPT_DIR/docker-test-results.log"

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

# Check if Docker is available
check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    log_success "Docker is available and running"
}

# Test 1: Build Standard Dockerfile
test_build_standard_dockerfile() {
    test_start "Build Standard Dockerfile"
    
    # Create a dummy binary for testing
    local dummy_binary="$PROJECT_ROOT/lavalink-rust"
    echo '#!/bin/bash
echo "Lavalink Rust Test Binary"
echo "Version: 4.0.0-test"
sleep 1
' > "$dummy_binary"
    chmod +x "$dummy_binary"
    
    # Try to build the Docker image
    if docker build -f "$SCRIPT_DIR/Dockerfile" -t lavalink-rust-test:standard "$PROJECT_ROOT" > /dev/null 2>&1; then
        test_pass "Build Standard Dockerfile"
        # Clean up the image
        docker rmi lavalink-rust-test:standard > /dev/null 2>&1 || true
    else
        test_fail "Build Standard Dockerfile" "Failed to build Docker image"
    fi
    
    # Clean up dummy binary
    rm -f "$dummy_binary"
}

# Test 2: Build Alpine Dockerfile
test_build_alpine_dockerfile() {
    test_start "Build Alpine Dockerfile"
    
    # Create a dummy musl binary for testing
    local dummy_binary="$PROJECT_ROOT/lavalink-rust-musl"
    echo '#!/bin/sh
echo "Lavalink Rust Alpine Test Binary"
echo "Version: 4.0.0-test-alpine"
sleep 1
' > "$dummy_binary"
    chmod +x "$dummy_binary"
    
    # Try to build the Alpine Docker image
    if docker build -f "$SCRIPT_DIR/Dockerfile.alpine" -t lavalink-rust-test:alpine "$PROJECT_ROOT" > /dev/null 2>&1; then
        test_pass "Build Alpine Dockerfile"
        # Clean up the image
        docker rmi lavalink-rust-test:alpine > /dev/null 2>&1 || true
    else
        test_fail "Build Alpine Dockerfile" "Failed to build Alpine Docker image"
    fi
    
    # Clean up dummy binary
    rm -f "$dummy_binary"
}

# Test 3: Docker Compose Validation
test_docker_compose_validation() {
    test_start "Docker Compose Validation"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    # Check if docker-compose or docker compose is available
    local compose_cmd=""
    if command -v docker-compose &> /dev/null; then
        compose_cmd="docker-compose"
    elif docker compose version &> /dev/null; then
        compose_cmd="docker compose"
    else
        test_fail "Docker Compose Validation" "Neither docker-compose nor docker compose available"
        return
    fi
    
    # Validate compose file
    if $compose_cmd -f "$compose_file" config > /dev/null 2>&1; then
        test_pass "Docker Compose Validation"
    else
        test_fail "Docker Compose Validation" "Invalid docker-compose.yml"
    fi
}

# Test 4: Docker Compose Services Check
test_docker_compose_services() {
    test_start "Docker Compose Services Check"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    local required_services=("lavalink-rust" "prometheus" "grafana")
    local missing_services=()
    
    for service in "${required_services[@]}"; do
        if ! grep -q "^  $service:" "$compose_file"; then
            missing_services+=("$service")
        fi
    done
    
    if [[ ${#missing_services[@]} -eq 0 ]]; then
        test_pass "Docker Compose Services Check"
    else
        test_fail "Docker Compose Services Check" "Missing services: ${missing_services[*]}"
    fi
}

# Test 5: Docker Network Configuration
test_docker_network_config() {
    test_start "Docker Network Configuration"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if grep -q "networks:" "$compose_file" && \
       grep -q "lavalink:" "$compose_file" && \
       grep -q "driver: bridge" "$compose_file"; then
        test_pass "Docker Network Configuration"
    else
        test_fail "Docker Network Configuration" "Missing or invalid network configuration"
    fi
}

# Test 6: Docker Volume Configuration
test_docker_volume_config() {
    test_start "Docker Volume Configuration"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if grep -q "volumes:" "$compose_file" && \
       grep -q "prometheus_data:" "$compose_file" && \
       grep -q "grafana_data:" "$compose_file"; then
        test_pass "Docker Volume Configuration"
    else
        test_fail "Docker Volume Configuration" "Missing or invalid volume configuration"
    fi
}

# Test 7: Health Check Configuration
test_health_check_config() {
    test_start "Health Check Configuration"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if grep -q "healthcheck:" "$compose_file" && \
       grep -q "curl.*localhost:2333" "$compose_file"; then
        test_pass "Health Check Configuration"
    else
        test_fail "Health Check Configuration" "Missing or invalid health check configuration"
    fi
}

# Test 8: Environment Variables
test_environment_variables() {
    test_start "Environment Variables"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    local required_env_vars=("RUST_LOG" "LAVALINK_SERVER_PASSWORD" "LAVALINK_SERVER_PORT")
    local missing_vars=()
    
    for var in "${required_env_vars[@]}"; do
        if ! grep -q "$var" "$compose_file"; then
            missing_vars+=("$var")
        fi
    done
    
    if [[ ${#missing_vars[@]} -eq 0 ]]; then
        test_pass "Environment Variables"
    else
        test_fail "Environment Variables" "Missing environment variables: ${missing_vars[*]}"
    fi
}

# Test 9: Port Configuration
test_port_configuration() {
    test_start "Port Configuration"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if grep -q "2333:2333" "$compose_file" && \
       grep -q "9091:9090" "$compose_file" && \
       grep -q "3000:3000" "$compose_file"; then
        test_pass "Port Configuration"
    else
        test_fail "Port Configuration" "Missing or invalid port configuration"
    fi
}

# Test 10: Resource Limits
test_resource_limits() {
    test_start "Resource Limits"
    
    local compose_file="$SCRIPT_DIR/docker-compose.yml"
    
    if grep -q "deploy:" "$compose_file" && \
       grep -q "resources:" "$compose_file" && \
       grep -q "limits:" "$compose_file" && \
       grep -q "memory:" "$compose_file"; then
        test_pass "Resource Limits"
    else
        test_fail "Resource Limits" "Missing or invalid resource limits"
    fi
}

# Initialize test environment
setup_test_env() {
    log_info "Setting up Docker test environment..."
    
    # Initialize test log
    echo "=== Lavalink Rust Docker Test Results ===" > "$TEST_LOG"
    echo "Date: $(date)" >> "$TEST_LOG"
    echo "Docker Version: $(docker --version)" >> "$TEST_LOG"
    echo "" >> "$TEST_LOG"
    
    log_success "Docker test environment ready"
}

# Main test runner
run_docker_tests() {
    log_info "Starting Lavalink Rust Docker Test Suite"
    
    check_docker
    setup_test_env
    
    # Run all Docker tests
    test_build_standard_dockerfile
    test_build_alpine_dockerfile
    test_docker_compose_validation
    test_docker_compose_services
    test_docker_network_config
    test_docker_volume_config
    test_health_check_config
    test_environment_variables
    test_port_configuration
    test_resource_limits
    
    # Print summary
    echo ""
    log_info "=== Docker Test Summary ==="
    log_info "Total tests: $TESTS_TOTAL"
    log_success "Passed: $TESTS_PASSED"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Failed: $TESTS_FAILED"
    else
        log_success "Failed: $TESTS_FAILED"
    fi
    
    echo ""
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "All Docker tests passed! ✓"
        echo "Test results saved to: $TEST_LOG"
        exit 0
    else
        log_error "Some Docker tests failed! ✗"
        echo "Check test results in: $TEST_LOG"
        exit 1
    fi
}

# Run Docker tests
run_docker_tests

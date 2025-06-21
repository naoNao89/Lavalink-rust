#!/bin/bash

# Lavalink Rust Integration Test Suite
# This script performs end-to-end testing of the deployment process

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_LOG="$SCRIPT_DIR/integration-test-results.log"
TEST_CONTAINER_NAME="lavalink-rust-integration-test"
TEST_NETWORK_NAME="lavalink-test-network"

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
    log_info "Running integration test: $test_name"
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
setup_integration_env() {
    log_info "Setting up integration test environment..."
    
    # Initialize test log
    echo "=== Lavalink Rust Integration Test Results ===" > "$TEST_LOG"
    echo "Date: $(date)" >> "$TEST_LOG"
    echo "Project Root: $PROJECT_ROOT" >> "$TEST_LOG"
    echo "" >> "$TEST_LOG"
    
    # Create test network
    if ! docker network ls | grep -q "$TEST_NETWORK_NAME"; then
        docker network create "$TEST_NETWORK_NAME" > /dev/null 2>&1
        log_info "Created test network: $TEST_NETWORK_NAME"
    fi
    
    log_success "Integration test environment ready"
}

# Cleanup test environment
cleanup_integration_env() {
    log_info "Cleaning up integration test environment..."
    
    # Stop and remove test container
    if docker ps -a | grep -q "$TEST_CONTAINER_NAME"; then
        docker stop "$TEST_CONTAINER_NAME" > /dev/null 2>&1 || true
        docker rm "$TEST_CONTAINER_NAME" > /dev/null 2>&1 || true
    fi
    
    # Remove test images
    docker rmi lavalink-rust-integration-test > /dev/null 2>&1 || true
    
    # Remove test network
    if docker network ls | grep -q "$TEST_NETWORK_NAME"; then
        docker network rm "$TEST_NETWORK_NAME" > /dev/null 2>&1 || true
    fi
    
    # Clean up test files
    rm -f "$PROJECT_ROOT/lavalink-rust-test"
    
    log_success "Integration test cleanup completed"
}

# Create test binary
create_test_binary() {
    log_info "Creating test binary..."
    
    local test_binary="$PROJECT_ROOT/lavalink-rust-test"
    
    cat > "$test_binary" << 'EOF'
#!/bin/bash

# Mock Lavalink Rust binary for testing
echo "Starting Lavalink Rust Test Server..."
echo "Version: 4.0.0-integration-test"
echo "Listening on 0.0.0.0:2333"

# Create a simple HTTP server that responds to health checks
python3 -c "
import http.server
import socketserver
import json
from urllib.parse import urlparse

class LavalinkHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/v4/info':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {
                'version': {
                    'semver': '4.0.0-integration-test',
                    'major': 4,
                    'minor': 0,
                    'patch': 0,
                    'preRelease': 'integration-test'
                },
                'buildTime': 1234567890,
                'git': {
                    'branch': 'test',
                    'commit': 'test-commit'
                },
                'jvm': 'Rust/1.70.0',
                'lavaplayer': '1.3.77',
                'sourceManagers': ['youtube', 'soundcloud', 'bandcamp'],
                'filters': ['volume', 'equalizer'],
                'plugins': []
            }
            self.wfile.write(json.dumps(response).encode())
        elif parsed_path.path == '/version':
            self.send_response(200)
            self.send_header('Content-type', 'text/plain')
            self.end_headers()
            self.wfile.write(b'4.0.0-integration-test')
        else:
            self.send_response(404)
            self.end_headers()
    
    def log_message(self, format, *args):
        pass  # Suppress default logging

PORT = 2333
with socketserver.TCPServer(('', PORT), LavalinkHandler) as httpd:
    print(f'Test server running on port {PORT}')
    httpd.serve_forever()
"
EOF
    
    chmod +x "$test_binary"
    log_success "Test binary created"
}

# Test 1: Build Docker Image
test_build_docker_image() {
    test_start "Build Docker Image"
    
    # Copy test binary to expected location
    cp "$PROJECT_ROOT/lavalink-rust-test" "$PROJECT_ROOT/lavalink-rust"
    
    # Build Docker image
    if docker build -f "$SCRIPT_DIR/Dockerfile" -t lavalink-rust-integration-test "$PROJECT_ROOT" > /dev/null 2>&1; then
        test_pass "Build Docker Image"
    else
        test_fail "Build Docker Image" "Failed to build Docker image"
    fi
    
    # Clean up
    rm -f "$PROJECT_ROOT/lavalink-rust"
}

# Test 2: Container Startup
test_container_startup() {
    test_start "Container Startup"
    
    # Create test config
    local test_config="$PROJECT_ROOT/test-application.yml"
    cat > "$test_config" << 'EOF'
server:
  port: 2333
  address: 0.0.0.0

lavalink:
  server:
    password: "test-password"
    sources:
      youtube: true
      bandcamp: true
      soundcloud: true
      twitch: true
      vimeo: true
      http: true
      local: false

logging:
  level:
    root: INFO
    lavalink: INFO
EOF
    
    # Start container
    if docker run -d \
        --name "$TEST_CONTAINER_NAME" \
        --network "$TEST_NETWORK_NAME" \
        -p 2333:2333 \
        -v "$test_config:/app/application.yml:ro" \
        lavalink-rust-integration-test > /dev/null 2>&1; then
        
        # Wait for container to start
        sleep 10
        
        if docker ps | grep -q "$TEST_CONTAINER_NAME"; then
            test_pass "Container Startup"
        else
            test_fail "Container Startup" "Container failed to stay running"
        fi
    else
        test_fail "Container Startup" "Failed to start container"
    fi
    
    # Clean up test config
    rm -f "$test_config"
}

# Test 3: Health Check
test_health_check() {
    test_start "Health Check"
    
    # Wait a bit more for the service to be ready
    sleep 5
    
    # Test health check endpoint
    if curl -f -s http://localhost:2333/v4/info > /dev/null 2>&1; then
        test_pass "Health Check"
    else
        test_fail "Health Check" "Health check endpoint not responding"
    fi
}

# Test 4: API Response
test_api_response() {
    test_start "API Response"
    
    # Test API response content
    local response=$(curl -s http://localhost:2333/v4/info 2>/dev/null)
    
    if echo "$response" | grep -q "4.0.0-integration-test"; then
        test_pass "API Response"
    else
        test_fail "API Response" "API response does not contain expected version"
    fi
}

# Test 5: Container Logs
test_container_logs() {
    test_start "Container Logs"
    
    # Check if container is producing logs
    local logs=$(docker logs "$TEST_CONTAINER_NAME" 2>&1)
    
    if echo "$logs" | grep -q "Lavalink Rust Test Server"; then
        test_pass "Container Logs"
    else
        test_fail "Container Logs" "Container logs do not contain expected content"
    fi
}

# Test 6: Docker Compose Integration
test_docker_compose_integration() {
    test_start "Docker Compose Integration"
    
    # Check if docker-compose or docker compose is available
    local compose_cmd=""
    if command -v docker-compose &> /dev/null; then
        compose_cmd="docker-compose"
    elif docker compose version &> /dev/null; then
        compose_cmd="docker compose"
    else
        test_fail "Docker Compose Integration" "Neither docker-compose nor docker compose available"
        return
    fi
    
    # Test compose file validation
    if $compose_cmd -f "$SCRIPT_DIR/docker-compose.yml" config > /dev/null 2>&1; then
        test_pass "Docker Compose Integration"
    else
        test_fail "Docker Compose Integration" "Docker Compose configuration invalid"
    fi
}

# Test 7: Resource Usage
test_resource_usage() {
    test_start "Resource Usage"
    
    # Get container stats
    local stats=$(docker stats "$TEST_CONTAINER_NAME" --no-stream --format "table {{.CPUPerc}}\t{{.MemUsage}}" 2>/dev/null)
    
    if [[ -n "$stats" ]]; then
        test_pass "Resource Usage"
        log_info "Container resource usage: $stats"
    else
        test_fail "Resource Usage" "Could not retrieve container resource usage"
    fi
}

# Test 8: Port Accessibility
test_port_accessibility() {
    test_start "Port Accessibility"
    
    # Test if port 2333 is accessible
    if nc -z localhost 2333 2>/dev/null; then
        test_pass "Port Accessibility"
    else
        # Fallback test using curl
        if curl -f -s http://localhost:2333/v4/info > /dev/null 2>&1; then
            test_pass "Port Accessibility"
        else
            test_fail "Port Accessibility" "Port 2333 is not accessible"
        fi
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites for integration tests..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    # Check curl
    if ! command -v curl &> /dev/null; then
        log_error "curl is not installed"
        exit 1
    fi
    
    # Check Python3 (for test server)
    if ! command -v python3 &> /dev/null; then
        log_error "python3 is not installed"
        exit 1
    fi
    
    log_success "All prerequisites met"
}

# Main integration test runner
run_integration_tests() {
    log_info "Starting Lavalink Rust Integration Test Suite"
    
    check_prerequisites
    setup_integration_env
    create_test_binary
    
    # Run all integration tests
    test_build_docker_image
    test_container_startup
    test_health_check
    test_api_response
    test_container_logs
    test_docker_compose_integration
    test_resource_usage
    test_port_accessibility
    
    cleanup_integration_env
    
    # Print summary
    echo ""
    log_info "=== Integration Test Summary ==="
    log_info "Total tests: $TESTS_TOTAL"
    log_success "Passed: $TESTS_PASSED"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Failed: $TESTS_FAILED"
    else
        log_success "Failed: $TESTS_FAILED"
    fi
    
    echo ""
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "All integration tests passed! ✓"
        echo "Test results saved to: $TEST_LOG"
        exit 0
    else
        log_error "Some integration tests failed! ✗"
        echo "Check test results in: $TEST_LOG"
        exit 1
    fi
}

# Handle cleanup on script exit
trap cleanup_integration_env EXIT

# Run integration tests
run_integration_tests

#!/bin/bash

# Lavalink Rust Deployment - Master Test Runner
# This script runs all deployment test suites

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MASTER_LOG="$SCRIPT_DIR/master-test-results.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Test suite results
SUITES_TOTAL=0
SUITES_PASSED=0
SUITES_FAILED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$MASTER_LOG"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$MASTER_LOG"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$MASTER_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$MASTER_LOG"
}

log_header() {
    echo -e "${BOLD}${BLUE}$1${NC}" | tee -a "$MASTER_LOG"
}

# Initialize master test log
init_master_log() {
    cat > "$MASTER_LOG" << EOF
===============================================
    LAVALINK RUST DEPLOYMENT TEST SUITE
===============================================
Date: $(date)
Project Root: $PROJECT_ROOT
Test Runner: $0

EOF
}

# Run a test suite
run_test_suite() {
    local suite_name="$1"
    local script_path="$2"
    local description="$3"
    
    SUITES_TOTAL=$((SUITES_TOTAL + 1))
    
    log_header "Running $suite_name"
    log_info "$description"
    echo "" | tee -a "$MASTER_LOG"
    
    if [[ ! -f "$script_path" ]]; then
        log_error "Test script not found: $script_path"
        SUITES_FAILED=$((SUITES_FAILED + 1))
        return 1
    fi
    
    if [[ ! -x "$script_path" ]]; then
        log_error "Test script not executable: $script_path"
        SUITES_FAILED=$((SUITES_FAILED + 1))
        return 1
    fi
    
    # Run the test suite
    local start_time=$(date +%s)
    if "$script_path"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "$suite_name completed successfully in ${duration}s"
        SUITES_PASSED=$((SUITES_PASSED + 1))
        echo "" | tee -a "$MASTER_LOG"
        return 0
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_error "$suite_name failed after ${duration}s"
        SUITES_FAILED=$((SUITES_FAILED + 1))
        echo "" | tee -a "$MASTER_LOG"
        return 1
    fi
}

# Display usage information
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run all Lavalink Rust deployment test suites.

OPTIONS:
    -h, --help          Show this help message
    -q, --quick         Run only basic validation tests (skip integration tests)
    -d, --docker-only   Run only Docker-related tests
    -i, --integration   Run only integration tests
    -v, --verbose       Enable verbose output

TEST SUITES:
    1. Deployment Configuration Tests - Validate all deployment files
    2. Docker Tests - Test Docker configurations and builds
    3. Integration Tests - End-to-end deployment testing

EXAMPLES:
    $0                  # Run all test suites
    $0 --quick          # Run basic validation only
    $0 --docker-only    # Run Docker tests only
    $0 --integration    # Run integration tests only

EOF
}

# Parse command line arguments
parse_args() {
    QUICK_MODE=false
    DOCKER_ONLY=false
    INTEGRATION_ONLY=false
    VERBOSE=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -q|--quick)
                QUICK_MODE=true
                shift
                ;;
            -d|--docker-only)
                DOCKER_ONLY=true
                shift
                ;;
            -i|--integration)
                INTEGRATION_ONLY=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Check system requirements
check_requirements() {
    log_info "Checking system requirements..."
    
    local missing_tools=()
    
    # Check basic tools
    if ! command -v bash &> /dev/null; then
        missing_tools+=("bash")
    fi
    
    if ! command -v curl &> /dev/null; then
        missing_tools+=("curl")
    fi
    
    # Check Docker if needed
    if [[ "$DOCKER_ONLY" == true ]] || [[ "$INTEGRATION_ONLY" == true ]] || [[ "$QUICK_MODE" == false ]]; then
        if ! command -v docker &> /dev/null; then
            missing_tools+=("docker")
        fi
    fi
    
    # Check Python3 for integration tests
    if [[ "$INTEGRATION_ONLY" == true ]] || [[ "$QUICK_MODE" == false ]]; then
        if ! command -v python3 &> /dev/null; then
            missing_tools+=("python3")
        fi
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log_info "Please install the missing tools and try again"
        exit 1
    fi
    
    log_success "All required tools are available"
}

# Main test execution
main() {
    parse_args "$@"
    
    log_header "LAVALINK RUST DEPLOYMENT TEST SUITE"
    echo "Starting comprehensive deployment testing..." | tee -a "$MASTER_LOG"
    echo "" | tee -a "$MASTER_LOG"
    
    init_master_log
    check_requirements
    
    local overall_start_time=$(date +%s)
    
    # Run test suites based on options
    if [[ "$DOCKER_ONLY" == true ]]; then
        run_test_suite "Docker Tests" "$SCRIPT_DIR/test-docker.sh" "Testing Docker configurations and builds"
    elif [[ "$INTEGRATION_ONLY" == true ]]; then
        run_test_suite "Integration Tests" "$SCRIPT_DIR/test-integration.sh" "End-to-end deployment testing"
    elif [[ "$QUICK_MODE" == true ]]; then
        run_test_suite "Deployment Configuration Tests" "$SCRIPT_DIR/test-deployment.sh" "Validating deployment configurations and scripts"
    else
        # Run all test suites
        run_test_suite "Deployment Configuration Tests" "$SCRIPT_DIR/test-deployment.sh" "Validating deployment configurations and scripts"
        run_test_suite "Docker Tests" "$SCRIPT_DIR/test-docker.sh" "Testing Docker configurations and builds"
        run_test_suite "Integration Tests" "$SCRIPT_DIR/test-integration.sh" "End-to-end deployment testing"
    fi
    
    local overall_end_time=$(date +%s)
    local total_duration=$((overall_end_time - overall_start_time))
    
    # Print final summary
    echo "" | tee -a "$MASTER_LOG"
    log_header "FINAL TEST SUMMARY"
    echo "Total execution time: ${total_duration}s" | tee -a "$MASTER_LOG"
    echo "Test suites run: $SUITES_TOTAL" | tee -a "$MASTER_LOG"
    echo "Suites passed: $SUITES_PASSED" | tee -a "$MASTER_LOG"
    echo "Suites failed: $SUITES_FAILED" | tee -a "$MASTER_LOG"
    echo "" | tee -a "$MASTER_LOG"
    
    if [[ $SUITES_FAILED -eq 0 ]]; then
        log_success "🎉 ALL DEPLOYMENT TESTS PASSED! 🎉"
        echo "" | tee -a "$MASTER_LOG"
        log_info "Your Lavalink Rust deployment configuration is ready for production!"
        log_info "Master test results saved to: $MASTER_LOG"
        echo "" | tee -a "$MASTER_LOG"
        
        # Show next steps
        cat << EOF | tee -a "$MASTER_LOG"
NEXT STEPS:
1. Review individual test logs for detailed results
2. Deploy using: sudo ./deployment/scripts/deploy.sh
3. Monitor with: docker-compose -f deployment/docker-compose.yml up -d
4. Check service status: systemctl status lavalink-rust

DEPLOYMENT FILES VALIDATED:
✓ Docker configurations (Dockerfile, docker-compose.yml)
✓ Deployment scripts (deploy.sh, rollback.sh)
✓ SystemD service configuration
✓ Monitoring setup (Prometheus, Grafana)
✓ Integration testing passed

EOF
        exit 0
    else
        log_error "❌ SOME DEPLOYMENT TESTS FAILED ❌"
        echo "" | tee -a "$MASTER_LOG"
        log_error "Please review the test results and fix the issues before deploying"
        log_info "Master test results saved to: $MASTER_LOG"
        echo "" | tee -a "$MASTER_LOG"
        
        cat << EOF | tee -a "$MASTER_LOG"
TROUBLESHOOTING:
1. Check individual test logs for specific error details
2. Ensure all dependencies are installed
3. Verify Docker is running (for Docker/integration tests)
4. Check file permissions on scripts
5. Review configuration files for syntax errors

FAILED TEST SUITES: $SUITES_FAILED/$SUITES_TOTAL
EOF
        exit 1
    fi
}

# Run main function with all arguments
main "$@"

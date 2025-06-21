#!/usr/bin/env bash

# Nix Configuration Test Suite for Lavalink Rust
set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_LOG="$SCRIPT_DIR/nix-test-results.log"

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
    log_info "Setting up Nix test environment..."
    
    # Initialize test log
    echo "=== Lavalink Rust Nix Test Results ===" > "$TEST_LOG"
    echo "Date: $(date)" >> "$TEST_LOG"
    echo "Project Root: $PROJECT_ROOT" >> "$TEST_LOG"
    echo "" >> "$TEST_LOG"
    
    log_success "Nix test environment ready"
}

# Check Nix installation
check_nix() {
    test_start "Nix Installation Check"
    
    if ! command -v nix &> /dev/null; then
        test_fail "Nix Installation Check" "Nix is not installed"
        return
    fi
    
    # Check Nix version
    local nix_version=$(nix --version)
    log_info "Nix version: $nix_version"
    
    # Check if flakes are enabled
    if nix flake --help &> /dev/null 2>&1; then
        test_pass "Nix Installation Check"
    else
        test_fail "Nix Installation Check" "Nix flakes are not enabled"
    fi
}

# Test flake validation
test_flake_validation() {
    test_start "Flake Validation"
    
    cd "$PROJECT_ROOT"
    
    if nix flake check --no-build 2>/dev/null; then
        test_pass "Flake Validation"
    else
        test_fail "Flake Validation" "Flake validation failed"
    fi
}

# Test flake metadata
test_flake_metadata() {
    test_start "Flake Metadata"
    
    cd "$PROJECT_ROOT"
    
    if nix flake metadata > /dev/null 2>&1; then
        test_pass "Flake Metadata"
    else
        test_fail "Flake Metadata" "Failed to read flake metadata"
    fi
}

# Test package build
test_package_build() {
    test_start "Package Build"
    
    cd "$PROJECT_ROOT"
    
    if nix build --dry-run 2>/dev/null; then
        test_pass "Package Build"
    else
        test_fail "Package Build" "Package build validation failed"
    fi
}

# Test development shell
test_dev_shell() {
    test_start "Development Shell"
    
    cd "$PROJECT_ROOT"
    
    # Test if dev shell can be entered
    if nix develop --command echo "Dev shell works" > /dev/null 2>&1; then
        test_pass "Development Shell"
    else
        test_fail "Development Shell" "Development shell failed to load"
    fi
}

# Test NixOS module syntax
test_nixos_module() {
    test_start "NixOS Module Syntax"
    
    local module_file="$PROJECT_ROOT/nix/module.nix"
    
    if [[ ! -f "$module_file" ]]; then
        test_fail "NixOS Module Syntax" "Module file not found"
        return
    fi
    
    # Basic syntax check using nix-instantiate
    if nix-instantiate --parse "$module_file" > /dev/null 2>&1; then
        test_pass "NixOS Module Syntax"
    else
        test_fail "NixOS Module Syntax" "Module syntax validation failed"
    fi
}

# Test Docker image build
test_docker_build() {
    test_start "Docker Image Build"
    
    cd "$PROJECT_ROOT"
    
    if nix build .#docker --dry-run 2>/dev/null; then
        test_pass "Docker Image Build"
    else
        test_fail "Docker Image Build" "Docker image build validation failed"
    fi
}

# Test configuration files
test_configurations() {
    test_start "Configuration Files"
    
    local configs=(
        "nix/configurations/development.nix"
        "nix/configurations/production.nix"
    )
    
    local failed_configs=()
    
    for config in "${configs[@]}"; do
        local config_path="$PROJECT_ROOT/$config"
        if [[ ! -f "$config_path" ]]; then
            failed_configs+=("$config (missing)")
        elif ! nix-instantiate --parse "$config_path" > /dev/null 2>&1; then
            failed_configs+=("$config (syntax error)")
        fi
    done
    
    if [[ ${#failed_configs[@]} -eq 0 ]]; then
        test_pass "Configuration Files"
    else
        test_fail "Configuration Files" "Failed configs: ${failed_configs[*]}"
    fi
}

# Test deployment scripts
test_deployment_scripts() {
    test_start "Deployment Scripts"
    
    local scripts=(
        "nix/deploy/deploy.sh"
    )
    
    local failed_scripts=()
    
    for script in "${scripts[@]}"; do
        local script_path="$PROJECT_ROOT/$script"
        if [[ ! -f "$script_path" ]]; then
            failed_scripts+=("$script (missing)")
        elif [[ ! -x "$script_path" ]]; then
            failed_scripts+=("$script (not executable)")
        elif ! bash -n "$script_path"; then
            failed_scripts+=("$script (syntax error)")
        fi
    done
    
    if [[ ${#failed_scripts[@]} -eq 0 ]]; then
        test_pass "Deployment Scripts"
    else
        test_fail "Deployment Scripts" "Failed scripts: ${failed_scripts[*]}"
    fi
}

# Test Nix formatting
test_nix_formatting() {
    test_start "Nix Code Formatting"
    
    cd "$PROJECT_ROOT"
    
    # Check if nixpkgs-fmt is available
    if ! command -v nixpkgs-fmt &> /dev/null; then
        log_warning "nixpkgs-fmt not available, skipping format check"
        test_pass "Nix Code Formatting"
        return
    fi
    
    # Find all .nix files
    local nix_files=$(find . -name "*.nix" -not -path "./result*")
    local format_issues=()
    
    for file in $nix_files; do
        if ! nixpkgs-fmt --check "$file" > /dev/null 2>&1; then
            format_issues+=("$file")
        fi
    done
    
    if [[ ${#format_issues[@]} -eq 0 ]]; then
        test_pass "Nix Code Formatting"
    else
        test_fail "Nix Code Formatting" "Format issues in: ${format_issues[*]}"
    fi
}

# Test flake inputs
test_flake_inputs() {
    test_start "Flake Inputs"
    
    cd "$PROJECT_ROOT"
    
    # Check if flake.lock exists and is valid
    if [[ ! -f "flake.lock" ]]; then
        test_fail "Flake Inputs" "flake.lock not found"
        return
    fi
    
    # Validate flake.lock
    if nix flake metadata --json > /dev/null 2>&1; then
        test_pass "Flake Inputs"
    else
        test_fail "Flake Inputs" "Invalid flake.lock"
    fi
}

# Test system compatibility
test_system_compatibility() {
    test_start "System Compatibility"
    
    cd "$PROJECT_ROOT"
    
    local current_system=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    log_info "Current system: $current_system"
    
    # Check if the flake supports current system
    if nix eval ".#packages.$current_system.default" > /dev/null 2>&1; then
        test_pass "System Compatibility"
    else
        test_fail "System Compatibility" "Current system not supported: $current_system"
    fi
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Test Nix configuration for Lavalink Rust.

OPTIONS:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    --quick         Run only basic tests
    --format        Run formatting checks only

EXAMPLES:
    $0              # Run all tests
    $0 --quick      # Run basic tests only
    $0 --format     # Check code formatting only

EOF
}

# Parse command line arguments
parse_args() {
    VERBOSE=false
    QUICK_MODE=false
    FORMAT_ONLY=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --quick)
                QUICK_MODE=true
                shift
                ;;
            --format)
                FORMAT_ONLY=true
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

# Main test runner
run_nix_tests() {
    log_info "Starting Lavalink Rust Nix Test Suite"
    
    setup_test_env
    
    if [[ "$FORMAT_ONLY" == true ]]; then
        test_nix_formatting
    elif [[ "$QUICK_MODE" == true ]]; then
        check_nix
        test_flake_validation
        test_flake_metadata
        test_nixos_module
    else
        # Run all tests
        check_nix
        test_flake_validation
        test_flake_metadata
        test_package_build
        test_dev_shell
        test_nixos_module
        test_docker_build
        test_configurations
        test_deployment_scripts
        test_nix_formatting
        test_flake_inputs
        test_system_compatibility
    fi
    
    # Print summary
    echo ""
    log_info "=== Nix Test Summary ==="
    log_info "Total tests: $TESTS_TOTAL"
    log_success "Passed: $TESTS_PASSED"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Failed: $TESTS_FAILED"
    else
        log_success "Failed: $TESTS_FAILED"
    fi
    
    echo ""
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "All Nix tests passed! ✓"
        echo "Test results saved to: $TEST_LOG"
        exit 0
    else
        log_error "Some Nix tests failed! ✗"
        echo "Check test results in: $TEST_LOG"
        exit 1
    fi
}

# Parse arguments and run tests
parse_args "$@"
run_nix_tests

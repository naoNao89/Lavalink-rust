# Lavalink Rust Deployment Testing

This directory contains comprehensive test suites for validating the Lavalink Rust deployment configuration. The tests ensure that all deployment components are properly configured and ready for production use.

## Test Suites

### 1. Deployment Configuration Tests (`test-deployment.sh`)
Validates all deployment files and configurations:
- Docker Compose configuration syntax
- Dockerfile validation (standard and Alpine)
- Deployment and rollback script validation
- SystemD service file validation
- Monitoring configuration (Prometheus/Grafana)
- Required dependencies check
- Configuration file validation
- Directory structure validation

### 2. Docker Tests (`test-docker.sh`)
Tests Docker-specific functionality:
- Docker image builds (standard and Alpine)
- Docker Compose validation
- Service configuration checks
- Network and volume configuration
- Health check configuration
- Environment variables
- Port configuration
- Resource limits

### 3. Integration Tests (`test-integration.sh`)
End-to-end deployment testing:
- Docker image building
- Container startup and health
- API endpoint testing
- Container logging validation
- Resource usage monitoring
- Port accessibility testing
- Docker Compose integration

## Quick Start

### Run All Tests
```bash
# Run all test suites
./deployment/run-all-tests.sh

# Run with verbose output
./deployment/run-all-tests.sh --verbose
```

### Run Specific Test Suites
```bash
# Run only basic validation tests (no Docker required)
./deployment/run-all-tests.sh --quick

# Run only Docker tests
./deployment/run-all-tests.sh --docker-only

# Run only integration tests
./deployment/run-all-tests.sh --integration
```

### Run Individual Test Scripts
```bash
# Basic deployment validation
./deployment/test-deployment.sh

# Docker configuration tests
./deployment/test-docker.sh

# Integration tests
./deployment/test-integration.sh
```

## Prerequisites

### Basic Tests
- `bash` (4.0+)
- `curl`
- `python3` (for YAML validation)

### Docker Tests
- `docker` (20.0+)
- `docker-compose` or `docker compose`

### Integration Tests
- All of the above
- `netcat` (optional, for port testing)

## Test Results

Test results are saved to log files:
- `test-results.log` - Basic deployment tests
- `docker-test-results.log` - Docker tests
- `integration-test-results.log` - Integration tests
- `master-test-results.log` - Master test runner results

## Understanding Test Output

### Test Status Indicators
- ✓ **PASS** - Test completed successfully
- ✗ **FAIL** - Test failed with error details
- ⚠ **WARNING** - Test passed with warnings

### Color Coding
- 🔵 **BLUE** - Informational messages
- 🟢 **GREEN** - Success messages
- 🟡 **YELLOW** - Warning messages
- 🔴 **RED** - Error messages

## Common Issues and Solutions

### Docker Not Available
If Docker tests fail with "Docker daemon is not running":
```bash
# Start Docker daemon (varies by system)
sudo systemctl start docker  # Linux
# or
open -a Docker  # macOS
```

### Permission Issues
If scripts fail with permission errors:
```bash
# Make scripts executable
chmod +x deployment/*.sh
```

### Missing Dependencies
Install missing tools:
```bash
# Ubuntu/Debian
sudo apt-get install curl python3 netcat-openbsd

# macOS
brew install curl python3 netcat

# CentOS/RHEL
sudo yum install curl python3 nc
```

## Continuous Integration

These tests are designed to run in CI/CD pipelines. Example GitHub Actions usage:

```yaml
- name: Test Deployment Configuration
  run: ./deployment/run-all-tests.sh --quick

- name: Test Docker Configuration
  run: ./deployment/test-docker.sh
  if: runner.os == 'Linux'

- name: Run Integration Tests
  run: ./deployment/test-integration.sh
  if: runner.os == 'Linux'
```

## Test Development

### Adding New Tests

1. **Basic Tests**: Add to `test-deployment.sh`
2. **Docker Tests**: Add to `test-docker.sh`
3. **Integration Tests**: Add to `test-integration.sh`

### Test Framework Functions
```bash
test_start "Test Name"          # Initialize test
test_pass "Test Name"           # Mark test as passed
test_fail "Test Name" "Reason"  # Mark test as failed
log_info "Message"              # Log informational message
log_success "Message"           # Log success message
log_warning "Message"           # Log warning message
log_error "Message"             # Log error message
```

## Production Deployment Validation

Before deploying to production:

1. **Run Full Test Suite**:
   ```bash
   ./deployment/run-all-tests.sh
   ```

2. **Review Test Results**:
   - Check all tests pass
   - Review any warnings
   - Verify configuration matches requirements

3. **Manual Verification**:
   - Review generated configurations
   - Validate security settings
   - Confirm resource allocations

4. **Deploy with Confidence**:
   ```bash
   sudo ./deployment/scripts/deploy.sh
   ```

## Support

If tests fail or you encounter issues:

1. Check the detailed log files
2. Review the troubleshooting section in test output
3. Verify all prerequisites are met
4. Check file permissions and paths
5. Ensure Docker is running (for Docker tests)

For additional help, refer to the main project documentation or deployment guides.

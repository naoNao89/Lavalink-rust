# Nix-based testing infrastructure
{ pkgs, lib, lavalink-rust }:

let
  # Test configuration
  testConfig = pkgs.writeText "test-application.yml" ''
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
  '';

  # Test script for basic functionality
  basicTest = pkgs.writeShellScript "basic-test.sh" ''
    set -euo pipefail
    
    echo "Starting Lavalink Rust basic test..."
    
    # Start Lavalink in background
    ${lavalink-rust}/bin/lavalink-rust --config ${testConfig} &
    LAVALINK_PID=$!
    
    # Wait for startup
    echo "Waiting for Lavalink to start..."
    sleep 10
    
    # Test health endpoint
    echo "Testing health endpoint..."
    if curl -f -s http://localhost:2333/v4/info > /dev/null; then
        echo "✓ Health endpoint responding"
    else
        echo "✗ Health endpoint not responding"
        kill $LAVALINK_PID || true
        exit 1
    fi
    
    # Test API response
    echo "Testing API response..."
    RESPONSE=$(curl -s http://localhost:2333/v4/info)
    if echo "$RESPONSE" | grep -q "version"; then
        echo "✓ API response contains version info"
    else
        echo "✗ API response missing version info"
        kill $LAVALINK_PID || true
        exit 1
    fi
    
    # Cleanup
    kill $LAVALINK_PID || true
    echo "✓ Basic test completed successfully"
  '';

  # Integration test with NixOS VM
  integrationTest = pkgs.nixosTest {
    name = "lavalink-rust-integration";
    
    nodes.machine = { config, pkgs, ... }: {
      imports = [ ../module.nix ];
      
      services.lavalink-rust = {
        enable = true;
        settings = {
          server = {
            port = 2333;
            address = "0.0.0.0";
          };
          lavalink.server.password = "test-password";
        };
      };
      
      # Required for testing
      environment.systemPackages = with pkgs; [ curl jq ];
    };

    testScript = ''
      machine.start()
      machine.wait_for_unit("lavalink-rust.service")
      machine.wait_for_open_port(2333)
      
      # Test health endpoint
      machine.succeed("curl -f http://localhost:2333/v4/info")
      
      # Test API response format
      response = machine.succeed("curl -s http://localhost:2333/v4/info | jq -r .version.semver")
      assert "4.0.0" in response
      
      # Test service status
      machine.succeed("systemctl is-active lavalink-rust")
      
      print("✓ Integration test completed successfully")
    '';
  };

  # Performance test
  performanceTest = pkgs.writeShellScript "performance-test.sh" ''
    set -euo pipefail
    
    echo "Starting Lavalink Rust performance test..."
    
    # Start Lavalink
    ${lavalink-rust}/bin/lavalink-rust --config ${testConfig} &
    LAVALINK_PID=$!
    
    # Wait for startup
    sleep 10
    
    # Performance test with multiple concurrent requests
    echo "Running performance test with 100 concurrent requests..."
    
    for i in {1..100}; do
        curl -s http://localhost:2333/v4/info > /dev/null &
    done
    
    wait
    
    echo "✓ Performance test completed"
    
    # Memory usage check
    MEMORY_KB=$(ps -o rss= -p $LAVALINK_PID)
    MEMORY_MB=$((MEMORY_KB / 1024))
    echo "Memory usage: ''${MEMORY_MB}MB"
    
    if [ $MEMORY_MB -lt 512 ]; then
        echo "✓ Memory usage within acceptable limits"
    else
        echo "⚠ Memory usage higher than expected: ''${MEMORY_MB}MB"
    fi
    
    # Cleanup
    kill $LAVALINK_PID || true
  '';

  # Docker test
  dockerTest = pkgs.writeShellScript "docker-test.sh" ''
    set -euo pipefail
    
    echo "Starting Docker test..."
    
    # Load Docker image
    docker load < ${(import ../docker.nix { inherit pkgs lavalink-rust; }).standard}
    
    # Run container
    CONTAINER_ID=$(docker run -d -p 2333:2333 lavalink-rust:latest)
    
    # Wait for startup
    sleep 15
    
    # Test health endpoint
    if curl -f -s http://localhost:2333/v4/info > /dev/null; then
        echo "✓ Docker container health check passed"
    else
        echo "✗ Docker container health check failed"
        docker logs $CONTAINER_ID
        docker stop $CONTAINER_ID
        docker rm $CONTAINER_ID
        exit 1
    fi
    
    # Cleanup
    docker stop $CONTAINER_ID
    docker rm $CONTAINER_ID
    
    echo "✓ Docker test completed successfully"
  '';

in {
  # Individual test derivations
  basic = pkgs.runCommand "lavalink-rust-basic-test" {
    buildInputs = [ pkgs.curl lavalink-rust ];
  } ''
    ${basicTest}
    touch $out
  '';

  performance = pkgs.runCommand "lavalink-rust-performance-test" {
    buildInputs = [ pkgs.curl pkgs.procps lavalink-rust ];
  } ''
    ${performanceTest}
    touch $out
  '';

  docker = pkgs.runCommand "lavalink-rust-docker-test" {
    buildInputs = [ pkgs.docker pkgs.curl ];
  } ''
    ${dockerTest}
    touch $out
  '';

  # NixOS integration test
  integration = integrationTest;

  # Test runner script
  runner = pkgs.writeShellScript "run-tests.sh" ''
    set -euo pipefail
    
    echo "🧪 Running Lavalink Rust Nix Tests"
    echo "=================================="
    
    TESTS_PASSED=0
    TESTS_FAILED=0
    
    run_test() {
        local test_name="$1"
        local test_command="$2"
        
        echo ""
        echo "Running $test_name..."
        
        if $test_command; then
            echo "✓ $test_name passed"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            echo "✗ $test_name failed"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
    }
    
    # Run tests
    run_test "Basic Test" "${basicTest}"
    run_test "Performance Test" "${performanceTest}"
    
    # Only run Docker test if Docker is available
    if command -v docker &> /dev/null && docker info &> /dev/null; then
        run_test "Docker Test" "${dockerTest}"
    else
        echo "⚠ Skipping Docker test (Docker not available)"
    fi
    
    # Summary
    echo ""
    echo "Test Summary:"
    echo "============="
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo "🎉 All tests passed!"
        exit 0
    else
        echo "❌ Some tests failed"
        exit 1
    fi
  '';

  # All tests combined
  all = pkgs.runCommand "lavalink-rust-all-tests" {
    buildInputs = [ pkgs.curl pkgs.procps lavalink-rust ];
  } ''
    ${basicTest}
    ${performanceTest}
    touch $out
  '';
}

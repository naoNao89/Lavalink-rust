# Lavalink Rust - Nix Deployment Guide

This guide covers deploying Lavalink Rust using the Nix ecosystem for reproducible, declarative deployments.

## 🚀 Quick Start

### Prerequisites

1. **Install Nix** with flakes enabled:
   ```bash
   # Install Nix
   curl -L https://nixos.org/nix/install | sh
   
   # Enable flakes (add to ~/.config/nix/nix.conf or /etc/nix/nix.conf)
   experimental-features = nix-command flakes
   ```

2. **For NixOS deployments**, install deploy-rs:
   ```bash
   nix profile install github:serokell/deploy-rs
   ```

### Development Environment

Enter the development shell with all dependencies:
```bash
nix develop
```

This provides:
- Rust toolchain with clippy and rustfmt
- All system dependencies (FFmpeg, Opus, etc.)
- Development tools (rust-analyzer, cargo-watch, etc.)
- Container tools (Docker, docker-compose)
- Nix tools (nixpkgs-fmt, nil)

### Building

```bash
# Build with Nix
nix build

# Build Docker image
nix build .#docker

# Run directly
nix run

# Run tests
nix flake check
```

## 📦 Package Outputs

The flake provides several outputs:

### Packages
- `packages.default` - The main Lavalink Rust binary
- `packages.lavalink-rust` - Same as default
- `packages.docker` - Docker image built with Nix

### Development
- `devShells.default` - Complete development environment

### Checks
- `checks.cargo-test` - Cargo test suite
- `checks.cargo-clippy` - Clippy linting
- `checks.cargo-fmt` - Format checking
- `checks.cargo-audit` - Security audit

## 🏗️ Deployment Options

### 1. NixOS Service Module

For NixOS systems, use the provided service module:

```nix
# configuration.nix
{
  imports = [ ./path/to/lavalink-rust/nix/module.nix ];
  
  services.lavalink-rust = {
    enable = true;
    openFirewall = true;
    
    settings = {
      server.port = 2333;
      lavalink.server.password = "your-secure-password";
    };
    
    environment = {
      RUST_LOG = "info";
    };
  };
}
```

### 2. Docker Deployment

```bash
# Build Docker image
nix build .#docker

# Load and run
docker load < result
docker run -p 2333:2333 lavalink-rust:latest
```

### 3. Automated Deployment

Use the deployment script for different environments:

```bash
# Deploy to development (local)
./nix/deploy/deploy.sh development

# Deploy to staging with tests
./nix/deploy/deploy.sh --test staging

# Dry-run production deployment
./nix/deploy/deploy.sh --dry-run production
```

## 🌍 Environment Configurations

### Development
- Debug logging enabled
- Local file sources enabled
- Development tools included
- Monitoring stack (Prometheus/Grafana)

### Production
- Optimized for security and performance
- HTTPS with automatic certificates
- Restricted firewall rules
- Automated backups and updates
- Monitoring and alerting

## 🔧 Configuration

### Service Configuration

The NixOS module accepts these options:

```nix
services.lavalink-rust = {
  enable = true;                    # Enable the service
  package = pkgs.lavalink-rust;     # Package to use
  user = "lavalink";                # Service user
  group = "lavalink";               # Service group
  dataDir = "/var/lib/lavalink-rust"; # Data directory
  openFirewall = false;             # Open firewall ports
  
  settings = {
    # Lavalink configuration (merged with defaults)
    server.port = 2333;
    lavalink.server.password = "password";
  };
  
  environment = {
    # Environment variables
    RUST_LOG = "info";
    RUST_BACKTRACE = "1";
  };
  
  extraArgs = [];                   # Extra command line arguments
};
```

### Custom Configuration File

```nix
services.lavalink-rust = {
  enable = true;
  configFile = ./custom-application.yml;
};
```

## 🧪 Testing

### Run All Tests
```bash
nix flake check
```

### Individual Test Suites
```bash
# Basic functionality test
nix run .#tests.basic

# Performance test
nix run .#tests.performance

# Docker test
nix run .#tests.docker

# NixOS integration test
nix build .#tests.integration
```

### Custom Test Runner
```bash
nix run .#tests.runner
```

## 🐳 Docker Images

Three Docker image variants are available:

### Standard Image
```bash
nix build .#docker.standard
```
- Full Debian-based image
- All dependencies included
- Production-ready

### Minimal Image
```bash
nix build .#docker.minimal
```
- Minimal dependencies
- Smaller size
- Basic functionality

### Debug Image
```bash
nix build .#docker.debug
```
- Additional debugging tools
- Development utilities
- Troubleshooting support

## 🔄 CI/CD Integration

### GitHub Actions

```yaml
name: Nix Build and Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: cachix/install-nix-action@v22
        with:
          extra_nix_config: |
            experimental-features = nix-command flakes
      
      - name: Build
        run: nix build
      
      - name: Test
        run: nix flake check
      
      - name: Build Docker image
        run: nix build .#docker
```

### GitLab CI

```yaml
build:
  image: nixos/nix:latest
  before_script:
    - nix-env -iA nixpkgs.git
  script:
    - nix build
    - nix flake check
```

## 🚀 Production Deployment

### 1. Prepare Target System

Ensure the target system has:
- NixOS installed
- SSH access configured
- Nix flakes enabled

### 2. Configure Secrets

Set up secrets management:
```bash
# Create password file
echo "your-secure-password" | sudo tee /etc/lavalink-password

# Set up ACME email
sudo mkdir -p /etc/nixos
echo "admin@yourdomain.com" | sudo tee /etc/nixos/acme-email
```

### 3. Deploy

```bash
# Test deployment
./nix/deploy/deploy.sh --dry-run production

# Deploy with tests
./nix/deploy/deploy.sh --test production
```

### 4. Verify

```bash
# Check service status
ssh production-host "systemctl status lavalink-rust"

# Test API
curl https://your-domain.com/v4/info
```

## 🔧 Troubleshooting

### Build Issues

```bash
# Clean build cache
nix store gc

# Rebuild with verbose output
nix build --verbose

# Check flake inputs
nix flake metadata
```

### Service Issues

```bash
# Check service logs
journalctl -u lavalink-rust -f

# Check configuration
nixos-rebuild dry-run --flake .#production

# Rollback if needed
nixos-rebuild switch --rollback
```

### Development Issues

```bash
# Update flake inputs
nix flake update

# Enter debug shell
nix develop --verbose

# Check environment
nix-shell --run env
```

## 📚 Advanced Usage

### Custom Overlays

```nix
# flake.nix
{
  outputs = { self, nixpkgs }: {
    overlays.default = final: prev: {
      lavalink-rust = self.packages.${final.system}.lavalink-rust;
    };
  };
}
```

### Multiple Environments

```nix
# Deploy to multiple environments
nix run .#deploy -- staging
nix run .#deploy -- production
```

### Custom Modules

```nix
# Import custom module
{
  imports = [ 
    ./lavalink-rust/nix/module.nix
    ./custom-monitoring.nix
  ];
}
```

## 🤝 Contributing

When contributing to the Nix configuration:

1. Test changes with `nix flake check`
2. Format Nix files with `nixpkgs-fmt`
3. Update documentation as needed
4. Test deployment in development environment

## 📖 Resources

- [Nix Manual](https://nixos.org/manual/nix/stable/)
- [NixOS Manual](https://nixos.org/manual/nixos/stable/)
- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [deploy-rs](https://github.com/serokell/deploy-rs)

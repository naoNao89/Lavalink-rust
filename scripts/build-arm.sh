#!/bin/bash
# ARM Cross-compilation Build Script for Lavalink-rust

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/target"
DIST_DIR="$PROJECT_ROOT/dist"

# ARM targets to build
ARM_TARGETS=(
    "armv7-unknown-linux-gnueabihf"
    "armv7-unknown-linux-musleabihf"
    "arm-unknown-linux-gnueabihf"
)

# Feature sets for different ARM devices
declare -A FEATURE_SETS=(
    ["tv-box"]="arm-tv-box"
    ["embedded"]="arm-embedded"
    ["iot"]="arm-iot"
    ["minimal"]="minimal"
)

# Print usage information
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -t, --target TARGET     ARM target (armv7-unknown-linux-gnueabihf, etc.)"
    echo "  -f, --features FEATURES Feature set (tv-box, embedded, iot, minimal)"
    echo "  -r, --release          Build in release mode"
    echo "  -c, --clean            Clean before building"
    echo "  -p, --package          Create distribution packages"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --target armv7-unknown-linux-gnueabihf --features tv-box --release"
    echo "  $0 --clean --package"
}

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

# Check if required tools are installed
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v rustup &> /dev/null; then
        missing_deps+=("rustup")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_error "Please install the missing dependencies and try again."
        exit 1
    fi
    
    log_success "All dependencies are installed"
}

# Install ARM targets
install_targets() {
    log_info "Installing ARM targets..."
    
    for target in "${ARM_TARGETS[@]}"; do
        log_info "Installing target: $target"
        rustup target add "$target" || {
            log_warning "Failed to install target: $target"
        }
    done
    
    log_success "ARM targets installation completed"
}

# Clean build artifacts
clean_build() {
    log_info "Cleaning build artifacts..."
    cargo clean
    rm -rf "$DIST_DIR"
    log_success "Build artifacts cleaned"
}

# Build for specific target and features
build_target() {
    local target="$1"
    local features="$2"
    local release_flag="$3"
    
    log_info "Building for target: $target with features: $features"
    
    local cargo_cmd="cargo build --target $target --no-default-features --features $features"
    
    if [ "$release_flag" = "true" ]; then
        cargo_cmd="$cargo_cmd --release"
        local build_type="release"
    else
        local build_type="debug"
    fi
    
    log_info "Running: $cargo_cmd"
    
    if $cargo_cmd; then
        log_success "Build completed for $target"
        
        # Copy binary to dist directory
        local binary_name="lavalink-rust"
        local source_path="$BUILD_DIR/$target/$build_type/$binary_name"
        local dest_path="$DIST_DIR/$target-$features-$build_type"
        
        mkdir -p "$dest_path"
        
        if [ -f "$source_path" ]; then
            cp "$source_path" "$dest_path/"
            log_success "Binary copied to: $dest_path/$binary_name"
        else
            log_warning "Binary not found at: $source_path"
        fi
    else
        log_error "Build failed for $target"
        return 1
    fi
}

# Create distribution packages
create_packages() {
    log_info "Creating distribution packages..."
    
    if [ ! -d "$DIST_DIR" ]; then
        log_warning "No distribution directory found. Run build first."
        return 1
    fi
    
    cd "$DIST_DIR"
    
    for dir in */; do
        if [ -d "$dir" ]; then
            local package_name="${dir%/}"
            log_info "Creating package for: $package_name"
            
            tar -czf "${package_name}.tar.gz" "$dir"
            log_success "Package created: ${package_name}.tar.gz"
        fi
    done
    
    cd "$PROJECT_ROOT"
    log_success "All packages created"
}

# Main function
main() {
    local target=""
    local features=""
    local release_flag="false"
    local clean_flag="false"
    local package_flag="false"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -t|--target)
                target="$2"
                shift 2
                ;;
            -f|--features)
                features="$2"
                shift 2
                ;;
            -r|--release)
                release_flag="true"
                shift
                ;;
            -c|--clean)
                clean_flag="true"
                shift
                ;;
            -p|--package)
                package_flag="true"
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Check dependencies
    check_dependencies
    
    # Install ARM targets
    install_targets
    
    # Clean if requested
    if [ "$clean_flag" = "true" ]; then
        clean_build
    fi
    
    # Build targets
    if [ -n "$target" ] && [ -n "$features" ]; then
        # Build specific target
        if [[ " ${ARM_TARGETS[*]} " =~ " $target " ]]; then
            if [[ -v FEATURE_SETS["$features"] ]]; then
                build_target "$target" "${FEATURE_SETS[$features]}" "$release_flag"
            else
                log_error "Unknown feature set: $features"
                log_error "Available feature sets: ${!FEATURE_SETS[*]}"
                exit 1
            fi
        else
            log_error "Unknown target: $target"
            log_error "Available targets: ${ARM_TARGETS[*]}"
            exit 1
        fi
    else
        # Build all targets with default features
        log_info "Building all ARM targets with default features..."
        
        for target in "${ARM_TARGETS[@]}"; do
            for feature_name in "${!FEATURE_SETS[@]}"; do
                build_target "$target" "${FEATURE_SETS[$feature_name]}" "$release_flag" || {
                    log_warning "Build failed for $target with $feature_name features"
                }
            done
        done
    fi
    
    # Create packages if requested
    if [ "$package_flag" = "true" ]; then
        create_packages
    fi
    
    log_success "ARM build script completed successfully!"
}

# Run main function with all arguments
main "$@"

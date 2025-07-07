#!/bin/bash

# Discord End-to-End Test Runner
# This script helps set up and run Discord E2E tests with proper environment configuration

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}Discord End-to-End Test Runner${NC}"
echo "=================================="

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if environment variable is set
check_env_var() {
    local var_name="$1"
    local var_value="${!var_name}"
    
    if [ -z "$var_value" ]; then
        print_error "Environment variable $var_name is not set"
        return 1
    else
        print_success "Environment variable $var_name is set"
        return 0
    fi
}

# Function to validate Discord bot token format
validate_bot_token() {
    local token="$1"
    
    # Basic validation - Discord bot tokens are typically 59+ characters
    if [ ${#token} -lt 50 ]; then
        print_warning "Bot token seems too short (${#token} characters). Please verify it's correct."
        return 1
    fi
    
    # Check if it looks like a Discord bot token (contains dots)
    if [[ ! "$token" =~ \. ]]; then
        print_warning "Bot token doesn't appear to be in Discord format. Please verify it's correct."
        return 1
    fi
    
    return 0
}

# Function to validate Discord ID format
validate_discord_id() {
    local id="$1"
    local id_type="$2"
    
    # Discord IDs are 17-19 digit numbers
    if [[ ! "$id" =~ ^[0-9]{17,19}$ ]]; then
        print_error "$id_type ID '$id' is not in valid Discord ID format (17-19 digits)"
        return 1
    fi
    
    return 0
}

# Check if we're in the right directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    print_error "This script must be run from the Lavalink-rust project directory"
    exit 1
fi

print_info "Project root: $PROJECT_ROOT"

# Check for .env file
ENV_FILE="$PROJECT_ROOT/.env"
if [ -f "$ENV_FILE" ]; then
    print_info "Found .env file, sourcing it..."
    source "$ENV_FILE"
else
    print_warning "No .env file found. Make sure environment variables are set manually."
fi

# Check required environment variables
print_info "Checking required environment variables..."

MISSING_VARS=0

if ! check_env_var "DISCORD_BOT_TOKEN"; then
    MISSING_VARS=$((MISSING_VARS + 1))
fi

if ! check_env_var "DISCORD_GUILD_ID"; then
    MISSING_VARS=$((MISSING_VARS + 1))
fi

if ! check_env_var "DISCORD_VOICE_CHANNEL_ID"; then
    MISSING_VARS=$((MISSING_VARS + 1))
fi

# Validate environment variables if they're set
if [ -n "$DISCORD_BOT_TOKEN" ]; then
    if validate_bot_token "$DISCORD_BOT_TOKEN"; then
        print_success "Bot token format appears valid"
    fi
fi

if [ -n "$DISCORD_GUILD_ID" ]; then
    if validate_discord_id "$DISCORD_GUILD_ID" "Guild"; then
        print_success "Guild ID format is valid"
    else
        MISSING_VARS=$((MISSING_VARS + 1))
    fi
fi

if [ -n "$DISCORD_VOICE_CHANNEL_ID" ]; then
    if validate_discord_id "$DISCORD_VOICE_CHANNEL_ID" "Voice Channel"; then
        print_success "Voice Channel ID format is valid"
    else
        MISSING_VARS=$((MISSING_VARS + 1))
    fi
fi

# Exit if any required variables are missing or invalid
if [ $MISSING_VARS -gt 0 ]; then
    print_error "Missing or invalid environment variables. Please set them and try again."
    echo ""
    echo "Required environment variables:"
    echo "  DISCORD_BOT_TOKEN=your_bot_token_here"
    echo "  DISCORD_GUILD_ID=your_guild_id_here"
    echo "  DISCORD_VOICE_CHANNEL_ID=your_voice_channel_id_here"
    echo ""
    echo "You can create a .env file in the project root with these variables."
    echo "See docs/testing/discord-e2e-tests.md for detailed setup instructions."
    exit 1
fi

# Set default logging level if not set
if [ -z "$RUST_LOG" ]; then
    export RUST_LOG="info"
    print_info "Set RUST_LOG to 'info' (default)"
fi

# Parse command line arguments
TEST_FILTER=""
VERBOSE=""
NOCAPTURE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --test)
            TEST_FILTER="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE="--verbose"
            shift
            ;;
        --nocapture)
            NOCAPTURE="--nocapture"
            shift
            ;;
        --debug)
            export RUST_LOG="debug"
            NOCAPTURE="--nocapture"
            print_info "Enabled debug logging"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --test TEST_NAME    Run specific test (e.g., test_discord_bot_initialization)"
            echo "  --verbose, -v       Enable verbose output"
            echo "  --nocapture         Show test output (useful for debugging)"
            echo "  --debug             Enable debug logging and nocapture"
            echo "  --help, -h          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Run all E2E tests"
            echo "  $0 --test test_voice_channel_connection  # Run specific test"
            echo "  $0 --debug                           # Run with debug logging"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build the cargo test command
CARGO_CMD="cargo test --test discord_e2e_tests"

if [ -n "$TEST_FILTER" ]; then
    CARGO_CMD="$CARGO_CMD $TEST_FILTER"
fi

CARGO_CMD="$CARGO_CMD -- --ignored"

if [ -n "$VERBOSE" ]; then
    CARGO_CMD="$CARGO_CMD $VERBOSE"
fi

if [ -n "$NOCAPTURE" ]; then
    CARGO_CMD="$CARGO_CMD $NOCAPTURE"
fi

# Show configuration summary
echo ""
print_info "Test Configuration Summary:"
echo "  Bot Token: ${DISCORD_BOT_TOKEN:0:20}... (${#DISCORD_BOT_TOKEN} chars)"
echo "  Guild ID: $DISCORD_GUILD_ID"
echo "  Voice Channel ID: $DISCORD_VOICE_CHANNEL_ID"
echo "  Rust Log Level: $RUST_LOG"
if [ -n "$TEST_FILTER" ]; then
    echo "  Test Filter: $TEST_FILTER"
fi
echo ""

# Confirm before running
read -p "Run Discord E2E tests with this configuration? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Test run cancelled by user"
    exit 0
fi

# Change to project directory
cd "$PROJECT_ROOT"

# Run the tests
print_info "Running Discord E2E tests..."
print_info "Command: $CARGO_CMD"
echo ""

if eval "$CARGO_CMD"; then
    echo ""
    print_success "All Discord E2E tests completed successfully!"
else
    echo ""
    print_error "Some Discord E2E tests failed. Check the output above for details."
    exit 1
fi

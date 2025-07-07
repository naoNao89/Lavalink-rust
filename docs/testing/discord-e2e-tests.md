# Discord End-to-End Testing Guide

This guide explains how to run end-to-end tests with a real Discord bot to validate the complete voice integration functionality.

## Overview

The Discord E2E tests (`tests/discord_e2e_tests.rs`) provide comprehensive validation of:

- Discord bot initialization and authentication
- Voice channel connection and disconnection
- Audio quality management in real Discord environment
- Streaming manager integration
- Connection recovery scenarios
- Performance under load
- Full integration workflow

## Prerequisites

### 1. Discord Bot Setup

You need a Discord bot with the following permissions:

- **Connect** - Connect to voice channels
- **Speak** - Transmit audio in voice channels
- **Use Voice Activity** - Use voice activity detection

#### Creating a Discord Bot

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" section
4. Click "Add Bot"
5. Copy the bot token (keep it secure!)
6. Under "Privileged Gateway Intents", enable:
   - Server Members Intent (if needed)
   - Message Content Intent (if needed)

#### Inviting the Bot to Your Server

1. Go to the "OAuth2" > "URL Generator" section
2. Select scopes: `bot`
3. Select bot permissions: `Connect`, `Speak`, `Use Voice Activity`
4. Copy the generated URL and open it in your browser
5. Select your test server and authorize the bot

### 2. Test Server Setup

Create a test Discord server with:

- At least one voice channel
- The bot invited with proper permissions
- Note the Guild ID and Voice Channel ID

#### Getting IDs

Enable Developer Mode in Discord (User Settings > Advanced > Developer Mode), then:

- Right-click your server → "Copy Server ID" (Guild ID)
- Right-click a voice channel → "Copy Channel ID" (Voice Channel ID)

## Environment Variables

Set the following environment variables before running tests:

```bash
# Required: Discord bot token
export DISCORD_BOT_TOKEN="your_bot_token_here"

# Required: Test guild (server) ID
export DISCORD_GUILD_ID="123456789012345678"

# Required: Test voice channel ID
export DISCORD_VOICE_CHANNEL_ID="987654321098765432"

# Optional: Rust logging level
export RUST_LOG=info
```

### Using a .env File

Create a `.env` file in the project root:

```env
DISCORD_BOT_TOKEN=your_bot_token_here
DISCORD_GUILD_ID=123456789012345678
DISCORD_VOICE_CHANNEL_ID=987654321098765432
RUST_LOG=info
```

Then source it before running tests:
```bash
source .env
```

## Running the Tests

### All E2E Tests

```bash
# Run all Discord E2E tests
cargo test --test discord_e2e_tests -- --ignored

# Run with verbose output
cargo test --test discord_e2e_tests -- --ignored --nocapture

# Run with debug logging
RUST_LOG=debug cargo test --test discord_e2e_tests -- --ignored --nocapture
```

### Individual Tests

```bash
# Test Discord bot initialization
cargo test --test discord_e2e_tests test_discord_bot_initialization -- --ignored

# Test voice channel connection
cargo test --test discord_e2e_tests test_voice_channel_connection -- --ignored

# Test audio quality integration
cargo test --test discord_e2e_tests test_audio_quality_integration -- --ignored

# Test full integration workflow
cargo test --test discord_e2e_tests test_full_integration_workflow -- --ignored
```

### CI/CD Integration

For automated testing in CI/CD pipelines, store credentials as secrets:

```yaml
# GitHub Actions example
env:
  DISCORD_BOT_TOKEN: ${{ secrets.DISCORD_BOT_TOKEN }}
  DISCORD_GUILD_ID: ${{ secrets.DISCORD_GUILD_ID }}
  DISCORD_VOICE_CHANNEL_ID: ${{ secrets.DISCORD_VOICE_CHANNEL_ID }}

steps:
  - name: Run Discord E2E Tests
    run: cargo test --test discord_e2e_tests -- --ignored
```

## Test Descriptions

### Core Connection Tests

- **`test_discord_bot_initialization`** - Validates Discord bot setup and authentication
- **`test_voice_channel_connection`** - Tests joining voice channels
- **`test_voice_channel_disconnection`** - Tests leaving voice channels

### Quality Management Tests

- **`test_audio_quality_integration`** - Tests quality metrics and preset switching
- **`test_quality_degradation_detection`** - Tests quality trend analysis
- **`test_streaming_manager_integration`** - Tests streaming manager functionality

### Reliability Tests

- **`test_connection_recovery_scenarios`** - Tests connection retry logic
- **`test_voice_event_logging`** - Tests event collection and logging
- **`test_concurrent_operations`** - Tests thread safety under concurrent load

### Performance Tests

- **`test_performance_under_load`** - Tests performance with rapid operations
- **`test_full_integration_workflow`** - Tests complete end-to-end workflow

## Expected Behavior

### Successful Test Run

When all environment variables are set correctly and the bot has proper permissions:

```
test test_discord_bot_initialization ... ok
test test_voice_channel_connection ... ok
test test_voice_channel_disconnection ... ok
test test_audio_quality_integration ... ok
test test_streaming_manager_integration ... ok
test test_connection_recovery_scenarios ... ok
test test_voice_event_logging ... ok
test test_quality_degradation_detection ... ok
test test_concurrent_operations ... ok
test test_performance_under_load ... ok
test test_full_integration_workflow ... ok
```

### Missing Credentials

If environment variables are not set:

```
Skipping Discord E2E test: missing environment variables
Required: DISCORD_BOT_TOKEN, DISCORD_GUILD_ID, DISCORD_VOICE_CHANNEL_ID
```

### Connection Issues

Some tests may show warnings about connection failures in test environments:

```
Voice channel connection failed (expected in test environment): ...
```

This is normal - the tests validate that connection attempts are made properly, even if they fail due to test environment limitations.

## Troubleshooting

### Bot Token Issues

- Ensure the token is valid and hasn't been regenerated
- Check that the bot is properly invited to your test server
- Verify the bot has the required permissions

### Permission Issues

- Ensure the bot has `Connect` and `Speak` permissions
- Check that the voice channel allows the bot to join
- Verify the bot role has sufficient permissions

### Network Issues

- Some tests may timeout in environments with restricted network access
- Consider increasing timeout values for slower networks
- Check firewall settings if running in restricted environments

### Test Environment Limitations

- Tests are designed to work even when actual Discord connections fail
- Focus on the test logic and error handling rather than successful connections
- Many tests validate the attempt to connect rather than successful connection

## Security Considerations

- **Never commit Discord bot tokens to version control**
- Use environment variables or secure secret management
- Rotate bot tokens regularly
- Limit bot permissions to minimum required
- Use separate bots for testing and production

## Integration with CI/CD

The tests are marked with `#[ignore]` to prevent accidental runs without credentials. In CI/CD:

1. Store credentials as encrypted secrets
2. Set environment variables in the CI environment
3. Run tests with the `--ignored` flag
4. Consider running E2E tests on a schedule rather than every commit
5. Use separate test Discord servers for different environments

## Contributing

When adding new E2E tests:

1. Use the `discord_e2e_test!` macro for consistency
2. Include proper error handling for test environment limitations
3. Add comprehensive logging for debugging
4. Test both success and failure scenarios
5. Update this documentation with new test descriptions

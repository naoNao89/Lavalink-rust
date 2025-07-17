# Statistics Implementation in Lavalink-rust

## Overview

Lavalink-rust now provides comprehensive statistics functionality that matches the Java Lavalink v4 implementation, including real system monitoring and periodic WebSocket broadcasting.

## Features

### Real System Monitoring
- **Memory Statistics**: Real memory usage (free, used, allocated, reservable) using the `sysinfo` crate
- **CPU Statistics**: Real CPU usage (system load and Lavalink-specific load) with per-core monitoring
- **Player Statistics**: Accurate player counts (total players and currently playing players)
- **Uptime Tracking**: Server uptime in milliseconds
- **Frame Statistics**: Audio frame processing statistics

### Periodic Broadcasting
- **WebSocket Events**: Stats are automatically broadcasted to all connected WebSocket clients every 60 seconds
- **REST API**: Stats are available via the `/v4/stats` endpoint
- **Real-time Updates**: Stats reflect current system state and player activity

## Configuration

### Feature Flags
- `system-stats`: Enables real system monitoring (enabled by default)
- When disabled, falls back to placeholder values for compatibility

### Dependencies
- `sysinfo = "0.36"`: Cross-platform system monitoring
- Minimal overhead with efficient system polling

## API Compatibility

### REST Endpoint
```
GET /v4/stats
```

Returns JSON with the following structure:
```json
{
  "players": 1,
  "playingPlayers": 0,
  "uptime": 123456,
  "memory": {
    "free": 285769728,
    "used": 6213451776,
    "allocated": 8589934592,
    "reservable": 8589934592
  },
  "cpu": {
    "cores": 8,
    "systemLoad": 0.186,
    "lavalinkLoad": 0.001
  },
  "frameStats": {
    "sent": 1000,
    "nulled": 5,
    "deficit": 2
  }
}
```

### WebSocket Events
Stats events are automatically sent to all connected clients every 60 seconds:
```json
{
  "op": "stats",
  "players": 1,
  "playingPlayers": 0,
  "uptime": 123456,
  "memory": { ... },
  "cpu": { ... },
  "frameStats": { ... }
}
```

## Implementation Details

### StatsCollector
- Thread-safe system monitoring with `Mutex<System>`
- Efficient CPU polling (minimum 1-second intervals for accurate readings)
- Graceful fallback to placeholder values if system monitoring fails
- Integration with PlayerManager for real player counts

### Background Task
- Spawned during server startup
- 60-second interval timer with missed tick behavior handling
- Automatic cleanup of disconnected WebSocket sessions
- Error handling for failed message delivery

### Performance
- Minimal system overhead
- Efficient memory and CPU polling
- Background task runs independently of request handling
- Cross-platform compatibility (Linux, macOS, Windows)

## Testing

Comprehensive test suite includes:
- Unit tests for stats collection
- Integration tests with player manager
- JSON serialization compatibility tests
- Real system stats validation
- Cross-platform compatibility tests

Run tests with:
```bash
cargo test stats --features system-stats
```

## Migration from Java Lavalink

This implementation provides full compatibility with Java Lavalink v4 stats:
- Same JSON structure and field names
- Same WebSocket event format
- Same REST endpoint behavior
- Same 60-second broadcast interval

Discord bots and other clients should work without modification.

## Troubleshooting

### Missing Stats in Discord Bots
- Ensure WebSocket connection is established
- Check that the `system-stats` feature is enabled
- Verify that stats broadcasting task is running
- Check server logs for WebSocket errors

### Inaccurate System Stats
- Ensure sufficient permissions for system monitoring
- Check that `sysinfo` crate supports your platform
- Verify that system monitoring is not being blocked by security software

### Performance Issues
- Stats collection has minimal overhead
- Background task runs only once per minute
- System polling is optimized for efficiency
- Consider disabling `system-stats` feature if not needed

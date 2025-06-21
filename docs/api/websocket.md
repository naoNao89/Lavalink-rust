---
description: Lavalink Rust WebSocket API documentation.
---

# WebSocket API

Lavalink Rust provides a WebSocket API for real-time communication with clients. The WebSocket protocol is fully compatible with the Lavalink v4 specification while offering enhanced performance and reliability.

!!! rust "Rust Implementation Benefits"
    - **Lower Latency**: Native async implementation provides faster WebSocket message processing
    - **Better Concurrency**: Tokio-based async runtime handles thousands of concurrent connections efficiently
    - **Memory Safety**: Rust's ownership system prevents memory leaks and connection issues
    - **Predictable Performance**: No garbage collection pauses affecting WebSocket responsiveness

## Opening a Connection

You can establish a WebSocket connection against the path `/v4/websocket`.

When opening a WebSocket connection, you must supply these required headers:

| Header Name     | Description                                     |
|-----------------|-------------------------------------------------|
| `Authorization` | The password you set in your Lavalink config    |
| `User-Id`       | The user id of the bot                          |
| `Client-Name`   | The name of the client in `NAME/VERSION` format |
| `Session-Id`?   | The id of the previous session to resume        |

!!! info "Session Resuming"
    For more information on resuming sessions, see the [Session Management](../advanced/session-management.md) guide.

<details markdown="1">
<summary>Example Headers</summary>

```
Authorization: youshallnotpass
User-Id: 170939974227541168
Client-Name: lavalink-client/2.0.0
Session-Id: previous-session-id-here
```

</details>

## Message Format

WebSocket messages follow this standard format:

| Field | Type                 | Description                           |
|-------|----------------------|---------------------------------------|
| op    | [OP Type](#op-types) | The operation type                    |
| ...   | ...                  | Extra fields depending on the op type |

<details markdown="1">
<summary>Example Message</summary>

```json
{
  "op": "ready",
  "resumed": false,
  "sessionId": "session-id-here"
}
```

</details>

## OP Types

| OP Type                           | Description                                                   |
|-----------------------------------|---------------------------------------------------------------|
| [ready](#ready-op)                | Dispatched when you successfully connect to the Lavalink node |
| [playerUpdate](#player-update-op) | Dispatched every x seconds with the latest player state       |
| [stats](#stats-op)                | Dispatched when the node sends stats once per minute          |
| [event](#event-op)                | Dispatched when player or voice events occur                  |

### Ready OP

Dispatched by Lavalink Rust upon successful connection and authorization. Contains fields determining if resuming was successful, as well as the session id.

| Field     | Type   | Description                                                                                    |
|-----------|--------|------------------------------------------------------------------------------------------------|
| resumed   | bool   | Whether this session was resumed                                                               |
| sessionId | string | The Lavalink session id of this connection. Not to be confused with a Discord voice session id |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "ready",
  "resumed": false,
  "sessionId": "rust-session-abc123"
}
```

</details>

---

### Player Update OP

Dispatched every x seconds (configurable in `application.yml`) with the current state of the player.

!!! performance "Rust Performance"
    Player updates in Rust Lavalink are more accurate and have lower overhead due to efficient memory management and precise timing.

| Field   | Type                                 | Description                |
|---------|--------------------------------------|----------------------------|
| guildId | string                               | The guild id of the player |
| state   | [Player State](#player-state) object | The player state           |

#### Player State

| Field     | Type | Description                                                                              |
|-----------|------|------------------------------------------------------------------------------------------|
| time      | int  | Unix timestamp in milliseconds                                                           |
| position  | int  | The position of the track in milliseconds                                                |
| connected | bool | Whether Lavalink is connected to the voice gateway                                       |
| ping      | int  | The ping of the node to the Discord voice server in milliseconds (`-1` if not connected) |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "playerUpdate",
  "guildId": "817327181659111454",
  "state": {
    "time": 1500467109,
    "position": 60000,
    "connected": true,
    "ping": 42
  }
}
```

</details>

---

### Stats OP

A collection of statistics sent every minute. Rust Lavalink provides more detailed and accurate statistics.

!!! rust "Enhanced Statistics"
    Rust Lavalink provides more accurate memory statistics and better performance monitoring due to native system integration.

#### Stats Object

| Field          | Type                                | Description                                                                                      |
|----------------|-------------------------------------|--------------------------------------------------------------------------------------------------|
| players        | int                                 | The amount of players connected to the node                                                      |
| playingPlayers | int                                 | The amount of players playing a track                                                            |
| uptime         | int                                 | The uptime of the node in milliseconds                                                           |
| memory         | [Memory](#memory) object            | The memory stats of the node                                                                     |
| cpu            | [CPU](#cpu) object                  | The cpu stats of the node                                                                        |
| frameStats     | ?[Frame Stats](#frame-stats) object | The frame stats of the node. `null` if the node has no players or when retrieved via `/v4/stats` |

##### Memory

| Field      | Type | Description                              |
|------------|------|------------------------------------------|
| free       | int  | The amount of free memory in bytes       |
| used       | int  | The amount of used memory in bytes       |
| allocated  | int  | The amount of allocated memory in bytes  |
| reservable | int  | The amount of reservable memory in bytes |

!!! rust "Memory Tracking"
    Rust Lavalink provides more accurate memory statistics as it doesn't have garbage collection overhead and tracks memory usage more precisely.

##### CPU

| Field        | Type  | Description                      |
|--------------|-------|----------------------------------|
| cores        | int   | The amount of cores the node has |
| systemLoad   | float | The system load of the node      |
| lavalinkLoad | float | The load of Lavalink on the node |

##### Frame Stats

| Field     | Type | Description                                                          |
|-----------|------|----------------------------------------------------------------------|
| sent      | int  | The amount of frames sent to Discord                                 |
| nulled    | int  | The amount of frames that were nulled                                |
| deficit   | int  | The difference between sent frames and the expected amount of frames |

!!! info "Frame Calculation"
    The expected amount of frames is 3000 (1 every 20 ms) per player. If the `deficit` is negative, too many frames were sent, and if it's positive, not enough frames got sent.

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "stats",
  "players": 5,
  "playingPlayers": 3,
  "uptime": 123456789,
  "memory": {
    "free": 512000000,
    "used": 104857600,
    "allocated": 134217728,
    "reservable": 1073741824
  },
  "cpu": {
    "cores": 8,
    "systemLoad": 0.25,
    "lavalinkLoad": 0.15
  },
  "frameStats": {
    "sent": 9000,
    "nulled": 5,
    "deficit": -15
  }
}
```

</details>

---

### Event OP

Server dispatched an event. See the [Event Types](#event-types) section for more information.

| Field   | Type                      | Description                         |
|---------|---------------------------|-------------------------------------|
| type    | [EventType](#event-types) | The type of event                   |
| guildId | string                    | The guild id                        |
| ...     | ...                       | Extra fields depending on the event |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "TrackStartEvent",
  "guildId": "817327181659111454",
  "track": {
    "encoded": "...",
    "info": { ... }
  }
}
```

</details>

#### Event Types

| Event Type                                    | Description                                                                 |
|-----------------------------------------------|-----------------------------------------------------------------------------|
| [TrackStartEvent](#trackstartevent)           | Dispatched when a track starts playing                                      |
| [TrackEndEvent](#trackendevent)               | Dispatched when a track ends                                                |
| [TrackExceptionEvent](#trackexceptionevent)   | Dispatched when a track throws an exception                                 |
| [TrackStuckEvent](#trackstuckevent)           | Dispatched when a track gets stuck while playing                            |
| [WebSocketClosedEvent](#websocketclosedevent) | Dispatched when the websocket connection to Discord voice servers is closed |

##### TrackStartEvent

Dispatched when a track starts playing.

| Field | Type                          | Description                    |
|-------|-------------------------------|--------------------------------|
| track | [Track](rest.md#track) object | The track that started playing |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "TrackStartEvent",
  "guildId": "817327181659111454",
  "track": {
    "encoded": "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==",
    "info": {
      "identifier": "dQw4w9WgXcQ",
      "isSeekable": true,
      "author": "RickAstleyVEVO",
      "length": 212000,
      "isStream": false,
      "position": 0,
      "title": "Rick Astley - Never Gonna Give You Up",
      "uri": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      "artworkUrl": "https://i.ytimg.com/vi/dQw4w9WgXcQ/maxresdefault.jpg",
      "isrc": null,
      "sourceName": "youtube"
    },
    "pluginInfo": {},
    "userData": {}
  }
}
```

</details>

##### TrackEndEvent

Dispatched when a track ends.

| Field  | Type                                | Description                  |
|--------|-------------------------------------|------------------------------|
| track  | [Track](rest.md#track) object       | The track that ended playing |
| reason | [TrackEndReason](#track-end-reason) | The reason the track ended   |

###### Track End Reason

| Reason       | Description                | May Start Next |
|--------------|----------------------------|----------------|
| `finished`   | The track finished playing | true           |
| `loadFailed` | The track failed to load   | true           |
| `stopped`    | The track was stopped      | false          |
| `replaced`   | The track was replaced     | false          |
| `cleanup`    | The track was cleaned up   | false          |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "TrackEndEvent",
  "guildId": "817327181659111454",
  "track": {
    "encoded": "...",
    "info": { ... }
  },
  "reason": "finished"
}
```

</details>

##### TrackExceptionEvent

Dispatched when a track throws an exception.

!!! rust "Enhanced Error Handling"
    Rust Lavalink provides more detailed error information and better stack traces due to Rust's superior error handling capabilities.

| Field     | Type                                  | Description                        |
|-----------|---------------------------------------|------------------------------------|
| track     | [Track](rest.md#track) object         | The track that threw the exception |
| exception | [Exception](#exception-object) object | The occurred exception             |

###### Exception Object

| Field           | Type                  | Description                       |
|-----------------|-----------------------|-----------------------------------|
| message         | ?string               | The message of the exception      |
| severity        | [Severity](#severity) | The severity of the exception     |
| cause           | string                | The cause of the exception        |
| causeStackTrace | string                | The full stack trace of the cause |

###### Severity

| Severity     | Description                                                                                                                                                                                                                    |
|--------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `common`     | The cause is known and expected, indicates that there is nothing wrong with the library itself                                                                                                                                 |
| `suspicious` | The cause might not be exactly known, but is possibly caused by outside factors. For example when an outside service responds in a format that we do not expect                                                                |
| `fault`      | The probable cause is an issue with the library or there is no way to tell what the cause might be. This is the default level and other levels are used in cases where the thrower has more in-depth knowledge about the error |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "TrackExceptionEvent",
  "guildId": "817327181659111454",
  "track": {
    "encoded": "...",
    "info": { ... }
  },
  "exception": {
    "message": "Failed to decode audio stream",
    "severity": "common",
    "cause": "Invalid audio format",
    "causeStackTrace": "rust_lavalink::audio::decoder::decode_error\n    at src/audio/decoder.rs:42:5\n..."
  }
}
```

</details>

##### TrackStuckEvent

Dispatched when a track gets stuck while playing.

| Field       | Type                          | Description                                     |
|-------------|-------------------------------|-------------------------------------------------|
| track       | [Track](rest.md#track) object | The track that got stuck                        |
| thresholdMs | int                           | The threshold in milliseconds that was exceeded |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "TrackStuckEvent",
  "guildId": "817327181659111454",
  "track": {
    "encoded": "...",
    "info": { ... }
  },
  "thresholdMs": 10000
}
```

</details>

##### WebSocketClosedEvent

Dispatched when an audio WebSocket (to Discord) is closed.
This can happen for various reasons (normal and abnormal), e.g. when using an expired voice server update.
4xxx codes are usually bad.
See the [Discord Docs](https://discord.com/developers/docs/topics/opcodes-and-status-codes#voice-voice-close-event-codes).

!!! rust "Connection Reliability"
    Rust Lavalink provides better connection handling and more detailed close event information due to robust async networking.

| Field    | Type   | Description                                                                                                                       |
|----------|--------|-----------------------------------------------------------------------------------------------------------------------------------|
| code     | int    | The [Discord close event code](https://discord.com/developers/docs/topics/opcodes-and-status-codes#voice-voice-close-event-codes) |
| reason   | string | The close reason                                                                                                                  |
| byRemote | bool   | Whether the connection was closed by Discord                                                                                      |

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "op": "event",
  "type": "WebSocketClosedEvent",
  "guildId": "817327181659111454",
  "code": 4006,
  "reason": "Your session is no longer valid.",
  "byRemote": true
}
```

</details>

---

## Client Implementation Notes

### Connection Management

!!! rust "Rust-Specific Benefits"
    - **Automatic Reconnection**: Built-in connection resilience with exponential backoff
    - **Memory Efficiency**: Lower memory usage per connection compared to Java implementation
    - **Concurrent Handling**: Better support for multiple simultaneous connections

### Performance Considerations

1. **Message Processing**: Rust Lavalink processes WebSocket messages faster due to zero-copy deserialization where possible
2. **Memory Usage**: Each WebSocket connection uses significantly less memory than the Java implementation
3. **Latency**: Lower message latency due to native async implementation

### Error Handling

Rust Lavalink provides enhanced error handling:

- More detailed error messages
- Better stack traces with Rust-specific information
- Structured error responses for easier client debugging
- Graceful degradation on connection issues

### Migration from Java Lavalink

The WebSocket API is 100% compatible with existing clients:

- All message formats remain identical
- Event types and structures are unchanged
- Connection headers and authentication work the same way
- Session resuming works identically

The only differences are performance improvements and enhanced error information.

For more information about migrating from Java Lavalink, see the [Migration Guide](../migration/from-java.md).

---

## Testing WebSocket Connections

For testing WebSocket connections, you can use:

1. **Browser Developer Tools**: Connect directly from browser console
2. **WebSocket Testing Tools**: Use tools like Postman or Insomnia
3. **Client Libraries**: Use existing Lavalink client libraries

Example JavaScript connection:

```javascript
const ws = new WebSocket('ws://localhost:2333/v4/websocket', {
  headers: {
    'Authorization': 'youshallnotpass',
    'User-Id': '170939974227541168',
    'Client-Name': 'test-client/1.0.0'
  }
});

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

For more examples and testing guidance, see the [API Testing Guide](../getting-started/testing.md).

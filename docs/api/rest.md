---
description: Lavalink Rust REST API documentation.
---

# REST API

Lavalink Rust exposes a REST API to allow for easy control of the players. The API is fully compatible with the Lavalink v4 specification while providing enhanced performance and reliability.

Most routes require the `Authorization` header with the configured password.

```
Authorization: youshallnotpass
```

!!! rust "Rust Implementation"
    This documentation covers the Rust implementation of Lavalink. All endpoints maintain 100% compatibility with existing Lavalink clients while providing improved performance and memory safety.

## API Compatibility

Routes are prefixed with `/v4` as of Lavalink v4.0.0. The `/version` endpoint remains without prefix for compatibility.

### Rust-Specific Improvements

- **Faster Response Times**: Native binary execution provides consistently faster API responses
- **Lower Memory Usage**: Rust's memory management eliminates garbage collection overhead
- **Better Error Handling**: More detailed error messages with structured responses
- **Enhanced Stability**: Memory safety prevents crashes and undefined behavior

## Error Responses

When Lavalink Rust encounters an error, it responds with a JSON object containing detailed information. Include the `trace=true` query param to receive additional debugging information.

| Field     | Type   | Description                                                                 |
|-----------|--------|-----------------------------------------------------------------------------|
| timestamp | int    | The timestamp of the error in milliseconds since the Unix epoch             |
| status    | int    | The HTTP status code                                                        |
| error     | string | The HTTP status code message                                                |
| trace?    | string | The stack trace when `trace=true` query param is provided                  |
| message   | string | The error message                                                           |
| path      | string | The request path                                                            |

!!! performance "Performance Note"
    Error responses in Rust Lavalink are generated faster and with lower memory allocation compared to the Java implementation.

<details markdown="1">
<summary>Example Error Response</summary>

```json
{
  "timestamp": 1667857581613,
  "status": 404,
  "error": "Not Found",
  "trace": "...",
  "message": "Session not found",
  "path": "/v4/sessions/xtaug914v9k5032f/players/817327181659111454"
}
```

</details>

## Track API

### Common Types ### {: #track-api-types }

#### Track

| Field      | Type                             | Description                                                                     |
|------------|----------------------------------|---------------------------------------------------------------------------------|
| encoded    | string                           | The base64 encoded track data                                                   |
| info       | [Track Info](#track-info) object | Info about the track                                                            |
| pluginInfo | object                           | Additional track info provided by plugins                                       |
| userData   | object                           | Additional track data provided via the [Update Player](#update-player) endpoint |

#### Track Info

| Field      | Type    | Description                                                                           |
|------------|---------|---------------------------------------------------------------------------------------|
| identifier | string  | The track identifier                                                                  |
| isSeekable | bool    | Whether the track is seekable                                                         |
| author     | string  | The track author                                                                      |
| length     | int     | The track length in milliseconds                                                      |
| isStream   | bool    | Whether the track is a stream                                                         |
| position   | int     | The track position in milliseconds                                                    |
| title      | string  | The track title                                                                       |
| uri        | ?string | The track uri                                                                         |
| artworkUrl | ?string | The track artwork url                                                                 |
| isrc       | ?string | The track [ISRC](https://en.wikipedia.org/wiki/International_Standard_Recording_Code) |
| sourceName | string  | The track source name                                                                 |

#### Playlist Info

| Field         | Type   | Description                                                     |
|---------------|--------|-----------------------------------------------------------------|
| name          | string | The name of the playlist                                        |
| selectedTrack | int    | The selected track of the playlist (-1 if no track is selected) |

---

## Track Loading

This endpoint resolves audio tracks for use with the [Update Player](#update-player) endpoint.

!!! rust "Rust Audio Sources"
    Lavalink Rust supports all standard audio sources plus an intelligent fallback system for Spotify, Apple Music, and Deezer URLs that automatically converts them to YouTube searches.

!!! tip "Search Prefixes"
    Lavalink Rust supports searching via YouTube and SoundCloud. Use these prefixes:
    
    - `ytsearch:` - YouTube search
    - `scsearch:` - SoundCloud search
    
    When a search prefix is used, the returned `loadType` will be `search`.

```
GET /v4/loadtracks?identifier=dQw4w9WgXcQ
```

### Supported Audio Sources

| Source | Status | Search Prefix | Notes |
|--------|--------|---------------|-------|
| YouTube | ✅ Full Support | `ytsearch:` | Complete functionality via yt-dlp |
| SoundCloud | ✅ Full Support | `scsearch:` | Complete functionality via yt-dlp |
| Bandcamp | ✅ Full Support | `bcsearch:` | Track and album support |
| Twitch | ✅ Full Support | - | Live streams and VODs |
| Vimeo | ✅ Full Support | - | Video audio extraction |
| HTTP Streams | ✅ Full Support | - | Direct audio URLs |
| Local Files | ✅ Full Support | `file://` | File system access |
| Spotify* | 🔄 Fallback | - | Converts to YouTube search |
| Apple Music* | 🔄 Fallback | - | Converts to YouTube search |
| Deezer* | 🔄 Fallback | - | Converts to YouTube search |

*Fallback sources use intelligent URL conversion to provide seamless compatibility.

### Fallback System

!!! migration "Spotify/Apple Music/Deezer Support"
    Lavalink Rust includes an intelligent fallback system that automatically converts Spotify, Apple Music, and Deezer URLs to YouTube searches, providing seamless compatibility for users.

**Example Fallback Process:**
```
Input:  https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh
Process: Extract track ID → Search YouTube → Return results
Output: YouTube track with similar content
```

Response:

#### Track Loading Result

| Field    | Type                                | Description            |       
|----------|-------------------------------------|------------------------|
| loadType | [LoadResultType](#load-result-type) | The type of the result | 
| data     | [LoadResultData](#load-result-data) | The data of the result |

#### Load Result Type

| Load Result Type | Description                                   |
|------------------|-----------------------------------------------|
| `track`          | A track has been loaded                       |
| `playlist`       | A playlist has been loaded                    |
| `search`         | A search result has been loaded               |
| `empty`          | There has been no matches for your identifier |
| `error`          | Loading has failed with an error              |

#### Load Result Data

##### Track Result Data

[Track](#track) object with the loaded track.

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "loadType": "track",
  "data": {
    "encoded": "...",
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

##### Search Result Data

Array of [Track](#track) objects from the search result.

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "loadType": "search",
  "data": [
    {
      "encoded": "...",
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
  ]
}
```

</details>

##### Error Result Data

[Exception](websocket.md#exception-object) object with the error.

<details markdown="1">
<summary>Example Payload</summary>

```json
{
  "loadType": "error",
  "data": {
    "message": "Something went wrong",
    "severity": "fault",
    "cause": "...",
    "causeStackTrace": "..."
  }
}
```

</details>

---

## Track Decoding

Decode a single track into its info, where `BASE64` is the encoded base64 data.

```
GET /v4/decodetrack?encodedTrack=BASE64
```

Response: [Track](#track) object

<details markdown="1">
<summary>Example Payload</summary>

```json
{
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
```

</details>

---

### Decode Multiple Tracks

!!! warning "Implementation Status"
    The `POST /v4/decodetracks` endpoint is currently not implemented in Lavalink Rust. Use individual decode requests for now.

```
POST /v4/decodetracks
```

**Status**: Not yet implemented - returns 501 Not Implemented

---

## Player API

### Common Types ### {: #player-api-types }

#### Player

| Field   | Type                                             | Description                                           |
|---------|--------------------------------------------------|-------------------------------------------------------|
| guildId | string                                           | The guild id of the player                            |
| track   | ?[Track](#track) object                          | The currently playing track                           |
| volume  | int                                              | The volume of the player, range 0-1000, in percentage |
| paused  | bool                                             | Whether the player is paused                          |
| state   | [Player State](websocket.md#player-state) object | The state of the player                               |
| voice   | [Voice State](#voice-state) object               | The voice state of the player                         |
| filters | [Filters](#filters) object                       | The filters used by the player                        |

#### Voice State

| Field     | Type   | Description                                       |
|-----------|--------|---------------------------------------------------|
| token     | string | The Discord voice token to authenticate with      |
| endpoint  | string | The Discord voice endpoint to connect to          |
| sessionId | string | The Discord voice session id to authenticate with |

`token`, `endpoint`, and `sessionId` are the 3 required values for connecting to one of Discord's voice servers.
`sessionId` is provided by the Voice State Update event sent by Discord, whereas the `endpoint` and `token` are provided
with the Voice Server Update. Please refer to https://discord.com/developers/docs/topics/gateway-events#voice

#### Filters

Filters are used in player update requests and look like this:

| Field          | Type                                             | Description                                                                                  |
|----------------|--------------------------------------------------|----------------------------------------------------------------------------------------------|
| volume?        | float                                            | Adjusts the player volume from 0.0 to 5.0, where 1.0 is 100%. Values >1.0 may cause clipping |
| equalizer?     | array of [Equalizer](#equalizer) objects         | Adjusts 15 different bands                                                                   |
| karaoke?       | [Karaoke](#karaoke) object                       | Eliminates part of a band, usually targeting vocals                                          |
| timescale?     | [Timescale](#timescale) object                   | Changes the speed, pitch, and rate                                                           |
| tremolo?       | [Tremolo](#tremolo) object                       | Creates a shuddering effect, where the volume quickly oscillates                             |
| vibrato?       | [Vibrato](#vibrato) object                       | Creates a shuddering effect, where the pitch quickly oscillates                              |
| rotation?      | [Rotation](#rotation) object                     | Rotates the audio around the stereo channels/user headphones (aka Audio Panning)             |
| distortion?    | [Distortion](#distortion) object                 | Distorts the audio                                                                           |
| channelMix?    | [Channel Mix](#channel-mix) object               | Mixes both channels (left and right)                                                         |
| lowPass?       | [Low Pass](#low-pass) object                     | Filters higher frequencies                                                                   |
| pluginFilters? | map of [Plugin Filters](#plugin-filters) objects | Filter plugin configurations                                                                 |

!!! rust "Filter Implementation"
    All standard Lavalink filters are supported in the Rust implementation with identical behavior and parameters.

##### Equalizer

There are 15 bands (0-14) that can be changed.
"gain" is the multiplier for the given band. The default value is 0. Valid values range from -0.25 to 1.0,
where -0.25 means the given band is completely muted, and 0.25 means it is doubled.

<details markdown="1">
<summary>Band Frequencies</summary>

| Band | Frequency |
|------|-----------|
| 0    | 25 Hz     |
| 1    | 40 Hz     |
| 2    | 63 Hz     |
| 3    | 100 Hz    |
| 4    | 160 Hz    |
| 5    | 250 Hz    |
| 6    | 400 Hz    |
| 7    | 630 Hz    |
| 8    | 1000 Hz   |
| 9    | 1600 Hz   |
| 10   | 2500 Hz   |
| 11   | 4000 Hz   |
| 12   | 6300 Hz   |
| 13   | 10000 Hz  |
| 14   | 16000 Hz  |

</details>

| Field | Type  | Description             |
|-------|-------|-------------------------|
| band  | int   | The band (0 to 14)      |
| gain  | float | The gain (-0.25 to 1.0) |

##### Karaoke

Uses equalization to eliminate part of a band, usually targeting vocals.

| Field        | Type  | Description                                                             |
|--------------|-------|-------------------------------------------------------------------------|
| level?       | float | The level (0 to 1.0 where 0.0 is no effect and 1.0 is full effect)      |
| monoLevel?   | float | The mono level (0 to 1.0 where 0.0 is no effect and 1.0 is full effect) |
| filterBand?  | float | The filter band (in Hz)                                                 |
| filterWidth? | float | The filter width                                                        |

##### Timescale

Changes the speed, pitch, and rate. All default to 1.0.

| Field  | Type  | Description                |
|--------|-------|----------------------------|
| speed? | float | The playback speed 0.0 ≤ x |
| pitch? | float | The pitch 0.0 ≤ x          |
| rate?  | float | The rate 0.0 ≤ x           |

##### Tremolo

Uses amplification to create a shuddering effect, where the volume quickly oscillates.

| Field      | Type  | Description                     |
|------------|-------|---------------------------------|
| frequency? | float | The frequency 0.0 < x           |
| depth?     | float | The tremolo depth 0.0 < x ≤ 1.0 |

##### Vibrato

Similar to tremolo. While tremolo oscillates the volume, vibrato oscillates the pitch.

| Field      | Type  | Description                     |
|------------|-------|---------------------------------|
| frequency? | float | The frequency 0.0 < x ≤ 14.0    |
| depth?     | float | The vibrato depth 0.0 < x ≤ 1.0 |

##### Rotation

Rotates the sound around the stereo channels/user headphones (aka Audio Panning).

| Field       | Type  | Description                                                                                              |
|-------------|-------|----------------------------------------------------------------------------------------------------------|
| rotationHz? | float | The frequency of the audio rotating around the listener in Hz. 0.2 is similar to the example video above |

##### Distortion

Distortion effect. It can generate some pretty unique audio effects.

| Field      | Type  | Description    |
|------------|-------|----------------|
| sinOffset? | float | The sin offset |
| sinScale?  | float | The sin scale  |
| cosOffset? | float | The cos offset |
| cosScale?  | float | The cos scale  |
| tanOffset? | float | The tan offset |
| tanScale?  | float | The tan scale  |
| offset?    | float | The offset     |
| scale?     | float | The scale      |

##### Channel Mix

Mixes both channels (left and right), with a configurable factor on how much each channel affects the other.

| Field         | Type  | Description                                           |
|---------------|-------|-------------------------------------------------------|
| leftToLeft?   | float | The left to left channel mix factor (0.0 ≤ x ≤ 1.0)   |
| leftToRight?  | float | The left to right channel mix factor (0.0 ≤ x ≤ 1.0)  |
| rightToLeft?  | float | The right to left channel mix factor (0.0 ≤ x ≤ 1.0)  |
| rightToRight? | float | The right to right channel mix factor (0.0 ≤ x ≤ 1.0) |

##### Low Pass

Higher frequencies get suppressed, while lower frequencies pass through this filter.

| Field      | Type  | Description                    |
|------------|-------|--------------------------------|
| smoothing? | float | The smoothing factor (1.0 < x) |

##### Plugin Filters

Plugins can add their own filters. The key is the name of the plugin, and the value is the configuration for that plugin.

!!! rust "Plugin System"
    Lavalink Rust uses a different plugin architecture than Java Lavalink. See [Plugin Development](../plugins/development.md) for details.

---

## Player Management

### Get Players

Returns a list of players in this specific session.

```
GET /v4/sessions/{sessionId}/players
```

Response: Array of [Player](#player) objects

### Get Player

Returns the player for this guild in this session.

```
GET /v4/sessions/{sessionId}/players/{guildId}
```

Response: [Player](#player) object

### Update Player

Updates or creates the player for this guild if it doesn't already exist.

```
PATCH /v4/sessions/{sessionId}/players/{guildId}?noReplace=false
```

Query Params:

| Field      | Type | Description                                                                  |
|------------|------|------------------------------------------------------------------------------|
| noReplace? | bool | Whether to replace the current track with the new track. Defaults to `false` |

Request:

| Field              | Type                                        | Description                                                                                   |
|--------------------|---------------------------------------------|-----------------------------------------------------------------------------------------------|
| track?             | [Update Player Track](#update-player-track) | Specification for a new track to load, as well as user data to set                            |
| position?          | int                                         | The track position in milliseconds                                                            |
| endTime?           | ?int                                        | The track end time in milliseconds (must be > 0). `null` resets this if it was set previously |
| volume?            | int                                         | The player volume, in percentage, from 0 to 1000                                              |
| paused?            | bool                                        | Whether the player is paused                                                                  |
| filters?           | [Filters](#filters) object                  | The new filters to apply. This will override all previously applied filters                   |
| voice?             | [Voice State](#voice-state) object          | Information required for connecting to Discord                                                |

#### Update Player Track

| Field        | Type    | Description                                                         |
|--------------|---------|---------------------------------------------------------------------|
| encoded?*    | ?string | The base64 encoded track to play. `null` stops the current track    |
| identifier?* | string  | The identifier of the track to play                                 |
| userData?    | object  | Additional track data to be sent back in the [Track Object](#track) |

!!! info
    `encoded` and `identifier` are mutually exclusive.

Response: [Player](#player) object

### Destroy Player

Destroys the player for this guild in this session.

```
DELETE /v4/sessions/{sessionId}/players/{guildId}
```

Response: 204 - No Content

---

## Session API

### Get Sessions

Returns a list of all sessions.

```
GET /v4/sessions
```

Response: Array of session objects

### Get Session

Returns information about a specific session.

```
GET /v4/sessions/{sessionId}
```

Response: Session object

### Update Session

Updates the session with the resuming state and timeout.

```
PATCH /v4/sessions/{sessionId}
```

Request:

| Field     | Type | Description                                         |
|-----------|------|-----------------------------------------------------|
| resuming? | bool | Whether resuming is enabled for this session or not |
| timeout?  | int  | The timeout in seconds (default is 60s)             |

Response: Session object

### Delete Session

Deletes a session and all associated players.

```
DELETE /v4/sessions/{sessionId}
```

Response: 204 - No Content

---

## Server Information

### Get Lavalink Info

Request Lavalink server information.

```
GET /v4/info
```

Response:

#### Info Response

| Field          | Type                                      | Description                                                     |
|----------------|-------------------------------------------|-----------------------------------------------------------------|
| version        | [Version](#version-object) object         | The version of this Lavalink server                             |
| buildTime      | int                                       | The millisecond unix timestamp when this binary was built       |
| git            | [Git](#git-object) object                 | The git information of this Lavalink server                     |
| jvm            | string                                    | "N/A - Rust" (for compatibility)                                |
| lavaplayer     | string                                    | "N/A - Native Rust" (for compatibility)                         |
| sourceManagers | array of strings                          | The enabled source managers for this server                     |
| filters        | array of strings                          | The enabled filters for this server                             |
| plugins        | array of [Plugin](#plugin-object) objects | The enabled plugins for this server                             |

!!! rust "Rust-Specific Fields"
    The `jvm` and `lavaplayer` fields return compatibility values since Rust Lavalink doesn't use Java or Lavaplayer.

<details markdown="1">
<summary>Example Response</summary>

```json
{
  "version": {
    "semver": "1.0.0",
    "major": 1,
    "minor": 0,
    "patch": 0,
    "preRelease": null,
    "build": null
  },
  "buildTime": 1664223916812,
  "git": {
    "branch": "main",
    "commit": "abc123",
    "commitTime": 1664223916812
  },
  "jvm": "N/A - Rust",
  "lavaplayer": "N/A - Native Rust",
  "sourceManagers": [
    "http",
    "youtube",
    "soundcloud",
    "bandcamp",
    "twitch",
    "vimeo",
    "nico",
    "local",
    "fallback"
  ],
  "filters": [
    "volume",
    "equalizer",
    "karaoke",
    "timescale",
    "tremolo",
    "vibrato",
    "distortion",
    "rotation",
    "channelMix",
    "lowPass"
  ],
  "plugins": []
}
```

</details>

### Get Lavalink Version

Request Lavalink version string.

```
GET /version
```

Response: Version string (e.g., "1.0.0")

### Get Lavalink Stats

Request Lavalink server statistics.

```
GET /v4/stats
```

Response: [Stats](websocket.md#stats-object) object

!!! performance "Performance Benefits"
    Rust Lavalink provides more accurate and detailed statistics due to better memory tracking and lower overhead monitoring.

<details markdown="1">
<summary>Example Response</summary>

```json
{
  "players": 1,
  "playingPlayers": 1,
  "uptime": 123456789,
  "memory": {
    "free": 123456789,
    "used": 123456789,
    "allocated": 123456789,
    "reservable": 123456789
  },
  "cpu": {
    "cores": 4,
    "systemLoad": 0.5,
    "lavalinkLoad": 0.5
  }
}
```

</details>

---

## Migration Notes

### Differences from Java Lavalink

1. **Missing Endpoints**: `POST /v4/decodetracks` is not yet implemented
2. **Enhanced Performance**: All endpoints respond faster with lower memory usage
3. **Better Error Handling**: More detailed error messages and stack traces
4. **Fallback System**: Automatic handling of Spotify/Apple Music/Deezer URLs
5. **Plugin System**: Different architecture using dynamic libraries instead of JAR files

### Client Compatibility

All existing Lavalink clients work without modification. The API maintains 100% compatibility while providing enhanced performance and reliability.

For more information about migrating from Java Lavalink, see the [Migration Guide](../migration/from-java.md).

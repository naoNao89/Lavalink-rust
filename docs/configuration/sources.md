---
description: Audio source configuration for Lavalink Rust
---

# Audio Sources Configuration

Lavalink Rust supports multiple audio sources with enhanced performance and a unique fallback system for unsupported platforms.

!!! rust "Enhanced Audio Sources"
    Rust Lavalink provides the same audio sources as Java Lavalink plus an intelligent fallback system that automatically converts Spotify, Apple Music, and Deezer URLs to YouTube searches.

## Supported Audio Sources

### YouTube

Full YouTube support including videos, playlists, and search functionality.

```yaml
lavalink:
  server:
    sources:
      youtube: true
    youtubeSearchEnabled: true
    youtubePlaylistLoadLimit: 6
```

**Features:**
- ✅ Video playback
- ✅ Playlist loading (configurable limit)
- ✅ Search functionality (`ytsearch:`)
- ✅ Age-restricted content support
- ✅ Live stream support

**Configuration Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `youtube` | boolean | `true` | Enable YouTube source |
| `youtubeSearchEnabled` | boolean | `true` | Enable YouTube search |
| `youtubePlaylistLoadLimit` | integer | `6` | Maximum tracks to load from playlists |

### SoundCloud

SoundCloud track and playlist support with search functionality.

```yaml
lavalink:
  server:
    sources:
      soundcloud: true
    soundcloudSearchEnabled: true
```

**Features:**
- ✅ Track playback
- ✅ Playlist support
- ✅ Search functionality (`scsearch:`)
- ✅ User profile tracks
- ✅ Private tracks (with proper authentication)

### Bandcamp

Bandcamp album and track support.

```yaml
lavalink:
  server:
    sources:
      bandcamp: true
```

**Features:**
- ✅ Individual track playback
- ✅ Full album support
- ✅ Artist discography
- ✅ High-quality audio streams
- ✅ Search functionality (`bcsearch:`)

### Twitch

Twitch live streams and VOD support.

```yaml
lavalink:
  server:
    sources:
      twitch: true
```

**Features:**
- ✅ Live stream playback
- ✅ VOD (Video on Demand) support
- ✅ Clip support
- ✅ Multiple quality options
- ✅ Chat replay (metadata only)

### Vimeo

Vimeo video support with audio extraction.

```yaml
lavalink:
  server:
    sources:
      vimeo: true
```

**Features:**
- ✅ Video audio extraction
- ✅ Private video support (with authentication)
- ✅ Multiple quality options
- ✅ Embedded video support

### HTTP Streams

Direct HTTP audio stream support.

```yaml
lavalink:
  server:
    sources:
      http: true
```

**Supported Formats:**
- ✅ MP3, AAC, OGG, FLAC, WAV
- ✅ M3U8 playlists
- ✅ Icecast/Shoutcast streams
- ✅ Custom HTTP headers support

**Configuration Example:**
```yaml
lavalink:
  server:
    httpConfig:
      proxyHost: ""
      proxyPort: 0
      proxyUser: ""
      proxyPassword: ""
```

### Local Files

Local file system access for audio files.

```yaml
lavalink:
  server:
    sources:
      local: true
```

**Features:**
- ✅ Local file playback (`file://` URLs)
- ✅ Network mounted drives
- ✅ Symbolic link support
- ✅ Metadata extraction

**Security Note:**
!!! warning "Security Consideration"
    Local file access should be carefully configured in production environments. Consider restricting access to specific directories.

## Fallback System (Rust-Specific)

The fallback system is a unique feature of Lavalink Rust that provides seamless compatibility with Spotify, Apple Music, and Deezer URLs.

```yaml
lavalink:
  server:
    sources:
      fallback: true

rust:
  fallback:
    enabled: true
    youtube_search_prefix: "ytsearch:"
    max_search_results: 5
    cache_duration: 3600
    search_accuracy: "high"  # "low", "medium", "high"
```

### How Fallback Works

1. **URL Detection**: Recognizes Spotify, Apple Music, and Deezer URLs
2. **Metadata Extraction**: Extracts track information (title, artist, album)
3. **Search Conversion**: Converts to YouTube search query
4. **Result Matching**: Finds the best matching track
5. **Caching**: Caches results to improve performance

### Supported Fallback Sources

| Platform | URL Pattern | Status | Notes |
|----------|-------------|--------|-------|
| Spotify | `open.spotify.com/track/*` | ✅ Full Support | Track metadata extraction |
| Spotify | `open.spotify.com/album/*` | ✅ Full Support | Album track listing |
| Spotify | `open.spotify.com/playlist/*` | ✅ Full Support | Playlist conversion |
| Apple Music | `music.apple.com/*` | ✅ Full Support | Track and album support |
| Deezer | `deezer.com/track/*` | ✅ Full Support | Track metadata extraction |

### Fallback Configuration

```yaml
rust:
  fallback:
    enabled: true                    # Enable fallback system
    youtube_search_prefix: "ytsearch:" # Search prefix for YouTube
    max_search_results: 5            # Maximum search results to consider
    cache_duration: 3600             # Cache duration in seconds
    search_accuracy: "high"          # Search accuracy level
    
    # Advanced options
    metadata_timeout: 10             # Metadata extraction timeout (seconds)
    search_timeout: 15               # Search timeout (seconds)
    retry_attempts: 3                # Number of retry attempts
    
    # Search query optimization
    include_artist: true             # Include artist in search query
    include_album: false             # Include album in search query
    remove_features: true            # Remove "(feat. ...)" from queries
    remove_remixes: false            # Remove remix indicators
```

### Fallback Performance

!!! performance "Performance Benefits"
    - **Fast Metadata Extraction**: ~200ms average response time
    - **Intelligent Caching**: Reduces repeated API calls
    - **Parallel Processing**: Multiple fallback requests processed concurrently
    - **Memory Efficient**: Minimal memory overhead for caching

## Source Priority and Load Balancing

Configure source priority and load balancing for optimal performance.

```yaml
lavalink:
  server:
    sources:
      youtube: true
      soundcloud: true
      bandcamp: true
      fallback: true
      
rust:
  sources:
    priority:
      - youtube      # Try YouTube first
      - soundcloud   # Then SoundCloud
      - bandcamp     # Then Bandcamp
      - fallback     # Finally fallback system
      
    load_balancing:
      enabled: true
      max_concurrent_requests: 10
      timeout_ms: 30000
      retry_attempts: 3
```

## Rate Limiting Configuration

Configure rate limiting to avoid being blocked by audio sources.

```yaml
lavalink:
  server:
    ratelimit:
      ipBlocks: []               # IP blocks to rotate through
      excludeIps: []             # IPs to exclude from rate limiting
      strategy: "RotateOnBan"    # Rate limiting strategy
      searchTriggersFail: true   # Whether search triggers fail on rate limit
      retryLimit: -1             # Retry limit (-1 = unlimited)
      
rust:
  ratelimit:
    youtube:
      requests_per_minute: 100   # YouTube API rate limit
      burst_size: 10             # Burst request allowance
      
    soundcloud:
      requests_per_minute: 200   # SoundCloud rate limit
      burst_size: 20
      
    fallback:
      requests_per_minute: 300   # Fallback system rate limit
      burst_size: 30
```

## Source-Specific Configuration

### YouTube Configuration

```yaml
lavalink:
  server:
    youtubeConfig:
      email: ""                  # YouTube account email (optional)
      password: ""               # YouTube account password (optional)
      
rust:
  sources:
    youtube:
      use_oauth: false           # Use OAuth instead of email/password
      quality_preference: "high" # "low", "medium", "high"
      extract_chapters: true     # Extract video chapters as tracks
      max_playlist_size: 1000    # Maximum playlist size to load
```

### SoundCloud Configuration

```yaml
rust:
  sources:
    soundcloud:
      client_id: ""              # SoundCloud client ID (optional)
      quality_preference: "high" # Audio quality preference
      extract_reposts: false     # Include reposts in user tracks
```

### HTTP Configuration

```yaml
lavalink:
  server:
    httpConfig:
      proxyHost: ""              # HTTP proxy host
      proxyPort: 0               # HTTP proxy port
      proxyUser: ""              # HTTP proxy username
      proxyPassword: ""          # HTTP proxy password
      
rust:
  sources:
    http:
      user_agent: "Lavalink-Rust/1.0.0"  # Custom user agent
      timeout_ms: 30000                   # Request timeout
      max_redirects: 5                    # Maximum redirects to follow
      verify_ssl: true                    # Verify SSL certificates
```

## Troubleshooting

### Common Issues

1. **YouTube Rate Limiting**
   ```yaml
   lavalink:
     server:
       ratelimit:
         strategy: "RotateOnBan"
         ipBlocks: ["1.2.3.4", "5.6.7.8"]  # Use multiple IPs
   ```

2. **Fallback System Not Working**
   ```yaml
   rust:
     fallback:
       enabled: true
       search_accuracy: "medium"  # Try lower accuracy
       metadata_timeout: 20       # Increase timeout
   ```

3. **Local Files Not Loading**
   ```bash
   # Check file permissions
   chmod 644 /path/to/audio/files/*
   
   # Use absolute paths
   file:///absolute/path/to/file.mp3
   ```

### Debug Configuration

Enable debug logging for audio sources:

```yaml
logging:
  level:
    lavalink::sources: DEBUG
    lavalink::fallback: DEBUG
    
rust:
  debug:
    log_source_requests: true    # Log all source requests
    log_fallback_queries: true   # Log fallback search queries
    log_metadata_extraction: true # Log metadata extraction
```

For more information, see:
- [Audio Filters Configuration](filters.md)
- [Performance Tuning](performance.md)
- [Troubleshooting Guide](../getting-started/troubleshooting.md)

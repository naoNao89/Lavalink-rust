---
description: Comprehensive guide to Lavalink Rust's fallback system for Spotify, Apple Music, and Deezer URLs
---

# Fallback System

Lavalink Rust includes an intelligent fallback system that provides seamless compatibility with Spotify, Apple Music, and Deezer URLs by automatically converting them to YouTube searches.

## Overview

### Why Fallback is Needed

Popular music streaming services like Spotify, Apple Music, and Deezer require:
- Official API access with licensing agreements
- Commercial partnerships not available to open-source projects
- Complex authentication and rate limiting

The fallback system provides a **legal alternative** that works transparently with existing client code.

### How It Works

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Client Bot    │───▶│  Lavalink Rust   │───▶│   YouTube API   │
│                 │    │                  │    │                 │
│ Spotify URL     │    │ 1. Detect URL    │    │ Search Results  │
│ Apple Music URL │    │ 2. Extract Info  │    │                 │
│ Deezer URL      │    │ 3. Convert Query │    │                 │
│                 │    │ 4. Search YouTube│    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Process Flow

1. **URL Detection**: Recognizes supported streaming service URLs
2. **Metadata Extraction**: Extracts track information (title, artist, album)
3. **Query Generation**: Creates optimized YouTube search queries
4. **Search Execution**: Performs YouTube search using yt-dlp
5. **Result Matching**: Finds the best matching track
6. **Caching**: Stores results for improved performance
7. **Transparent Return**: Returns YouTube results as if they were from the original service

## Supported Services

### Spotify

**Supported URL Formats:**
```
# Web URLs
https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh
https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3
https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M

# URI Format
spotify:track:4iV5W9uYEdYUVa79Axb7Rh
spotify:album:1DFixLWuPkv3KT3TnV35m3
spotify:playlist:37i9dQZF1DXcBWIGoYBM5M

# Share URLs
https://spotify.link/abc123
```

**Extraction Process:**
1. Parse URL to extract track/album/playlist ID
2. Use Spotify Web API (if configured) or URL scraping
3. Extract metadata: title, artist, album, duration
4. Generate search query: `"artist - title"` or `"artist title"`

### Apple Music

**Supported URL Formats:**
```
# Track URLs
https://music.apple.com/us/album/song-name/123456789?i=987654321
https://music.apple.com/gb/song/song-name/987654321

# Album URLs
https://music.apple.com/us/album/album-name/123456789

# Playlist URLs
https://music.apple.com/us/playlist/playlist-name/pl.abc123
```

**Extraction Process:**
1. Parse URL to extract region, type, and ID
2. Extract metadata from URL structure or API calls
3. Handle regional variations and redirects
4. Generate optimized search queries

### Deezer

**Supported URL Formats:**
```
# Track URLs
https://www.deezer.com/track/123456789
https://www.deezer.com/en/track/123456789

# Album URLs
https://www.deezer.com/album/123456789

# Playlist URLs
https://www.deezer.com/playlist/123456789
```

**Extraction Process:**
1. Parse URL to extract content type and ID
2. Use Deezer API or web scraping for metadata
3. Handle different language/region variants
4. Create search queries with artist and title

## Configuration

### Basic Configuration

```yaml
# application.yml
lavalink:
  server:
    sources:
      fallback: true              # Enable fallback system

rust:
  fallback:
    enabled: true                 # Master fallback switch
    youtube_search_prefix: "ytsearch:" # Search prefix for YouTube
    max_search_results: 5         # Maximum search results to consider
    cache_duration: 3600          # Cache duration in seconds (1 hour)
    search_accuracy: "high"       # Search accuracy: "low", "medium", "high"
```

### Advanced Configuration

```yaml
rust:
  fallback:
    # Core settings
    enabled: true
    youtube_search_prefix: "ytsearch:"
    max_search_results: 5
    cache_duration: 3600
    search_accuracy: "high"
    
    # Timeout settings
    metadata_timeout: 10          # Metadata extraction timeout (seconds)
    search_timeout: 15            # YouTube search timeout (seconds)
    total_timeout: 30             # Total fallback operation timeout
    
    # Retry settings
    retry_attempts: 3             # Number of retry attempts
    retry_delay: 1000             # Delay between retries (milliseconds)
    exponential_backoff: true     # Use exponential backoff for retries
    
    # Quality settings
    prefer_official: true         # Prefer official/verified channels
    prefer_audio_only: false      # Prefer audio-only results
    min_duration_match: 0.8       # Minimum duration match ratio (0.0-1.0)
    
    # Caching
    cache_enabled: true           # Enable result caching
    cache_size: 10000             # Maximum cache entries
    cache_ttl: 3600               # Cache time-to-live (seconds)
    
    # Logging
    log_conversions: true         # Log URL conversions
    log_cache_hits: false         # Log cache hits/misses
    log_search_queries: false     # Log search queries (debug only)
```

### Service-Specific Configuration

```yaml
rust:
  fallback:
    spotify:
      enabled: true
      api_client_id: "your_client_id"     # Optional: Spotify API credentials
      api_client_secret: "your_secret"    # Optional: for better metadata
      use_web_api: true                   # Use Spotify Web API if available
      fallback_to_scraping: true          # Fall back to URL scraping
      
    apple_music:
      enabled: true
      prefer_region: "us"                 # Preferred region for searches
      handle_redirects: true              # Follow Apple Music redirects
      
    deezer:
      enabled: true
      api_app_id: "your_app_id"          # Optional: Deezer API credentials
      use_api: true                       # Use Deezer API if available
      fallback_to_scraping: true          # Fall back to web scraping
```

## Usage Examples

### Client Code (No Changes Required)

```javascript
// JavaScript example with discord.js
const { Manager } = require("erela.js");

const manager = new Manager({
    nodes: [{
        host: "localhost",
        port: 2333,
        password: "youshallnotpass",
    }],
});

// These all work transparently with fallback
await manager.search("https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh");
await manager.search("https://music.apple.com/us/song/example/123456");
await manager.search("https://www.deezer.com/track/123456789");

// Results are returned as YouTube tracks
```

### Python Example

```python
import aiohttp
import asyncio

async def load_track(url):
    async with aiohttp.ClientSession() as session:
        params = {"identifier": url}
        async with session.get("http://localhost:2333/v4/loadtracks", params=params) as resp:
            result = await resp.json()
            return result

# Works with any supported URL
spotify_result = await load_track("https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh")
apple_result = await load_track("https://music.apple.com/us/song/example/123456")
deezer_result = await load_track("https://www.deezer.com/track/123456789")
```

### REST API Examples

```bash
# Spotify URL
curl "http://localhost:2333/v4/loadtracks?identifier=https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh"

# Apple Music URL
curl "http://localhost:2333/v4/loadtracks?identifier=https://music.apple.com/us/song/example/123456"

# Deezer URL
curl "http://localhost:2333/v4/loadtracks?identifier=https://www.deezer.com/track/123456789"
```

## Search Quality Optimization

### Search Accuracy Levels

**Low Accuracy (Fast):**
```yaml
rust:
  fallback:
    search_accuracy: "low"
    max_search_results: 3
    prefer_official: false
```
- Basic title + artist search
- Faster processing
- May have lower match quality

**Medium Accuracy (Balanced):**
```yaml
rust:
  fallback:
    search_accuracy: "medium"
    max_search_results: 5
    prefer_official: true
```
- Enhanced search queries
- Balanced speed vs quality
- Good for most use cases

**High Accuracy (Best Quality):**
```yaml
rust:
  fallback:
    search_accuracy: "high"
    max_search_results: 10
    prefer_official: true
    min_duration_match: 0.9
```
- Advanced matching algorithms
- Duration and metadata comparison
- Best match quality, slower processing

### Custom Search Strategies

```yaml
rust:
  fallback:
    search_strategies:
      - strategy: "exact_match"
        weight: 1.0
        query_format: '"{artist} - {title}"'
        
      - strategy: "artist_title"
        weight: 0.8
        query_format: "{artist} {title}"
        
      - strategy: "title_only"
        weight: 0.6
        query_format: "{title}"
        
      - strategy: "album_context"
        weight: 0.9
        query_format: "{artist} {title} {album}"
```

## Monitoring and Analytics

### Metrics

```yaml
metrics:
  prometheus:
    enabled: true
    
rust:
  fallback:
    metrics:
      enabled: true
      track_conversion_rate: true
      track_search_time: true
      track_cache_performance: true
```

**Available Metrics:**
- `lavalink_fallback_conversions_total{service="spotify|apple|deezer"}`
- `lavalink_fallback_search_duration_seconds`
- `lavalink_fallback_cache_hits_total`
- `lavalink_fallback_cache_misses_total`
- `lavalink_fallback_errors_total{type="timeout|api_error|not_found"}`

### Logging

```yaml
logging:
  level:
    lavalink.fallback: INFO      # Set to DEBUG for detailed logs

rust:
  fallback:
    log_conversions: true        # Log successful conversions
    log_failures: true           # Log failed conversions
    log_cache_operations: false  # Log cache hits/misses (debug only)
```

**Log Examples:**
```
INFO  lavalink.fallback - Converting Spotify URL: https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh
INFO  lavalink.fallback - Extracted metadata: "Rick Astley - Never Gonna Give You Up"
INFO  lavalink.fallback - YouTube search query: "Rick Astley Never Gonna Give You Up"
INFO  lavalink.fallback - Found match: https://www.youtube.com/watch?v=dQw4w9WgXcQ (confidence: 0.95)
```

## Troubleshooting

### Common Issues

**1. No Results Found:**
```yaml
# Increase search results and reduce accuracy requirements
rust:
  fallback:
    max_search_results: 10
    min_duration_match: 0.5
    search_accuracy: "medium"
```

**2. Poor Match Quality:**
```yaml
# Increase accuracy and enable official preference
rust:
  fallback:
    search_accuracy: "high"
    prefer_official: true
    min_duration_match: 0.9
```

**3. Slow Performance:**
```yaml
# Reduce search scope and enable caching
rust:
  fallback:
    max_search_results: 3
    search_timeout: 10
    cache_enabled: true
    cache_duration: 7200
```

**4. API Rate Limiting:**
```yaml
# Add delays and reduce retry attempts
rust:
  fallback:
    retry_attempts: 2
    retry_delay: 2000
    exponential_backoff: true
```

### Debugging

```bash
# Enable debug logging
export RUST_LOG=lavalink::fallback=debug

# Test specific URL
curl -v "http://localhost:2333/v4/loadtracks?identifier=https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh"

# Check metrics
curl http://localhost:9090/metrics | grep fallback

# Monitor logs
journalctl -u lavalink-rust -f | grep fallback
```

## Best Practices

### Performance Optimization

1. **Enable Caching:**
   ```yaml
   rust:
     fallback:
       cache_enabled: true
       cache_duration: 3600
       cache_size: 10000
   ```

2. **Optimize Search Parameters:**
   ```yaml
   rust:
     fallback:
       max_search_results: 5      # Balance quality vs speed
       search_timeout: 15         # Reasonable timeout
       prefer_official: true      # Better quality matches
   ```

3. **Use Appropriate Accuracy:**
   ```yaml
   rust:
     fallback:
       search_accuracy: "medium"  # Good balance for most use cases
   ```

### Quality Assurance

1. **Monitor Conversion Rates:**
   - Track successful vs failed conversions
   - Monitor search quality metrics
   - Adjust parameters based on results

2. **Test Common URLs:**
   - Regularly test popular tracks
   - Verify search quality
   - Check for service changes

3. **User Feedback:**
   - Collect user feedback on match quality
   - Adjust search strategies accordingly
   - Document known limitations

### Security Considerations

1. **API Credentials:**
   ```yaml
   rust:
     fallback:
       spotify:
         api_client_id: "${SPOTIFY_CLIENT_ID}"
         api_client_secret: "${SPOTIFY_CLIENT_SECRET}"
   ```

2. **Rate Limiting:**
   ```yaml
   rust:
     fallback:
       rate_limit:
         requests_per_minute: 100
         burst_size: 10
   ```

3. **Input Validation:**
   - URLs are validated before processing
   - Malicious URLs are rejected
   - Timeouts prevent hanging requests

## Migration from Java Lavalink

### Key Differences

| Aspect | Java Lavalink | Rust Lavalink |
|--------|---------------|---------------|
| **Spotify Support** | LavaSrc plugin required | Built-in fallback system |
| **Configuration** | Plugin-specific config | Native configuration |
| **Performance** | Plugin overhead | Native implementation |
| **Maintenance** | Separate plugin updates | Integrated updates |

### Migration Steps

1. **Remove LavaSrc Plugin:**
   ```yaml
   # Remove from application.yml
   # lavalink:
   #   plugins:
   #     - dependency: "com.github.topi314.lavasrc:lavasrc-plugin:3.2.1"
   ```

2. **Enable Fallback System:**
   ```yaml
   # Add to application.yml
   rust:
     fallback:
       enabled: true
   ```

3. **Test Functionality:**
   ```bash
   # Test Spotify URL conversion
   curl "http://localhost:2333/v4/loadtracks?identifier=https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh"
   ```

4. **Monitor Performance:**
   - Check conversion success rates
   - Monitor response times
   - Verify search quality

For more information, see the [Migration Guide](../migration/from-java.md).

## Future Enhancements

### Planned Features

- **Machine Learning**: AI-powered search result ranking
- **Metadata Enhancement**: Better track metadata extraction
- **Multi-Source Fallback**: Fallback to multiple sources (SoundCloud, Bandcamp)
- **User Preferences**: Personalized search result preferences
- **Advanced Caching**: Distributed caching for multiple instances

### Contributing

The fallback system is actively developed. Contributions are welcome for:
- New service support
- Search algorithm improvements
- Performance optimizations
- Bug fixes and testing

See the [Contributing Guide](https://github.com/lavalink-devs/lavalink-rust/blob/main/CONTRIBUTING.md) for details.

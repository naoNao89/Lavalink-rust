// Audio processing and source management module
// This will handle audio sources, track loading, and audio processing

#[cfg(feature = "discord")]
pub mod quality;
#[cfg(feature = "discord")]
pub mod streaming;

// Minimal stubs for non-Discord builds
#[cfg(not(feature = "discord"))]
pub mod quality {
    //! Minimal audio quality stubs for non-Discord builds
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AudioQualityConfig {
        pub bitrate: u32,
        pub sample_rate: u32,
        pub channels: u32,
    }

    impl Default for AudioQualityConfig {
        fn default() -> Self {
            Self {
                bitrate: 128_000,
                sample_rate: 48_000,
                channels: 2,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct AudioQualityManager;

    impl AudioQualityManager {
        pub fn new(_config: AudioQualityConfig) -> Self {
            Self
        }

        pub fn with_preset(_guild_id: String, _preset: QualityPreset) -> Self {
            Self
        }

        pub fn get_config(&self) -> AudioQualityConfig {
            AudioQualityConfig::default()
        }

        pub fn update_config(&mut self, _config: AudioQualityConfig) -> anyhow::Result<()> {
            Ok(())
        }

        pub fn apply_preset(&mut self, _preset: QualityPreset) -> anyhow::Result<()> {
            Ok(())
        }

        pub fn update_network_metrics(&mut self, _metrics: NetworkMetrics) {
            // No-op for standalone mode
        }

        pub fn network_quality_score(&self) -> f64 {
            1.0 // Perfect quality in standalone mode
        }

        pub fn is_quality_appropriate(&self) -> bool {
            true // Always appropriate in standalone mode
        }

        pub fn estimated_bandwidth(&self) -> u64 {
            1_000_000 // 1 Mbps default
        }

        #[allow(dead_code)]
        pub fn create_songbird_config(&self) -> anyhow::Result<()> {
            Ok(()) // No songbird in standalone mode
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct NetworkMetrics {
        pub latency_ms: u32,
        pub packet_loss: f32,
        pub jitter_ms: u32,
    }

    impl Default for NetworkMetrics {
        fn default() -> Self {
            Self {
                latency_ms: 50,
                packet_loss: 0.0,
                jitter_ms: 5,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum QualityPreset {
        Low,
        Medium,
        High,
    }
}

#[cfg(not(feature = "discord"))]
pub mod streaming {
    //! Minimal audio streaming stubs for non-Discord builds
    use anyhow::Result;

    #[derive(Clone)]
    pub struct AudioStreamingManager;

    impl AudioStreamingManager {
        pub fn new(_guild_id: String) -> Self {
            Self
        }

        #[allow(dead_code)]
        pub async fn start_stream(&self, _track_url: &str) -> Result<()> {
            Ok(())
        }

        pub async fn stop_stream(&self) -> Result<()> {
            Ok(())
        }

        pub async fn get_current_session(&self) -> Option<StreamSession> {
            None // No active sessions in standalone mode
        }

        pub async fn is_streaming(&self) -> bool {
            false // Never streaming in standalone mode
        }

        pub async fn get_stream_health(&self) -> u8 {
            100 // Perfect health in standalone mode
        }

        pub async fn get_stream_metrics(&self) -> StreamMetrics {
            StreamMetrics::default()
        }
    }

    #[derive(Debug, Clone)]
    pub struct StreamSession {
        #[allow(dead_code)]
        pub id: String,
        #[allow(dead_code)]
        pub started_at: std::time::Instant,
    }

    #[derive(Debug, Clone)]
    pub struct StreamHealth {
        #[allow(dead_code)]
        pub is_healthy: bool,
        #[allow(dead_code)]
        pub issues: Vec<String>,
    }

    impl Default for StreamHealth {
        fn default() -> Self {
            Self {
                is_healthy: true,
                issues: Vec::new(),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct StreamMetrics {
        #[allow(dead_code)]
        pub bytes_sent: u64,
        #[allow(dead_code)]
        pub packets_sent: u64,
        #[allow(dead_code)]
        pub errors: u64,
    }

    #[derive(Debug, Clone)]
    pub struct StreamOptions {
        #[allow(dead_code)]
        pub bitrate: u32,
        #[allow(dead_code)]
        pub sample_rate: u32,
        #[allow(dead_code)]
        pub channels: u8,
    }

    impl Default for StreamOptions {
        fn default() -> Self {
            Self {
                bitrate: 128_000,
                sample_rate: 48_000,
                channels: 2,
            }
        }
    }
}

// StreamState struct available for both Discord and non-Discord builds
#[derive(Debug, Clone, Default)]
pub struct StreamState {
    #[allow(dead_code)]
    pub is_active: bool,
    #[allow(dead_code)]
    pub position: u64,
}

use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use regex::Regex;
use tracing::{debug, info, warn};

use tokio::process::Command as AsyncCommand;

use crate::config::SourcesConfig;
use crate::protocol::{
    Exception, LoadResult, LoadResultData, LoadType, Severity, Track, TrackInfo,
};

/// Helper function to create a Track with conditional compilation for optional fields
fn create_track(encoded: String, info: TrackInfo) -> Track {
    Track {
        encoded,
        info,
        #[cfg(feature = "plugins")]
        plugin_info: std::collections::HashMap::new(),
        #[cfg(feature = "rest-api")]
        user_data: std::collections::HashMap::new(),
    }
}

/// Audio source manager for loading tracks from various sources
#[derive(Clone)]
pub struct AudioSourceManager {
    sources: Vec<AudioSourceType>,
}

/// Enum for different audio source types
#[derive(Clone)]
pub enum AudioSourceType {
    Http(HttpAudioSource),
    YouTube(YouTubeAudioSource),
    SoundCloud(SoundCloudAudioSource),
    Bandcamp(BandcampAudioSource),
    Twitch(TwitchAudioSource),
    Vimeo(VimeoAudioSource),
    Nico(NicoAudioSource),
    Local(LocalAudioSource),
    Fallback(FallbackAudioSource),
}

/// Trait for audio sources (YouTube, SoundCloud, etc.)
#[async_trait]
pub trait AudioSource {
    /// Get the name of this audio source
    fn name(&self) -> &str;

    /// Check if this source can handle the given identifier
    fn can_handle(&self, identifier: &str) -> bool;

    /// Load a track from this source
    async fn load_track(&self, identifier: &str) -> Result<LoadResult>;

    /// Search for tracks from this source
    async fn search(&self, query: &str) -> Result<LoadResult>;
}

/// HTTP audio source for direct URLs
#[derive(Clone)]
pub struct HttpAudioSource;

/// YouTube audio source (placeholder)
#[derive(Clone)]
pub struct YouTubeAudioSource;

/// SoundCloud audio source (placeholder)
#[derive(Clone)]
pub struct SoundCloudAudioSource;

impl SoundCloudAudioSource {
    #[allow(dead_code)] // Used in tests
    pub fn new() -> Self {
        Self
    }
}

impl Default for SoundCloudAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Bandcamp audio source (placeholder)
#[derive(Clone)]
pub struct BandcampAudioSource;

impl BandcampAudioSource {
    #[allow(dead_code)] // Used in tests
    pub fn new() -> Self {
        Self
    }
}

impl Default for BandcampAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Twitch audio source (placeholder)
#[derive(Clone)]
pub struct TwitchAudioSource;

impl TwitchAudioSource {
    #[allow(dead_code)] // Used in tests
    pub fn new() -> Self {
        Self
    }
}

impl Default for TwitchAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Vimeo audio source (placeholder)
#[derive(Clone)]
pub struct VimeoAudioSource;

/// Niconico audio source (placeholder)
#[derive(Clone)]
pub struct NicoAudioSource;

/// Local file audio source (placeholder)
#[derive(Clone)]
pub struct LocalAudioSource;

/// Fallback audio source for unsupported platforms (Spotify, Apple Music, Deezer)
/// Converts unsupported URLs to YouTube searches
#[derive(Clone)]
pub struct FallbackAudioSource;

impl AudioSourceManager {
    /// Create a new audio source manager
    pub fn new() -> Self {
        Self::with_config(None)
    }

    /// Create a new audio source manager with configuration
    pub fn with_config(config: Option<&SourcesConfig>) -> Self {
        let mut sources = Vec::new();

        // Order matters: more specific sources should be checked first
        if config.is_none_or(|c| c.youtube.unwrap_or(true)) {
            sources.push(AudioSourceType::YouTube(YouTubeAudioSource));
        }
        if config.is_none_or(|c| c.soundcloud.unwrap_or(true)) {
            sources.push(AudioSourceType::SoundCloud(SoundCloudAudioSource));
        }
        if config.is_none_or(|c| c.bandcamp.unwrap_or(true)) {
            sources.push(AudioSourceType::Bandcamp(BandcampAudioSource));
        }
        if config.is_none_or(|c| c.twitch.unwrap_or(true)) {
            sources.push(AudioSourceType::Twitch(TwitchAudioSource));
        }
        if config.is_none_or(|c| c.vimeo.unwrap_or(true)) {
            sources.push(AudioSourceType::Vimeo(VimeoAudioSource));
        }
        if config.is_some_and(|c| c.nico.unwrap_or(false)) {
            sources.push(AudioSourceType::Nico(NicoAudioSource));
        }
        if config.is_some_and(|c| c.local.unwrap_or(false)) {
            sources.push(AudioSourceType::Local(LocalAudioSource));
        }

        // Always add fallback for unsupported sources
        sources.push(AudioSourceType::Fallback(FallbackAudioSource));

        // HTTP should be last as fallback
        if config.is_none_or(|c| c.http.unwrap_or(true)) {
            sources.push(AudioSourceType::Http(HttpAudioSource));
        }

        Self { sources }
    }

    /// Load a track from any available source
    pub async fn load_item(&self, identifier: &str) -> Result<LoadResult> {
        // Try each source in order until one can handle the identifier
        for source in &self.sources {
            if source.can_handle(identifier) {
                return source.load_track(identifier).await;
            }
        }

        // If no source can handle it, return an error with no data
        Ok(LoadResult {
            load_type: LoadType::Error,
            data: None,
        })
    }
}

// Implementation for AudioSourceType enum
#[async_trait]
impl AudioSource for AudioSourceType {
    fn name(&self) -> &str {
        match self {
            AudioSourceType::Http(source) => source.name(),
            AudioSourceType::YouTube(source) => source.name(),
            AudioSourceType::SoundCloud(source) => source.name(),
            AudioSourceType::Bandcamp(source) => source.name(),
            AudioSourceType::Twitch(source) => source.name(),
            AudioSourceType::Vimeo(source) => source.name(),
            AudioSourceType::Nico(source) => source.name(),
            AudioSourceType::Local(source) => source.name(),
            AudioSourceType::Fallback(source) => source.name(),
        }
    }

    fn can_handle(&self, identifier: &str) -> bool {
        match self {
            AudioSourceType::Http(source) => source.can_handle(identifier),
            AudioSourceType::YouTube(source) => source.can_handle(identifier),
            AudioSourceType::SoundCloud(source) => source.can_handle(identifier),
            AudioSourceType::Bandcamp(source) => source.can_handle(identifier),
            AudioSourceType::Twitch(source) => source.can_handle(identifier),
            AudioSourceType::Vimeo(source) => source.can_handle(identifier),
            AudioSourceType::Nico(source) => source.can_handle(identifier),
            AudioSourceType::Local(source) => source.can_handle(identifier),
            AudioSourceType::Fallback(source) => source.can_handle(identifier),
        }
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        match self {
            AudioSourceType::Http(source) => source.load_track(identifier).await,
            AudioSourceType::YouTube(source) => source.load_track(identifier).await,
            AudioSourceType::SoundCloud(source) => source.load_track(identifier).await,
            AudioSourceType::Bandcamp(source) => source.load_track(identifier).await,
            AudioSourceType::Twitch(source) => source.load_track(identifier).await,
            AudioSourceType::Vimeo(source) => source.load_track(identifier).await,
            AudioSourceType::Nico(source) => source.load_track(identifier).await,
            AudioSourceType::Local(source) => source.load_track(identifier).await,
            AudioSourceType::Fallback(source) => source.load_track(identifier).await,
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        match self {
            AudioSourceType::Http(source) => source.search(query).await,
            AudioSourceType::YouTube(source) => source.search(query).await,
            AudioSourceType::SoundCloud(source) => source.search(query).await,
            AudioSourceType::Bandcamp(source) => source.search(query).await,
            AudioSourceType::Twitch(source) => source.search(query).await,
            AudioSourceType::Vimeo(source) => source.search(query).await,
            AudioSourceType::Nico(source) => source.search(query).await,
            AudioSourceType::Local(source) => source.search(query).await,
            AudioSourceType::Fallback(source) => source.search(query).await,
        }
    }
}

// Placeholder implementations for audio sources
#[async_trait]
impl AudioSource for HttpAudioSource {
    fn name(&self) -> &str {
        "http"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        // HTTP source should only handle direct HTTP/HTTPS URLs that are not handled by other sources
        if !identifier.starts_with("http://") && !identifier.starts_with("https://") {
            return false;
        }

        // Don't handle URLs that should be handled by specific sources
        if identifier.contains("youtube.com")
            || identifier.contains("youtu.be")
            || identifier.contains("soundcloud.com")
            || identifier.contains("bandcamp.com")
            || identifier.contains("twitch.tv")
            || identifier.contains("vimeo.com")
            || identifier.contains("nicovideo.jp")
        {
            return false;
        }

        true
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Validate URL format
        if !identifier.starts_with("http://") && !identifier.starts_with("https://") {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid HTTP URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL must start with http:// or https://".to_string(),
                })),
            });
        }

        // Create HTTP client
        let client = reqwest::Client::new();

        // Send HEAD request to get metadata
        match client.head(identifier).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // Extract metadata from headers
                    let content_length = response
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);

                    let _content_type = response
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("audio/unknown");

                    // Create track info
                    let track_info = crate::protocol::TrackInfo {
                        identifier: identifier.to_string(),
                        is_seekable: false, // HTTP streams are generally not seekable
                        author: "Unknown".to_string(),
                        length: if content_length > 0 {
                            content_length
                        } else {
                            0
                        },
                        is_stream: content_length == 0, // If no content-length, assume it's a stream
                        position: 0,
                        title: extract_title_from_url(identifier),
                        uri: Some(identifier.to_string()),
                        artwork_url: None,
                        isrc: None,
                        source_name: "http".to_string(),
                    };

                    // Create encoded track (base64 encoded JSON for now)
                    let track_data = serde_json::json!({
                        "identifier": identifier,
                        "source": "http",
                        "uri": identifier
                    });
                    let encoded =
                        base64::engine::general_purpose::STANDARD.encode(track_data.to_string());

                    let track = create_track(encoded, track_info);

                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some(format!("HTTP error: {}", response.status())),
                            severity: Severity::Common,
                            cause: "Failed to access HTTP resource".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("Failed to load HTTP resource: {e}")),
                    severity: Severity::Common,
                    cause: "Network error".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, _query: &str) -> Result<LoadResult> {
        Ok(LoadResult {
            load_type: LoadType::Empty,
            data: None,
        })
    }
}

#[async_trait]
impl AudioSource for YouTubeAudioSource {
    fn name(&self) -> &str {
        "youtube"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("youtube.com")
            || identifier.contains("youtu.be")
            || identifier.starts_with("ytsearch:")
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Handle different YouTube URL formats and search queries
        if identifier.starts_with("ytsearch:") {
            // For search queries, use the search method
            return self
                .search(identifier.strip_prefix("ytsearch:").unwrap_or(identifier))
                .await;
        }

        // Direct URL - validate and normalize
        if !self.is_valid_youtube_url(identifier) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid YouTube URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL is not a valid YouTube URL".to_string(),
                })),
            });
        }

        // Use yt-dlp command-line to extract video information
        match self.extract_video_info(identifier).await {
            Ok(tracks) => {
                if let Some(track) = tracks.into_iter().next() {
                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some("Failed to extract video information".to_string()),
                            severity: Severity::Common,
                            cause: "Video not found or unavailable".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("YouTube extraction failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp error".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // Use yt-dlp command-line to search YouTube
        match self.extract_video_info(&format!("ytsearch5:{query}")).await {
            Ok(tracks) => {
                if tracks.is_empty() {
                    Ok(LoadResult {
                        load_type: LoadType::Empty,
                        data: None,
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Search,
                        data: Some(LoadResultData::Search(tracks)),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("YouTube search failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp search error".to_string(),
                })),
            }),
        }
    }
}

impl YouTubeAudioSource {
    /// Check if a URL is a valid YouTube URL
    fn is_valid_youtube_url(&self, url: &str) -> bool {
        let youtube_patterns = [
            r"^https?://(www\.)?youtube\.com/watch\?v=[\w-]+",
            r"^https?://(www\.)?youtu\.be/[\w-]+",
            r"^https?://(www\.)?youtube\.com/playlist\?list=[\w-]+",
            r"^https?://(www\.)?youtube\.com/channel/[\w-]+",
            r"^https?://(www\.)?youtube\.com/user/[\w-]+",
        ];

        youtube_patterns.iter().any(|pattern| {
            Regex::new(pattern)
                .map(|re| re.is_match(url))
                .unwrap_or(false)
        })
    }

    /// Extract video information using yt-dlp command-line
    async fn extract_video_info(&self, identifier: &str) -> Result<Vec<crate::protocol::Track>> {
        // Check if yt-dlp is available
        if !self.is_ytdlp_available().await {
            return Err(anyhow::anyhow!(
                "yt-dlp is not installed or not available in PATH"
            ));
        }

        // Build yt-dlp command using system PATH
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.args([
            "--dump-json",
            "--no-playlist",   // For single videos, don't expand playlists
            "--flat-playlist", // For playlists, get basic info only
            "--no-warnings",
            "--ignore-errors",
            identifier,
        ]);

        // Execute command
        let output = cmd.output().await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", error_msg));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        // yt-dlp outputs one JSON object per line for multiple results
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(track) = self.json_to_track(json) {
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse yt-dlp JSON output: {}", e);
                }
            }
        }

        Ok(tracks)
    }

    /// Check if yt-dlp is available in the system
    async fn is_ytdlp_available(&self) -> bool {
        AsyncCommand::new("yt-dlp")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Convert yt-dlp JSON output to Track
    fn json_to_track(&self, json: serde_json::Value) -> Option<crate::protocol::Track> {
        let id = json.get("id")?.as_str()?;
        let title = json.get("title")?.as_str().unwrap_or("Unknown Title");
        let uploader = json.get("uploader")?.as_str().unwrap_or("Unknown");
        let duration = json.get("duration")?.as_f64().unwrap_or(0.0) as u64 * 1000; // Convert to milliseconds
        let url = json
            .get("webpage_url")?
            .as_str()
            .or_else(|| json.get("url")?.as_str())?;

        // Get thumbnail from thumbnails array if available
        let thumbnail = json
            .get("thumbnails")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.last())
            .and_then(|thumb| thumb.get("url"))
            .and_then(|url| url.as_str())
            .or_else(|| json.get("thumbnail")?.as_str());

        // Create track info
        let track_info = crate::protocol::TrackInfo {
            identifier: id.to_string(),
            is_seekable: true,
            author: uploader.to_string(),
            length: duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri: Some(url.to_string()),
            artwork_url: thumbnail.map(|s| s.to_string()),
            isrc: None,
            source_name: "youtube".to_string(),
        };

        // Create encoded track data
        let track_data = serde_json::json!({
            "identifier": id,
            "source": "youtube",
            "uri": url,
            "title": title,
            "author": uploader,
            "duration": duration
        });

        let encoded = base64::engine::general_purpose::STANDARD.encode(track_data.to_string());

        Some(create_track(encoded, track_info))
    }
}

// Placeholder implementations for other audio sources

#[async_trait]
impl AudioSource for SoundCloudAudioSource {
    fn name(&self) -> &str {
        "soundcloud"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        // Handle SoundCloud URLs and search queries
        identifier.contains("soundcloud.com")
            || identifier.contains("snd.sc")
            || identifier.starts_with("scsearch:")
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Handle different SoundCloud URL formats and search queries
        if identifier.starts_with("scsearch:") {
            // For search queries, use the search method
            return self
                .search(identifier.strip_prefix("scsearch:").unwrap_or(identifier))
                .await;
        }

        // Direct URL - validate and normalize
        if !self.is_valid_soundcloud_url(identifier) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid SoundCloud URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL is not a valid SoundCloud URL".to_string(),
                })),
            });
        }

        // Use yt-dlp to extract track information
        match self.extract_track_info(identifier).await {
            Ok(track_info) => {
                // For direct URLs, return as a single track
                if let Some(track) = track_info.into_iter().next() {
                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some("Failed to extract track information".to_string()),
                            severity: Severity::Common,
                            cause: "Track not found or unavailable".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("SoundCloud extraction failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp error".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // Use yt-dlp to search SoundCloud
        let search_query = format!("scsearch5:{query}");

        match self.extract_track_info(&search_query).await {
            Ok(tracks) => {
                if tracks.is_empty() {
                    Ok(LoadResult {
                        load_type: LoadType::Empty,
                        data: None,
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Search,
                        data: Some(LoadResultData::Search(tracks)),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("SoundCloud search failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp search error".to_string(),
                })),
            }),
        }
    }
}

impl SoundCloudAudioSource {
    /// Validate if the identifier is a valid SoundCloud URL
    fn is_valid_soundcloud_url(&self, identifier: &str) -> bool {
        // SoundCloud URL patterns
        let patterns = [
            r"^https?://soundcloud\.com/[\w-]+/[\w-]+", // Track URL
            r"^https?://soundcloud\.com/[\w-]+/sets/[\w-]+", // Playlist URL
            r"^https?://soundcloud\.com/[\w-]+",        // User URL
            r"^https?://snd\.sc/\w+",                   // Short URL
        ];

        patterns.iter().any(|pattern| {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(identifier))
                .unwrap_or(false)
        })
    }

    /// Extract track information using yt-dlp
    async fn extract_track_info(&self, identifier: &str) -> Result<Vec<crate::protocol::Track>> {
        // Check if yt-dlp is available
        if !self.is_ytdlp_available().await {
            return Err(anyhow::anyhow!(
                "yt-dlp is not installed or not available in PATH"
            ));
        }

        // Build yt-dlp command using system PATH
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.args([
            "--dump-json",
            "--no-playlist",   // For single tracks, don't expand playlists
            "--flat-playlist", // For playlists, get basic info only
            "--no-warnings",
            "--ignore-errors",
            identifier,
        ]);

        // Execute command
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(track) = self.json_to_track(json) {
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse JSON line: {line} - Error: {e}");
                }
            }
        }

        Ok(tracks)
    }

    /// Check if yt-dlp is available in the system
    async fn is_ytdlp_available(&self) -> bool {
        AsyncCommand::new("yt-dlp")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Convert yt-dlp JSON output to Track
    fn json_to_track(&self, json: serde_json::Value) -> Option<crate::protocol::Track> {
        let id = json.get("id")?.as_str()?;
        let title = json.get("title")?.as_str().unwrap_or("Unknown Title");
        let uploader = json.get("uploader")?.as_str().unwrap_or("Unknown");
        let duration = json.get("duration")?.as_f64().unwrap_or(0.0) as u64 * 1000; // Convert to milliseconds
        let url = json
            .get("webpage_url")?
            .as_str()
            .or_else(|| json.get("url")?.as_str())?;

        // Get thumbnail from thumbnails array if available
        let thumbnail = json
            .get("thumbnails")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.last())
            .and_then(|thumb| thumb.get("url"))
            .and_then(|url| url.as_str())
            .or_else(|| json.get("thumbnail")?.as_str());

        let track_info = crate::protocol::TrackInfo {
            identifier: id.to_string(),
            is_seekable: true,
            author: uploader.to_string(),
            length: duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri: Some(url.to_string()),
            artwork_url: thumbnail.map(|s| s.to_string()),
            isrc: None,
            source_name: "soundcloud".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::json!({
            "identifier": id,
            "source": "soundcloud",
            "uri": url,
            "title": title,
            "author": uploader,
            "duration": duration
        });

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            track_data.to_string(),
        );

        Some(create_track(encoded, track_info))
    }
}

#[async_trait]
impl AudioSource for BandcampAudioSource {
    fn name(&self) -> &str {
        "bandcamp"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("bandcamp.com") || identifier.starts_with("bcsearch:")
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Handle Bandcamp search queries
        if identifier.starts_with("bcsearch:") {
            return self
                .search(identifier.strip_prefix("bcsearch:").unwrap_or(identifier))
                .await;
        }

        // Direct URL - validate and normalize
        if !self.is_valid_bandcamp_url(identifier) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid Bandcamp URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL is not a valid Bandcamp URL".to_string(),
                })),
            });
        }

        // Use yt-dlp to extract track information
        match self.extract_track_info(identifier).await {
            Ok(track_info) => {
                if let Some(track) = track_info.into_iter().next() {
                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some("Failed to extract track information".to_string()),
                            severity: Severity::Common,
                            cause: "Track not found or unavailable".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("Bandcamp extraction failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp error".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        info!("Searching Bandcamp for: {}", query);

        // Implement basic Bandcamp search using web scraping
        match self.search_bandcamp_web(query).await {
            Ok(tracks) => {
                if tracks.is_empty() {
                    Ok(LoadResult {
                        load_type: LoadType::Empty,
                        data: None,
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Search,
                        data: Some(LoadResultData::Search(tracks)),
                    })
                }
            }
            Err(e) => {
                warn!("Bandcamp search failed: {}", e);
                Ok(LoadResult {
                    load_type: LoadType::Error,
                    data: Some(LoadResultData::Exception(Exception {
                        message: Some(format!(
                            "Bandcamp search failed: {e}. Try using direct Bandcamp URLs instead."
                        )),
                        severity: Severity::Common,
                        cause: format!("Search error: {e}"),
                    })),
                })
            }
        }
    }
}

impl BandcampAudioSource {
    /// Search Bandcamp using web scraping
    async fn search_bandcamp_web(&self, query: &str) -> Result<Vec<crate::protocol::Track>> {
        let search_url = format!(
            "https://bandcamp.com/search?q={}",
            urlencoding::encode(query)
        );

        // Add rate limiting to be respectful
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let response = reqwest::Client::new()
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Lavalink-Rust/4.0)")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let html = response.text().await?;
        self.parse_bandcamp_search_results(&html, query).await
    }

    /// Parse Bandcamp search results from HTML
    async fn parse_bandcamp_search_results(
        &self,
        html: &str,
        query: &str,
    ) -> Result<Vec<crate::protocol::Track>> {
        let mut tracks = Vec::new();

        // Simple regex-based parsing for Bandcamp search results
        let track_pattern = regex::Regex::new(
            r#"<div class="searchresult track">.*?<div class="heading">.*?<a href="([^"]+)"[^>]*>([^<]+)</a>.*?<div class="subhead">.*?by\s*<a[^>]*>([^<]+)</a>"#
        ).unwrap();

        for (count, captures) in track_pattern.captures_iter(html).enumerate() {
            if count >= 10 {
                break; // Limit to 10 results
            }

            let url = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let title = captures
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or("Unknown Title");
            let artist = captures
                .get(3)
                .map(|m| m.as_str())
                .unwrap_or("Unknown Artist");

            // Clean up HTML entities
            let title = html_escape::decode_html_entities(title).to_string();
            let artist = html_escape::decode_html_entities(artist).to_string();

            // Create a track info
            let track_info = crate::protocol::TrackInfo {
                identifier: url.to_string(),
                is_seekable: true,
                author: artist.clone(),
                length: 0, // Unknown length from search results
                is_stream: false,
                position: 0,
                title: title.clone(),
                uri: Some(url.to_string()),
                artwork_url: None,
                isrc: None,
                source_name: "bandcamp".to_string(),
            };

            // Create encoded track data
            let track_data = serde_json::to_vec(&track_info)?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&track_data);

            let track = create_track(encoded, track_info);

            tracks.push(track);
        }

        info!(
            "Found {} Bandcamp tracks for query: {}",
            tracks.len(),
            query
        );
        Ok(tracks)
    }

    /// Validate if the identifier is a valid Bandcamp URL
    fn is_valid_bandcamp_url(&self, identifier: &str) -> bool {
        // Bandcamp URL patterns
        let patterns = [
            r"^https?://[\w-]+\.bandcamp\.com/track/[\w-]+", // Track URL
            r"^https?://[\w-]+\.bandcamp\.com/album/[\w-]+", // Album URL
            r"^https?://bandcamp\.com/[\w-]+",               // Artist page
        ];

        patterns.iter().any(|pattern| {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(identifier))
                .unwrap_or(false)
        })
    }

    /// Extract track information using yt-dlp
    async fn extract_track_info(&self, identifier: &str) -> Result<Vec<crate::protocol::Track>> {
        // Check if yt-dlp is available
        if !self.is_ytdlp_available().await {
            return Err(anyhow::anyhow!(
                "yt-dlp is not installed or not available in PATH"
            ));
        }

        // Build yt-dlp command using system PATH
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.args([
            "--dump-json",
            "--no-playlist",
            "--flat-playlist",
            "--no-warnings",
            "--ignore-errors",
            identifier,
        ]);

        // Execute command
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(track) = self.json_to_track(json) {
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse JSON line: {line} - Error: {e}");
                }
            }
        }

        Ok(tracks)
    }

    /// Check if yt-dlp is available in the system
    async fn is_ytdlp_available(&self) -> bool {
        AsyncCommand::new("yt-dlp")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Convert yt-dlp JSON output to Track
    fn json_to_track(&self, json: serde_json::Value) -> Option<crate::protocol::Track> {
        let id = json.get("id")?.as_str()?;
        let title = json.get("title")?.as_str().unwrap_or("Unknown Title");
        let uploader = json.get("uploader")?.as_str().unwrap_or("Unknown");
        let duration = json.get("duration")?.as_f64().unwrap_or(0.0) as u64 * 1000;
        let url = json
            .get("webpage_url")?
            .as_str()
            .or_else(|| json.get("url")?.as_str())?;

        // Get thumbnail from thumbnails array if available
        let thumbnail = json
            .get("thumbnails")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.last())
            .and_then(|thumb| thumb.get("url"))
            .and_then(|url| url.as_str())
            .or_else(|| json.get("thumbnail")?.as_str());

        let track_info = crate::protocol::TrackInfo {
            identifier: id.to_string(),
            is_seekable: true,
            author: uploader.to_string(),
            length: duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri: Some(url.to_string()),
            artwork_url: thumbnail.map(|s| s.to_string()),
            isrc: None,
            source_name: "bandcamp".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::json!({
            "identifier": id,
            "source": "bandcamp",
            "uri": url,
            "title": title,
            "author": uploader,
            "duration": duration
        });

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            track_data.to_string(),
        );

        Some(create_track(encoded, track_info))
    }
}

#[async_trait]
impl AudioSource for TwitchAudioSource {
    fn name(&self) -> &str {
        "twitch"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("twitch.tv") || identifier.starts_with("twsearch:")
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Handle Twitch search queries
        if identifier.starts_with("twsearch:") {
            return self
                .search(identifier.strip_prefix("twsearch:").unwrap_or(identifier))
                .await;
        }

        // Direct URL - validate and normalize
        if !self.is_valid_twitch_url(identifier) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid Twitch URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL is not a valid Twitch URL".to_string(),
                })),
            });
        }

        // Use yt-dlp to extract stream information
        match self.extract_stream_info(identifier).await {
            Ok(stream_info) => {
                if let Some(track) = stream_info.into_iter().next() {
                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some("Failed to extract stream information".to_string()),
                            severity: Severity::Common,
                            cause: "Stream not found or offline".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("Twitch extraction failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp error or stream offline".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // Twitch search is limited - we can try to construct a channel URL
        let channel_url = if query.starts_with("https://") {
            query.to_string()
        } else {
            format!("https://www.twitch.tv/{query}")
        };

        match self.extract_stream_info(&channel_url).await {
            Ok(tracks) => {
                if tracks.is_empty() {
                    Ok(LoadResult {
                        load_type: LoadType::Empty,
                        data: None,
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Search,
                        data: Some(LoadResultData::Search(tracks)),
                    })
                }
            }
            Err(_) => {
                // Return empty result for search failures (channel might be offline)
                Ok(LoadResult {
                    load_type: LoadType::Empty,
                    data: None,
                })
            }
        }
    }
}

impl TwitchAudioSource {
    /// Check if a URL is a valid Twitch URL
    fn is_valid_twitch_url(&self, url: &str) -> bool {
        let twitch_patterns = [
            r"^https?://(www\.)?twitch\.tv/[\w-]+$",     // Channel URL
            r"^https?://(www\.)?twitch\.tv/videos/\d+$", // VOD URL
            r"^https?://(www\.)?twitch\.tv/[\w-]+/clip/[\w-]+$", // Clip URL
        ];

        twitch_patterns.iter().any(|pattern| {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(url))
                .unwrap_or(false)
        })
    }

    /// Extract stream information using yt-dlp
    async fn extract_stream_info(&self, identifier: &str) -> Result<Vec<crate::protocol::Track>> {
        // Check if yt-dlp is available
        if !self.is_ytdlp_available().await {
            return Err(anyhow::anyhow!(
                "yt-dlp is not installed or not available in PATH"
            ));
        }

        // Build yt-dlp command with full path
        let yt_dlp_path = "/Users/henri89/Library/Python/3.9/bin/yt-dlp";
        let mut cmd = AsyncCommand::new(yt_dlp_path);
        cmd.args([
            "--dump-json",
            "--no-playlist",
            "--flat-playlist",
            "--no-warnings",
            "--ignore-errors",
            identifier,
        ]);

        // Execute command
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(track) = self.json_to_track(json) {
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse JSON line: {line} - Error: {e}");
                }
            }
        }

        Ok(tracks)
    }

    /// Check if yt-dlp is available in the system
    async fn is_ytdlp_available(&self) -> bool {
        let yt_dlp_path = "/Users/henri89/Library/Python/3.9/bin/yt-dlp";
        AsyncCommand::new(yt_dlp_path)
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Convert yt-dlp JSON output to Track
    fn json_to_track(&self, json: serde_json::Value) -> Option<crate::protocol::Track> {
        let id = json.get("id")?.as_str()?;
        let title = json.get("title")?.as_str().unwrap_or("Unknown Title");
        let uploader = json.get("uploader")?.as_str().unwrap_or("Unknown");

        // For live streams, duration might be null or very long
        let duration = json
            .get("duration")
            .and_then(|d| d.as_f64())
            .map(|d| d as u64 * 1000)
            .unwrap_or(0); // 0 indicates live stream

        let url = json
            .get("webpage_url")?
            .as_str()
            .or_else(|| json.get("url")?.as_str())?;

        // Check if this is a live stream
        let is_live = json
            .get("is_live")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Get thumbnail from thumbnails array if available
        let thumbnail = json
            .get("thumbnails")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.last())
            .and_then(|thumb| thumb.get("url"))
            .and_then(|url| url.as_str())
            .or_else(|| json.get("thumbnail")?.as_str());

        let track_info = crate::protocol::TrackInfo {
            identifier: id.to_string(),
            is_seekable: !is_live, // Live streams are not seekable
            author: uploader.to_string(),
            length: duration,
            is_stream: is_live,
            position: 0,
            title: title.to_string(),
            uri: Some(url.to_string()),
            artwork_url: thumbnail.map(|s| s.to_string()),
            isrc: None,
            source_name: "twitch".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::json!({
            "identifier": id,
            "source": "twitch",
            "uri": url,
            "title": title,
            "author": uploader,
            "duration": duration,
            "is_live": is_live
        });

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            track_data.to_string(),
        );

        Some(create_track(encoded, track_info))
    }
}

#[async_trait]
impl AudioSource for VimeoAudioSource {
    fn name(&self) -> &str {
        "vimeo"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("vimeo.com") || identifier.starts_with("vmsearch:")
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Handle Vimeo search queries
        if identifier.starts_with("vmsearch:") {
            return self
                .search(identifier.strip_prefix("vmsearch:").unwrap_or(identifier))
                .await;
        }

        // Direct URL - validate and normalize
        if !self.is_valid_vimeo_url(identifier) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid Vimeo URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL is not a valid Vimeo URL".to_string(),
                })),
            });
        }

        // Use yt-dlp to extract video information
        match self.extract_video_info(identifier).await {
            Ok(video_info) => {
                if let Some(track) = video_info.into_iter().next() {
                    Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(track))),
                    })
                } else {
                    Ok(LoadResult {
                        load_type: LoadType::Error,
                        data: Some(LoadResultData::Exception(Exception {
                            message: Some("Failed to extract video information".to_string()),
                            severity: Severity::Common,
                            cause: "Video not found or unavailable".to_string(),
                        })),
                    })
                }
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(format!("Vimeo extraction failed: {e}")),
                    severity: Severity::Common,
                    cause: "yt-dlp error".to_string(),
                })),
            }),
        }
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // Note: Vimeo doesn't have a direct search API like YouTube
        // For now, return an informative error message
        // In a real implementation, you might want to:
        // 1. Use Vimeo's API if available
        // 2. Implement web scraping (with proper rate limiting)
        // 3. Return a helpful error message suggesting direct URLs

        Ok(LoadResult {
            load_type: LoadType::Error,
            data: Some(LoadResultData::Exception(Exception {
                message: Some(format!("Vimeo search is not currently supported. Please use direct Vimeo URLs instead. Search query was: '{query}'")),
                severity: Severity::Common,
                cause: "Vimeo search not implemented".to_string(),
            }))
        })
    }
}

impl VimeoAudioSource {
    /// Check if a URL is a valid Vimeo URL
    fn is_valid_vimeo_url(&self, url: &str) -> bool {
        let vimeo_patterns = [
            r"^https?://(www\.)?vimeo\.com/\d+",
            r"^https?://player\.vimeo\.com/video/\d+",
            r"^https?://vimeo\.com/channels/[\w-]+/\d+",
            r"^https?://vimeo\.com/groups/[\w-]+/videos/\d+",
        ];

        vimeo_patterns.iter().any(|pattern| {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(url))
                .unwrap_or(false)
        })
    }

    /// Extract video information using yt-dlp
    async fn extract_video_info(&self, identifier: &str) -> Result<Vec<crate::protocol::Track>> {
        // Check if yt-dlp is available
        if !self.is_ytdlp_available().await {
            return Err(anyhow::anyhow!(
                "yt-dlp is not installed or not available in PATH"
            ));
        }

        // Build yt-dlp command with full path
        let yt_dlp_path = "/Users/henri89/Library/Python/3.9/bin/yt-dlp";
        let mut cmd = AsyncCommand::new(yt_dlp_path);
        cmd.args([
            "--dump-json",
            "--no-playlist",
            "--flat-playlist",
            "--no-warnings",
            "--ignore-errors",
            identifier,
        ]);

        // Execute command
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(track) = self.json_to_track(json) {
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse JSON line: {line} - Error: {e}");
                }
            }
        }

        Ok(tracks)
    }

    /// Check if yt-dlp is available in the system
    async fn is_ytdlp_available(&self) -> bool {
        let yt_dlp_path = "/Users/henri89/Library/Python/3.9/bin/yt-dlp";
        AsyncCommand::new(yt_dlp_path)
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Convert yt-dlp JSON output to Track
    fn json_to_track(&self, json: serde_json::Value) -> Option<crate::protocol::Track> {
        let id = json.get("id")?.as_str()?;
        let title = json.get("title")?.as_str().unwrap_or("Unknown Title");
        let uploader = json.get("uploader")?.as_str().unwrap_or("Unknown");
        let duration = json.get("duration")?.as_f64().unwrap_or(0.0) as u64 * 1000;
        let url = json
            .get("webpage_url")?
            .as_str()
            .or_else(|| json.get("url")?.as_str())?;

        // Get thumbnail from thumbnails array if available
        let thumbnail = json
            .get("thumbnails")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.last())
            .and_then(|thumb| thumb.get("url"))
            .and_then(|url| url.as_str())
            .or_else(|| json.get("thumbnail")?.as_str());

        let track_info = crate::protocol::TrackInfo {
            identifier: id.to_string(),
            is_seekable: true,
            author: uploader.to_string(),
            length: duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri: Some(url.to_string()),
            artwork_url: thumbnail.map(|s| s.to_string()),
            isrc: None,
            source_name: "vimeo".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::json!({
            "identifier": id,
            "source": "vimeo",
            "uri": url,
            "title": title,
            "author": uploader,
            "duration": duration
        });

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            track_data.to_string(),
        );

        Some(create_track(encoded, track_info))
    }
}

#[async_trait]
impl AudioSource for NicoAudioSource {
    fn name(&self) -> &str {
        "nico"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("nicovideo.jp")
    }

    async fn load_track(&self, _identifier: &str) -> Result<LoadResult> {
        Ok(LoadResult {
            load_type: LoadType::Error,
            data: Some(LoadResultData::Exception(Exception {
                message: Some("Nico audio source not yet implemented".to_string()),
                severity: Severity::Common,
                cause: "Not implemented".to_string(),
            })),
        })
    }

    async fn search(&self, _query: &str) -> Result<LoadResult> {
        Ok(LoadResult {
            load_type: LoadType::Empty,
            data: None,
        })
    }
}

#[async_trait]
impl AudioSource for LocalAudioSource {
    fn name(&self) -> &str {
        "local"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        // Handle file:// URLs and local file paths
        if identifier.starts_with("file://") {
            let path = identifier.strip_prefix("file://").unwrap_or(identifier);
            return std::path::Path::new(path).exists();
        }

        // Handle direct file paths (no protocol)
        if !identifier.contains("://") {
            return std::path::Path::new(identifier).exists();
        }

        false
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Normalize the path
        let file_path = if identifier.starts_with("file://") {
            identifier.strip_prefix("file://").unwrap_or(identifier)
        } else {
            identifier
        };

        let path = std::path::Path::new(file_path);

        // Check if file exists
        if !path.exists() {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("File not found".to_string()),
                    severity: Severity::Common,
                    cause: "File does not exist".to_string(),
                })),
            });
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Path is not a file".to_string()),
                    severity: Severity::Common,
                    cause: "Path points to a directory or special file".to_string(),
                })),
            });
        }

        // Extract metadata from file
        let _file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown File");

        let title = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Unknown Title");

        // Get file size for duration estimation (rough approximation)
        let file_size = std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        // Rough duration estimation (assuming average bitrate of 128kbps)
        // This is very approximate - real implementation would use audio metadata libraries
        let estimated_duration = if file_size > 0 {
            (file_size * 8) / (128 * 1000) * 1000 // Convert to milliseconds
        } else {
            0
        };

        let track_info = crate::protocol::TrackInfo {
            identifier: file_path.to_string(),
            is_seekable: true,
            author: "Local File".to_string(),
            length: estimated_duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri: Some(format!("file://{file_path}")),
            artwork_url: None,
            isrc: None,
            source_name: "local".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::json!({
            "identifier": file_path,
            "source": "local",
            "uri": format!("file://{}", file_path),
            "title": title,
            "author": "Local File",
            "duration": estimated_duration
        });

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            track_data.to_string(),
        );

        let track = create_track(encoded, track_info);

        Ok(LoadResult {
            load_type: LoadType::Track,
            data: Some(LoadResultData::Track(Box::new(track))),
        })
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // Local search could scan directories for matching files
        // For now, return empty as this is complex to implement properly
        let _ = query; // Suppress unused parameter warning
        Ok(LoadResult {
            load_type: LoadType::Empty,
            data: None,
        })
    }
}

impl Default for AudioSourceManager {
    fn default() -> Self {
        Self::new()
    }
}

// Implement AudioSource trait for AudioSourceManager
#[async_trait]
impl AudioSource for AudioSourceManager {
    fn name(&self) -> &str {
        "manager"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        // Manager can handle anything that any of its sources can handle
        self.sources
            .iter()
            .any(|source| source.can_handle(identifier))
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Try each source in order until one can handle the identifier
        for source in &self.sources {
            if source.can_handle(identifier) {
                match source.load_track(identifier).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        debug!(
                            "Source {} failed to load {}: {}",
                            source.name(),
                            identifier,
                            e
                        );
                        continue;
                    }
                }
            }
        }

        // If no source could handle it, return an error
        Err(anyhow::anyhow!(
            "No audio source could handle identifier: {}",
            identifier
        ))
    }

    async fn search(&self, query: &str) -> Result<LoadResult> {
        // For search, try YouTube first as it's most comprehensive
        for source in &self.sources {
            if source.name() == "youtube" {
                match source.search(query).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        debug!("YouTube search failed for {}: {}", query, e);
                    }
                }
            }
        }

        // If YouTube failed, try other sources
        for source in &self.sources {
            if source.name() != "youtube" {
                match source.search(query).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        debug!(
                            "Source {} search failed for {}: {}",
                            source.name(),
                            query,
                            e
                        );
                        continue;
                    }
                }
            }
        }

        // If no source could search, return empty result
        Ok(LoadResult {
            load_type: LoadType::Empty,
            data: None,
        })
    }
}

impl AudioSourceManager {}

/// Extract a title from a URL
fn extract_title_from_url(url: &str) -> String {
    // Try to extract filename from URL
    if let Some(path) = url.split('/').next_back() {
        if !path.is_empty() && path.contains('.') {
            // Remove query parameters and fragments
            let clean_path = path.split('?').next().unwrap_or(path);
            let clean_path = clean_path.split('#').next().unwrap_or(clean_path);
            return clean_path.to_string();
        }
    }

    // Fallback to domain name
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return format!("Audio from {host}");
        }
    }

    "Unknown Audio".to_string()
}

// Implementation for FallbackAudioSource - handles unsupported platforms
#[async_trait]
impl AudioSource for FallbackAudioSource {
    fn name(&self) -> &str {
        "fallback"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        // Handle Spotify, Apple Music, Deezer URLs
        self.is_spotify_url(identifier)
            || self.is_apple_music_url(identifier)
            || self.is_deezer_url(identifier)
    }

    async fn load_track(&self, identifier: &str) -> Result<LoadResult> {
        // Extract track information and convert to YouTube search
        if let Some(search_query) = self.extract_track_info(identifier).await {
            // Use YouTube source to search for the track
            let youtube_source = YouTubeAudioSource;
            let search_result = youtube_source.search(&search_query).await?;

            // If we found results, return the first one as a single track
            if let LoadResult {
                load_type: LoadType::Search,
                data: Some(LoadResultData::Search(tracks)),
            } = search_result
            {
                if let Some(first_track) = tracks.into_iter().next() {
                    return Ok(LoadResult {
                        load_type: LoadType::Track,
                        data: Some(LoadResultData::Track(Box::new(first_track))),
                    });
                }
            }

            // If no results found, return empty
            Ok(LoadResult {
                load_type: LoadType::Empty,
                data: None,
            })
        } else {
            // Could not extract track info
            Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some(
                        "Could not extract track information from unsupported URL".to_string(),
                    ),
                    severity: Severity::Common,
                    cause: "Unsupported platform URL format".to_string(),
                })),
            })
        }
    }

    async fn search(&self, _query: &str) -> Result<LoadResult> {
        // Fallback source doesn't support direct search
        Ok(LoadResult {
            load_type: LoadType::Empty,
            data: None,
        })
    }
}

impl FallbackAudioSource {
    /// Check if URL is a Spotify URL
    fn is_spotify_url(&self, url: &str) -> bool {
        url.contains("spotify.com") || url.starts_with("spotify:")
    }

    /// Check if URL is an Apple Music URL
    fn is_apple_music_url(&self, url: &str) -> bool {
        url.contains("music.apple.com")
    }

    /// Check if URL is a Deezer URL
    fn is_deezer_url(&self, url: &str) -> bool {
        url.contains("deezer.com")
    }

    /// Extract track information and create a search query
    async fn extract_track_info(&self, identifier: &str) -> Option<String> {
        if self.is_spotify_url(identifier) {
            self.extract_spotify_info(identifier).await
        } else if self.is_apple_music_url(identifier) {
            self.extract_apple_music_info(identifier).await
        } else if self.is_deezer_url(identifier) {
            self.extract_deezer_info(identifier).await
        } else {
            None
        }
    }

    /// Extract Spotify track information
    async fn extract_spotify_info(&self, url: &str) -> Option<String> {
        // For now, we'll use a simple approach - extract track ID and use Spotify Web API
        // In a production environment, you'd want to use the Spotify Web API

        // Extract track ID from URL
        if let Some(track_id) = self.extract_spotify_track_id(url) {
            // For demo purposes, we'll create a generic search query
            // In production, you'd call Spotify API to get track details
            tracing::info!("Converting Spotify track {} to YouTube search", track_id);
            Some(format!("spotify track {track_id}"))
        } else {
            None
        }
    }

    /// Extract Apple Music track information
    async fn extract_apple_music_info(&self, url: &str) -> Option<String> {
        // Extract track information from Apple Music URL
        // This is a simplified implementation
        if url.contains("music.apple.com") {
            tracing::info!("Converting Apple Music URL to YouTube search");
            Some("apple music track".to_string())
        } else {
            None
        }
    }

    /// Extract Deezer track information
    async fn extract_deezer_info(&self, url: &str) -> Option<String> {
        // Extract track information from Deezer URL
        // This is a simplified implementation
        if url.contains("deezer.com") {
            tracing::info!("Converting Deezer URL to YouTube search");
            Some("deezer track".to_string())
        } else {
            None
        }
    }

    /// Extract Spotify track ID from URL
    fn extract_spotify_track_id(&self, url: &str) -> Option<String> {
        // Handle different Spotify URL formats
        if url.starts_with("spotify:track:") {
            // spotify:track:4iV5W9uYEdYUVa79Axb7Rh
            url.strip_prefix("spotify:track:").map(|s| s.to_string())
        } else if url.contains("spotify.com/track/") {
            // https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh
            url.split("/track/")
                .nth(1)?
                .split('?')
                .next()
                .map(|s| s.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;

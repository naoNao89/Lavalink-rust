//! Enhanced HTTP audio source for Lavalink-rust
//!
//! This module provides an enhanced HTTP audio source that can detect
//! audio content types, extract metadata, and validate streams.

use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::{Client, Response};
use std::time::Duration;
use tracing::{debug, warn};
use url::Url;

use crate::protocol::{
    Exception, LoadResult, LoadResultData, LoadType, Severity, Track, TrackInfo,
};

/// Enhanced HTTP audio source with content detection and metadata extraction
pub struct EnhancedHttpSource {
    client: Client,
}

impl EnhancedHttpSource {
    /// Create a new enhanced HTTP source
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Lavalink-rust/4.0.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Load a track from an HTTP URL with enhanced validation and metadata extraction
    pub async fn load_track(&self, url: &str) -> Result<LoadResult> {
        debug!("Loading HTTP track: {}", url);

        // Validate URL format
        if !self.is_valid_http_url(url) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Invalid HTTP URL".to_string()),
                    severity: Severity::Common,
                    cause: "URL must start with http:// or https://".to_string(),
                })),
            });
        }

        // Check if URL should be handled by other sources
        if self.should_be_handled_by_other_source(url) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("URL should be handled by a specific source".to_string()),
                    severity: Severity::Common,
                    cause: "This URL belongs to a platform with a dedicated source".to_string(),
                })),
            });
        }

        // Perform HEAD request to check content type and availability
        match self.validate_audio_url(url).await {
            Ok(metadata) => {
                let track = self.create_track_from_metadata(url, metadata).await?;
                Ok(LoadResult {
                    load_type: LoadType::Track,
                    data: Some(LoadResultData::Track(Box::new(track))),
                })
            }
            Err(e) => Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Failed to validate HTTP audio URL".to_string()),
                    severity: Severity::Common,
                    cause: e.to_string(),
                })),
            }),
        }
    }

    /// Validate if URL is a valid HTTP/HTTPS URL
    fn is_valid_http_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Check if URL should be handled by other sources
    fn should_be_handled_by_other_source(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();

        // List of domains that should be handled by specific sources
        let specific_domains = [
            "youtube.com",
            "youtu.be",
            "soundcloud.com",
            "snd.sc",
            "bandcamp.com",
            "twitch.tv",
            "vimeo.com",
            "nicovideo.jp",
            "spotify.com",
            "music.apple.com",
            "deezer.com",
        ];

        specific_domains
            .iter()
            .any(|domain| url_lower.contains(domain))
    }

    /// Validate audio URL and extract metadata
    async fn validate_audio_url(&self, url: &str) -> Result<AudioMetadata> {
        // Perform HEAD request first to check headers
        let head_response = self.client.head(url).send().await?;

        if !head_response.status().is_success() {
            return Err(anyhow!("HTTP request failed: {}", head_response.status()));
        }

        let content_type = head_response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        let content_length = head_response
            .headers()
            .get("content-length")
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<u64>().ok());

        // Check if content type indicates audio
        if !self.is_audio_content_type(&content_type) {
            // If content type is not clearly audio, try a partial GET request
            // to check if it's actually audio content
            if !self.probe_audio_content(url).await? {
                return Err(anyhow!("URL does not point to audio content"));
            }
        }

        // Extract filename from URL for title
        let filename = self.extract_filename_from_url(url);
        let title = self.clean_filename_for_title(&filename);

        Ok(AudioMetadata {
            title,
            content_type: content_type.clone(),
            content_length,
            is_stream: self.is_likely_stream(&content_type, content_length),
        })
    }

    /// Check if content type indicates audio
    fn is_audio_content_type(&self, content_type: &str) -> bool {
        let audio_types = [
            "audio/",
            "application/ogg",
            "application/x-mpegurl",         // M3U8 playlists
            "application/vnd.apple.mpegurl", // HLS playlists
        ];

        audio_types
            .iter()
            .any(|audio_type| content_type.starts_with(audio_type))
    }

    /// Probe content by downloading a small portion to check if it's audio
    async fn probe_audio_content(&self, url: &str) -> Result<bool> {
        // Download first 1KB to check for audio signatures
        let response = self
            .client
            .get(url)
            .header("Range", "bytes=0-1023")
            .send()
            .await?;

        if response.status().is_success() || response.status().as_u16() == 206 {
            let bytes = response.bytes().await?;
            return Ok(self.has_audio_signature(&bytes));
        }

        // If range request failed, try without range
        let response = self.client.get(url).send().await?;
        if response.status().is_success() {
            let bytes = response.bytes().await?;
            return Ok(self.has_audio_signature(&bytes[..bytes.len().min(1024)]));
        }

        Ok(false)
    }

    /// Check if bytes have audio file signatures
    fn has_audio_signature(&self, bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }

        // Check for common audio file signatures
        match &bytes[0..4] {
            [0x49, 0x44, 0x33, _] => true,    // ID3 (MP3)
            [0xFF, 0xFB, _, _] => true,       // MP3
            [0xFF, 0xF3, _, _] => true,       // MP3
            [0xFF, 0xF2, _, _] => true,       // MP3
            [0x4F, 0x67, 0x67, 0x53] => true, // OGG
            [0x52, 0x49, 0x46, 0x46] => {
                // RIFF (WAV)
                if bytes.len() >= 12 {
                    &bytes[8..12] == b"WAVE"
                } else {
                    false
                }
            }
            [0x66, 0x4C, 0x61, 0x43] => true, // FLAC
            _ => false,
        }
    }

    /// Extract filename from URL
    fn extract_filename_from_url(&self, url: &str) -> String {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(path_segments) = parsed_url.path_segments() {
                if let Some(filename) = path_segments.last() {
                    if !filename.is_empty() {
                        return filename.to_string();
                    }
                }
            }
        }

        "Unknown Track".to_string()
    }

    /// Clean filename to create a readable title
    fn clean_filename_for_title(&self, filename: &str) -> String {
        // Remove file extension
        let title = if let Some(dot_pos) = filename.rfind('.') {
            &filename[..dot_pos]
        } else {
            filename
        };

        // Replace underscores and hyphens with spaces
        let title = title.replace('_', " ").replace('-', " ");

        // Capitalize first letter of each word
        title
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Determine if this is likely a live stream
    fn is_likely_stream(&self, content_type: &str, content_length: Option<u64>) -> bool {
        // No content length usually indicates a stream
        if content_length.is_none() {
            return true;
        }

        // Playlist formats are usually streams
        if content_type.contains("mpegurl") || content_type.contains("m3u") {
            return true;
        }

        false
    }

    /// Create a Lavalink track from metadata
    async fn create_track_from_metadata(
        &self,
        url: &str,
        metadata: AudioMetadata,
    ) -> Result<Track> {
        let track = Track {
            encoded: base64::engine::general_purpose::STANDARD.encode(url),
            info: TrackInfo {
                identifier: url.to_string(),
                seekable: !metadata.is_stream,
                author: "Unknown Artist".to_string(),
                length: 0, // Cannot determine length without downloading
                stream: metadata.is_stream,
                position: 0,
                title: metadata.title,
                uri: Some(url.to_string()),
                artwork_url: None,
                isrc: None,
                source_name: "http".to_string(),
            },
            plugin_info: None,
            user_data: None,
        };

        Ok(track)
    }
}

impl Default for EnhancedHttpSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio metadata extracted from HTTP headers and content
#[derive(Debug, Clone)]
struct AudioMetadata {
    title: String,
    content_type: String,
    content_length: Option<u64>,
    is_stream: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_url_validation() {
        let source = EnhancedHttpSource::new();

        assert!(source.is_valid_http_url("https://example.com/audio.mp3"));
        assert!(source.is_valid_http_url("http://example.com/stream"));
        assert!(!source.is_valid_http_url("ftp://example.com/file"));
        assert!(!source.is_valid_http_url("not-a-url"));
    }

    #[test]
    fn test_other_source_detection() {
        let source = EnhancedHttpSource::new();

        assert!(source.should_be_handled_by_other_source("https://soundcloud.com/artist/track"));
        assert!(source.should_be_handled_by_other_source("https://youtube.com/watch?v=123"));
        assert!(!source.should_be_handled_by_other_source("https://example.com/audio.mp3"));
    }

    #[test]
    fn test_filename_extraction() {
        let source = EnhancedHttpSource::new();

        assert_eq!(
            source.extract_filename_from_url("https://example.com/my_song.mp3"),
            "my_song.mp3"
        );
        assert_eq!(
            source.extract_filename_from_url("https://example.com/path/to/audio.wav"),
            "audio.wav"
        );
        assert_eq!(
            source.extract_filename_from_url("https://example.com/"),
            "Unknown Track"
        );
    }

    #[test]
    fn test_title_cleaning() {
        let source = EnhancedHttpSource::new();

        assert_eq!(
            source.clean_filename_for_title("my_awesome_song.mp3"),
            "My Awesome Song"
        );
        assert_eq!(
            source.clean_filename_for_title("track-01-intro.wav"),
            "Track 01 Intro"
        );
        assert_eq!(
            source.clean_filename_for_title("UPPERCASE_TRACK.flac"),
            "UPPERCASE TRACK"
        );
    }

    #[test]
    fn test_audio_signature_detection() {
        let source = EnhancedHttpSource::new();

        // MP3 signature
        assert!(source.has_audio_signature(&[0xFF, 0xFB, 0x90, 0x00]));

        // OGG signature
        assert!(source.has_audio_signature(&[0x4F, 0x67, 0x67, 0x53]));

        // Not audio
        assert!(!source.has_audio_signature(&[0x89, 0x50, 0x4E, 0x47])); // PNG
    }
}

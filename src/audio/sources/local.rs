//! Local file audio source for Lavalink-rust
//!
//! This module provides local file audio loading with metadata extraction
//! using Symphonia for comprehensive audio format support.

use anyhow::{anyhow, Result};
use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

#[cfg(feature = "audio-processing")]
use symphonia::core::formats::FormatOptions;
#[cfg(feature = "audio-processing")]
use symphonia::core::io::MediaSourceStream;
#[cfg(feature = "audio-processing")]
use symphonia::core::meta::MetadataOptions;
#[cfg(feature = "audio-processing")]
use symphonia::core::probe::Hint;

use crate::protocol::{
    Exception, LoadResult, LoadResultData, LoadType, Severity, Track, TrackInfo,
};

/// Local file audio source with metadata extraction
#[derive(Clone)]
pub struct LocalAudioSource {
    /// Allowed base directories for security
    allowed_directories: Vec<PathBuf>,
    /// Maximum search depth for directory traversal
    max_search_depth: usize,
    /// Supported audio file extensions
    supported_extensions: Vec<String>,
}

impl LocalAudioSource {
    /// Create a new local audio source with default configuration
    pub fn new() -> Self {
        Self {
            allowed_directories: vec![
                PathBuf::from("./music"),
                PathBuf::from("./audio"),
                PathBuf::from("./tracks"),
            ],
            max_search_depth: 3,
            supported_extensions: vec![
                "mp3".to_string(),
                "flac".to_string(),
                "wav".to_string(),
                "ogg".to_string(),
                "m4a".to_string(),
                "aac".to_string(),
                "opus".to_string(),
                "wma".to_string(),
            ],
        }
    }

    /// Create a new local audio source with custom configuration
    #[allow(dead_code)]
    pub fn with_config(
        allowed_directories: Vec<PathBuf>,
        max_search_depth: usize,
        supported_extensions: Vec<String>,
    ) -> Self {
        Self {
            allowed_directories,
            max_search_depth,
            supported_extensions,
        }
    }

    /// Load a track from a local file path
    pub async fn load_track(&self, path: &str) -> Result<LoadResult> {
        debug!("Loading local track: {}", path);

        // Normalize the path
        let file_path = self.normalize_path(path)?;

        // Check if file exists first (before security check)
        if !file_path.exists() {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("File not found".to_string()),
                    severity: Severity::Common,
                    cause: format!("File does not exist: {}", file_path.display()),
                })),
            });
        }

        // Security check: ensure path is within allowed directories
        if !self.is_path_allowed(&file_path) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Access denied".to_string()),
                    severity: Severity::Common,
                    cause: "File path is not within allowed directories".to_string(),
                })),
            });
        }

        // Check if it's a supported audio file
        if !self.is_supported_audio_file(&file_path) {
            return Ok(LoadResult {
                load_type: LoadType::Error,
                data: Some(LoadResultData::Exception(Exception {
                    message: Some("Unsupported file format".to_string()),
                    severity: Severity::Common,
                    cause: "File extension is not supported".to_string(),
                })),
            });
        }

        // Extract metadata and create track
        match self.extract_metadata(&file_path).await {
            Ok(track) => Ok(LoadResult {
                load_type: LoadType::Track,
                data: Some(LoadResultData::Track(Box::new(track))),
            }),
            Err(e) => {
                warn!(
                    "Failed to extract metadata from {}: {}",
                    file_path.display(),
                    e
                );
                Ok(LoadResult {
                    load_type: LoadType::Error,
                    data: Some(LoadResultData::Exception(Exception {
                        message: Some("Failed to load audio file".to_string()),
                        severity: Severity::Common,
                        cause: format!("Metadata extraction failed: {e}"),
                    })),
                })
            }
        }
    }

    /// Search for audio files in allowed directories
    pub async fn search_tracks(&self, query: &str, limit: Option<u32>) -> Result<Vec<Track>> {
        debug!("Searching local files for: {}", query);

        let mut tracks = Vec::new();
        let limit = limit.unwrap_or(50).min(200) as usize;
        let query_lower = query.to_lowercase();

        for base_dir in &self.allowed_directories {
            if !base_dir.exists() {
                continue;
            }

            if let Ok(found_tracks) = self
                .search_directory(base_dir, &query_lower, limit - tracks.len())
                .await
            {
                tracks.extend(found_tracks);
                if tracks.len() >= limit {
                    break;
                }
            }
        }

        tracks.truncate(limit);
        Ok(tracks)
    }

    /// Normalize file path (handle file:// URLs and relative paths)
    fn normalize_path(&self, path: &str) -> Result<PathBuf> {
        let clean_path = if path.starts_with("file://") {
            path.strip_prefix("file://").unwrap_or(path)
        } else {
            path
        };

        // Convert to absolute path and canonicalize
        let path_buf = PathBuf::from(clean_path);
        if path_buf.is_absolute() {
            Ok(path_buf)
        } else {
            // For relative paths, try to resolve against current directory
            std::env::current_dir()
                .map(|cwd| cwd.join(path_buf))
                .map_err(|e| anyhow!("Failed to resolve relative path: {e}"))
        }
    }

    /// Check if a path is within allowed directories
    fn is_path_allowed(&self, path: &Path) -> bool {
        // If no allowed directories are configured, allow all paths
        if self.allowed_directories.is_empty() {
            return true;
        }

        // Check if the path is within any allowed directory
        for allowed_dir in &self.allowed_directories {
            // Try to canonicalize both paths, but fall back to string comparison if canonicalization fails
            match (path.canonicalize(), allowed_dir.canonicalize()) {
                (Ok(canonical_path), Ok(canonical_allowed)) => {
                    if canonical_path.starts_with(canonical_allowed) {
                        return true;
                    }
                }
                _ => {
                    // Fallback: use string-based path comparison for non-existent paths
                    if let (Some(path_str), Some(allowed_str)) =
                        (path.to_str(), allowed_dir.to_str())
                    {
                        if path_str.starts_with(allowed_str) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if a file has a supported audio extension
    fn is_supported_audio_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.supported_extensions.contains(&ext_str.to_lowercase());
            }
        }
        false
    }

    /// Extract metadata from an audio file using Symphonia
    #[cfg(feature = "audio-processing")]
    async fn extract_metadata(&self, path: &Path) -> Result<Track> {
        // Open the file
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint based on file extension
        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        // Probe the file to determine format
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow!("No audio track found"))?;

        // Extract basic information
        let duration_ms = if let Some(n_frames) = track.codec_params.n_frames {
            if let Some(sample_rate) = track.codec_params.sample_rate {
                n_frames * 1000 / sample_rate as u64
            } else {
                0
            }
        } else {
            0
        };

        // Extract metadata
        let metadata = format.metadata();
        let mut title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let mut artist = "Unknown Artist".to_string();

        // Try to get metadata from the file
        if let Some(metadata_rev) = metadata.current() {
            for tag in metadata_rev.tags() {
                match tag.key.as_str() {
                    "TITLE" | "TIT2" => title = tag.value.to_string(),
                    "ARTIST" | "TPE1" => artist = tag.value.to_string(),
                    _ => {}
                }
            }
        }

        // Create file URL
        let file_url = format!("file://{}", path.display());

        // Create track info
        let track_info = TrackInfo {
            identifier: path.to_string_lossy().to_string(),
            is_seekable: true,
            author: artist,
            length: duration_ms,
            is_stream: false,
            position: 0,
            title,
            uri: Some(file_url.clone()),
            artwork_url: None,
            isrc: None,
            source_name: "local".to_string(),
        };

        // Create encoded track data
        let track_data = serde_json::json!({
            "identifier": path.to_string_lossy(),
            "source": "local",
            "uri": file_url,
            "title": track_info.title,
            "author": track_info.author,
            "duration": duration_ms
        });

        let encoded = base64::engine::general_purpose::STANDARD.encode(track_data.to_string());

        Ok(Track {
            encoded,
            info: track_info,
            #[cfg(feature = "plugins")]
            plugin_info: HashMap::new(),
            #[cfg(feature = "rest-api")]
            user_data: HashMap::new(),
        })
    }

    /// Fallback metadata extraction when audio-processing feature is disabled
    #[cfg(not(feature = "audio-processing"))]
    async fn extract_metadata(&self, path: &Path) -> Result<Track> {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let file_url = format!("file://{}", path.display());

        let track_info = TrackInfo {
            identifier: path.to_string_lossy().to_string(),
            is_seekable: true,
            author: "Unknown Artist".to_string(),
            length: 0, // Cannot determine without audio processing
            is_stream: false,
            position: 0,
            title,
            uri: Some(file_url.clone()),
            artwork_url: None,
            isrc: None,
            source_name: "local".to_string(),
        };

        let track_data = serde_json::json!({
            "identifier": path.to_string_lossy(),
            "source": "local",
            "uri": file_url,
            "title": track_info.title,
            "author": track_info.author,
            "duration": 0
        });

        let encoded = base64::engine::general_purpose::STANDARD.encode(track_data.to_string());

        Ok(Track {
            encoded,
            info: track_info,
            plugin_info: HashMap::new(),
            user_data: HashMap::new(),
        })
    }

    /// Search for audio files in a directory recursively
    async fn search_directory(&self, dir: &Path, query: &str, limit: usize) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();

        if tracks.len() >= limit {
            return Ok(tracks);
        }

        // Use an iterative approach to avoid async recursion issues
        let mut dirs_to_search = vec![(dir.to_path_buf(), 0)];

        while let Some((current_dir, depth)) = dirs_to_search.pop() {
            if depth > self.max_search_depth || tracks.len() >= limit {
                continue;
            }

            let read_dir = match std::fs::read_dir(&current_dir) {
                Ok(rd) => rd,
                Err(e) => {
                    debug!("Failed to read directory {}: {}", current_dir.display(), e);
                    continue;
                }
            };

            for entry in read_dir {
                if tracks.len() >= limit {
                    break;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let path = entry.path();

                if path.is_dir() {
                    // Add subdirectory to search queue
                    dirs_to_search.push((path, depth + 1));
                } else if self.is_supported_audio_file(&path) {
                    // Check if filename matches query
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.to_lowercase().contains(query) {
                            // Try to extract metadata and add to results
                            if let Ok(track) = self.extract_metadata(&path).await {
                                tracks.push(track);
                            }
                        }
                    }
                }
            }
        }

        Ok(tracks)
    }

    /// Check if the local source can handle a given identifier
    pub fn can_handle(&self, identifier: &str) -> bool {
        // Handle file:// URLs
        if identifier.starts_with("file://") {
            let path = identifier.strip_prefix("file://").unwrap_or(identifier);
            return Path::new(path).exists();
        }

        // Handle direct file paths (no protocol)
        if !identifier.contains("://") {
            return Path::new(identifier).exists();
        }

        false
    }

    /// Get the name of this audio source
    pub fn name(&self) -> &str {
        "local"
    }
}

impl Default for LocalAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_source_creation() {
        let source = LocalAudioSource::new();
        assert_eq!(source.name(), "local");
        assert_eq!(source.max_search_depth, 3);
        assert!(source.supported_extensions.contains(&"mp3".to_string()));
    }

    #[tokio::test]
    async fn test_path_normalization() {
        let source = LocalAudioSource::new();

        // Test file:// URL normalization
        let result = source.normalize_path("file:///tmp/test.mp3");
        assert!(result.is_ok());

        // Test relative path normalization
        let result = source.normalize_path("./test.mp3");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_supported_file_detection() {
        let source = LocalAudioSource::new();

        assert!(source.is_supported_audio_file(Path::new("test.mp3")));
        assert!(source.is_supported_audio_file(Path::new("test.flac")));
        assert!(source.is_supported_audio_file(Path::new("test.wav")));
        assert!(!source.is_supported_audio_file(Path::new("test.txt")));
        assert!(!source.is_supported_audio_file(Path::new("test.jpg")));
    }

    #[tokio::test]
    async fn test_can_handle() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.mp3");
        fs::write(&test_file, b"fake mp3 content").unwrap();

        let source = LocalAudioSource::new();

        // Should handle existing files
        assert!(source.can_handle(&test_file.to_string_lossy()));
        assert!(source.can_handle(&format!("file://{}", test_file.display())));

        // Should not handle non-existent files
        assert!(!source.can_handle("/nonexistent/file.mp3"));
        assert!(!source.can_handle("https://example.com/test.mp3"));
    }

    #[tokio::test]
    async fn test_path_security() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed_dir).unwrap();

        let allowed_dirs = vec![allowed_dir.clone()];
        let source = LocalAudioSource::with_config(allowed_dirs, 3, vec!["mp3".to_string()]);

        // Should allow paths within allowed directory
        assert!(source.is_path_allowed(&allowed_dir.join("test.mp3")));

        // Should deny paths outside allowed directory
        let forbidden_dir = temp_dir.path().join("forbidden");
        assert!(!source.is_path_allowed(&forbidden_dir.join("test.mp3")));
        assert!(!source.is_path_allowed(Path::new("/etc/passwd")));
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let source = LocalAudioSource::new();
        let result = source.load_track("/nonexistent/file.mp3").await.unwrap();

        assert!(matches!(result.load_type, LoadType::Error));
        if let Some(LoadResultData::Exception(exception)) = result.data {
            assert!(exception
                .message
                .unwrap_or_default()
                .contains("File not found"));
        }
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let source = LocalAudioSource::new();
        let tracks = source.search_tracks("", Some(10)).await.unwrap();

        // Should return empty results for empty query in non-existent directories
        assert!(tracks.is_empty());
    }
}

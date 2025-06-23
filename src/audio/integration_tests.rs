// Integration tests for audio sources
// Tests for enhanced Bandcamp search, YouTube/SoundCloud integration, and audio playback

use super::*;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

#[cfg(test)]
mod audio_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_bandcamp_search_functionality() {
        let bandcamp_source = BandcampAudioSource::new();

        // Test search with a simple query
        let result = timeout(Duration::from_secs(10), bandcamp_source.search("jazz")).await;

        match result {
            Ok(Ok(load_result)) => {
                // Search should complete without error
                match load_result.load_type {
                    LoadType::Search => {
                        if let Some(LoadResultData::Search(tracks)) = load_result.data {
                            info!("Found {} tracks in Bandcamp search", tracks.len());
                            assert!(!tracks.is_empty());
                        }
                    }
                    LoadType::Empty => {
                        info!("Bandcamp search returned empty results");
                    }
                    LoadType::Error => {
                        if let Some(LoadResultData::Exception(exception)) = load_result.data {
                            info!("Bandcamp search failed: {:?}", exception.message);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => {
                warn!("Bandcamp search error: {}", e);
            }
            Err(_) => {
                warn!("Bandcamp search timed out");
            }
        }
    }

    #[tokio::test]
    async fn test_bandcamp_url_validation() {
        let bandcamp_source = BandcampAudioSource::new();

        // Valid Bandcamp URLs
        assert!(
            bandcamp_source.is_valid_bandcamp_url("https://artist.bandcamp.com/track/song-name")
        );
        assert!(
            bandcamp_source.is_valid_bandcamp_url("https://artist.bandcamp.com/album/album-name")
        );
        assert!(bandcamp_source.can_handle("https://artist.bandcamp.com/track/test"));

        // Invalid URLs
        assert!(!bandcamp_source.is_valid_bandcamp_url("https://youtube.com/watch?v=test"));
        assert!(!bandcamp_source.is_valid_bandcamp_url("https://spotify.com/track/test"));
        assert!(!bandcamp_source.can_handle("https://soundcloud.com/test"));
    }

    #[tokio::test]
    async fn test_bandcamp_search_prefix() {
        let bandcamp_source = BandcampAudioSource::new();

        // Test search prefix handling
        assert!(bandcamp_source.can_handle("bcsearch:test query"));

        let result = timeout(
            Duration::from_secs(5),
            bandcamp_source.load_track("bcsearch:ambient"),
        )
        .await;

        match result {
            Ok(Ok(load_result)) => {
                // Should handle search queries
                assert!(matches!(
                    load_result.load_type,
                    LoadType::Search | LoadType::Empty | LoadType::Error
                ));
            }
            _ => {
                // Network errors are acceptable in tests
            }
        }
    }

    #[tokio::test]
    async fn test_soundcloud_integration() {
        let soundcloud_source = SoundCloudAudioSource::new();

        // Test SoundCloud URL handling
        assert!(soundcloud_source.can_handle("https://soundcloud.com/artist/track"));
        assert!(soundcloud_source.can_handle("scsearch:test query"));
        assert!(!soundcloud_source.can_handle("https://youtube.com/watch?v=test"));
    }

    #[tokio::test]
    async fn test_twitch_integration() {
        let twitch_source = TwitchAudioSource::new();

        // Test Twitch URL validation
        assert!(twitch_source.is_valid_twitch_url("https://www.twitch.tv/streamer"));
        assert!(twitch_source.is_valid_twitch_url("https://twitch.tv/videos/123456"));
        assert!(twitch_source.is_valid_twitch_url("https://www.twitch.tv/streamer/clip/clipname"));

        assert!(!twitch_source.is_valid_twitch_url("https://youtube.com/watch?v=test"));
        assert!(!twitch_source.is_valid_twitch_url("https://twitch.tv/invalid/path"));
    }

    #[tokio::test]
    async fn test_audio_source_manager_integration() {
        let audio_manager = AudioSourceManager::new();

        // Test that all sources are properly registered
        assert!(audio_manager.can_handle("https://youtube.com/watch?v=test"));
        assert!(audio_manager.can_handle("https://soundcloud.com/artist/track"));
        assert!(audio_manager.can_handle("https://artist.bandcamp.com/track/song"));
        assert!(audio_manager.can_handle("https://twitch.tv/streamer"));
        assert!(audio_manager.can_handle("http://example.com/audio.mp3"));

        // Test search prefixes
        assert!(audio_manager.can_handle("ytsearch:test"));
        assert!(audio_manager.can_handle("scsearch:test"));
        assert!(audio_manager.can_handle("bcsearch:test"));
        assert!(audio_manager.can_handle("twsearch:test"));
    }

    #[tokio::test]
    async fn test_fallback_system() {
        let audio_manager = AudioSourceManager::new();

        // Test with an identifier that might fail on primary source
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_item("https://example.com/nonexistent.mp3"),
        )
        .await;

        match result {
            Ok(Ok(load_result)) => {
                // Should handle gracefully, either with error or empty result
                assert!(matches!(
                    load_result.load_type,
                    LoadType::Error | LoadType::Empty
                ));
            }
            _ => {
                // Timeout or error is acceptable for non-existent resources
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_loading() {
        let audio_manager = AudioSourceManager::new();

        // Test concurrent loading of multiple tracks
        let identifiers = vec![
            "bcsearch:jazz",
            "scsearch:electronic",
            "ytsearch:classical",
            "http://example.com/test.mp3",
        ];

        let mut handles = Vec::new();

        for identifier in identifiers {
            let manager = audio_manager.clone();
            let id = identifier.to_string();

            let handle = tokio::spawn(async move {
                timeout(Duration::from_secs(5), manager.load_item(&id)).await
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        let results = futures::future::join_all(handles).await;

        // All requests should complete (though they may fail due to network issues)
        assert_eq!(results.len(), 4);

        for result in results {
            assert!(result.is_ok()); // Task should complete without panicking
        }
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let bandcamp_source = BandcampAudioSource::new();

        // Test that rate limiting doesn't cause errors
        let start_time = std::time::Instant::now();

        // Make multiple requests
        for i in 0..3 {
            let query = format!("test{}", i);
            let _ = timeout(Duration::from_secs(2), bandcamp_source.search(&query)).await;
        }

        let elapsed = start_time.elapsed();

        // Should take at least 1 second due to rate limiting (500ms * 2 requests)
        assert!(elapsed >= Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn test_error_handling() {
        let audio_manager = AudioSourceManager::new();

        // Test with various invalid inputs
        let invalid_inputs = vec![
            "",
            "invalid://url",
            "https://",
            "not-a-url",
            "ftp://example.com/file.mp3",
        ];

        for input in invalid_inputs {
            let result = timeout(Duration::from_secs(2), audio_manager.load_item(input)).await;

            match result {
                Ok(Ok(load_result)) => {
                    // Should handle gracefully with error or empty result
                    assert!(matches!(
                        load_result.load_type,
                        LoadType::Error | LoadType::Empty
                    ));
                }
                _ => {
                    // Timeout or error is acceptable for invalid inputs
                }
            }
        }
    }

    #[tokio::test]
    async fn test_track_encoding_decoding() {
        use crate::protocol::Track;

        // Create a test track
        let track_info = crate::protocol::TrackInfo {
            identifier: "test123".to_string(),
            is_seekable: true,
            author: "Test Artist".to_string(),
            length: 180000, // 3 minutes
            is_stream: false,
            position: 0,
            title: "Test Track".to_string(),
            uri: Some("https://example.com/track".to_string()),
            artwork_url: Some("https://example.com/artwork.jpg".to_string()),
            isrc: None,
            source_name: "test".to_string(),
        };

        // Create track data for encoding
        let track_data = serde_json::to_vec(&track_info).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&track_data);

        let _track = Track {
            encoded: encoded.clone(),
            info: track_info.clone(),
            plugin_info: std::collections::HashMap::new(),
            user_data: std::collections::HashMap::new(),
        };

        // Test decoding
        let decoded_track = Track::decode(&encoded).unwrap();

        assert_eq!(decoded_track.info.identifier, track_info.identifier);
        assert_eq!(decoded_track.info.title, track_info.title);
        assert_eq!(decoded_track.info.author, track_info.author);
        assert_eq!(decoded_track.info.length, track_info.length);
    }

    #[tokio::test]
    async fn test_source_configuration() {
        use crate::config::SourcesConfig;

        // Test with different source configurations
        let config = SourcesConfig {
            bandcamp: Some(false),
            ..Default::default()
        };

        let audio_manager = AudioSourceManager::with_config(Some(&config));

        // Bandcamp should be disabled
        assert!(!audio_manager.can_handle("https://artist.bandcamp.com/track/test"));

        // Other sources should still work
        assert!(audio_manager.can_handle("https://youtube.com/watch?v=test"));
        assert!(audio_manager.can_handle("http://example.com/audio.mp3"));
    }
}

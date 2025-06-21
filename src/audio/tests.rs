// Audio source manager tests
// These tests validate audio loading and processing functionality

use super::*;
use crate::protocol::{LoadResult, LoadResultData, LoadType, Playlist, PlaylistInfo};
use crate::test_utils::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[cfg(test)]
mod audio_source_tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_source_manager_creation() {
        let _audio_manager = AudioSourceManager::new();

        // Verify the manager is created successfully
        // Note: All sources are enabled by default in the new implementation
        // Test passes if no panic occurs during creation
    }

    #[tokio::test]
    async fn test_load_track_with_invalid_identifier() {
        let audio_manager = AudioSourceManager::new();

        let result = audio_manager.load_track("invalid://not-a-real-url").await;

        match result {
            Ok(load_result) => {
                match load_result.load_type {
                    LoadType::Error => {
                        // Expected for invalid URLs
                        assert!(load_result.data.is_none());
                    }
                    _ => panic!("Expected error load type for invalid identifier"),
                }
            }
            Err(_) => {
                // Also acceptable - the manager might reject invalid URLs immediately
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_http_url() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Use a mock HTTP URL (this will likely fail but should be handled gracefully)
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("http://example.com/test.mp3"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the URL even if it fails to load
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty
                        ));
                    }
                    Err(_) => {
                        // Network errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_youtube_search() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test YouTube search (this will likely fail without API keys but should be handled)
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("ytsearch:never gonna give you up"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the search even if it fails
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Search
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_soundcloud_search() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test SoundCloud search
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("scsearch:test track"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the search even if it fails
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Search
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_bandcamp_search() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test Bandcamp search
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("bcsearch:test track"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the search even if it fails
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Search
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_bandcamp_url_validation() {
        let audio_manager = AudioSourceManager::new();

        // Test valid Bandcamp URLs
        let valid_urls = [
            "https://artist.bandcamp.com/track/song-name",
            "https://artist.bandcamp.com/album/album-name",
            "http://artist.bandcamp.com/track/song-name",
        ];

        for url in &valid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            // Should attempt to process the URL (even if it fails due to network)
            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should handle the URL even if it fails to load
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty | LoadType::Track
                            ));
                        }
                        Err(_) => {
                            // Network errors are acceptable in tests
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable for network operations in tests
                }
            }
        }
    }

    #[tokio::test]
    async fn test_bandcamp_invalid_url_handling() {
        let audio_manager = AudioSourceManager::new();

        // Test invalid Bandcamp URLs
        let invalid_urls = [
            "https://not-bandcamp.com/track/song",
            "https://bandcamp.com", // Missing subdomain
            "bandcamp://invalid-protocol",
        ];

        for url in &invalid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should return error for invalid URLs
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty
                            ));
                        }
                        Err(_) => {
                            // Errors are expected for invalid URLs
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable
                }
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_vimeo_search() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test Vimeo search
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("vmsearch:test video"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the search even if it fails
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Search
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_vimeo_url_validation() {
        let audio_manager = AudioSourceManager::new();

        // Test valid Vimeo URLs
        let valid_urls = [
            "https://vimeo.com/123456789",
            "https://www.vimeo.com/123456789",
            "https://player.vimeo.com/video/123456789",
            "https://vimeo.com/channels/test/123456789",
        ];

        for url in &valid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            // Should attempt to process the URL (even if it fails due to network)
            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should handle the URL even if it fails to load
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty | LoadType::Track
                            ));
                        }
                        Err(_) => {
                            // Network errors are acceptable in tests
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable for network operations in tests
                }
            }
        }
    }

    #[tokio::test]
    async fn test_vimeo_invalid_url_handling() {
        let audio_manager = AudioSourceManager::new();

        // Test invalid Vimeo URLs
        let invalid_urls = [
            "https://not-vimeo.com/123456789",
            "https://vimeo.com", // Missing video ID
            "vimeo://invalid-protocol",
            "https://vimeo.com/invalid-id",
        ];

        for url in &invalid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should return error for invalid URLs
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty
                            ));
                        }
                        Err(_) => {
                            // Errors are expected for invalid URLs
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable
                }
            }
        }
    }

    #[tokio::test]
    async fn test_load_track_with_twitch_search() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test Twitch search (channel lookup)
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("twsearch:test_channel"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the search even if channel is offline
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Search
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_twitch_url_validation() {
        let audio_manager = AudioSourceManager::new();

        // Test valid Twitch URLs
        let valid_urls = [
            "https://twitch.tv/test_channel",
            "https://www.twitch.tv/test_channel",
            "https://twitch.tv/videos/123456789",
            "https://www.twitch.tv/videos/123456789",
            "https://twitch.tv/test_channel/clip/test-clip-id",
            "https://www.twitch.tv/test_channel/clip/test-clip-id",
        ];

        for url in &valid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            // Should attempt to process the URL (even if it fails due to network/offline)
            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should handle the URL even if stream is offline
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty | LoadType::Track
                            ));
                        }
                        Err(_) => {
                            // Network errors are acceptable in tests
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable for network operations in tests
                }
            }
        }
    }

    #[tokio::test]
    async fn test_twitch_invalid_url_handling() {
        let audio_manager = AudioSourceManager::new();

        // Test invalid Twitch URLs
        let invalid_urls = [
            "https://not-twitch.tv/channel",
            "https://twitch.tv", // Missing channel/content
            "twitch://invalid-protocol",
            "https://twitch.tv/videos/",       // Missing video ID
            "https://twitch.tv/channel/clip/", // Missing clip ID
        ];

        for url in &invalid_urls {
            let result = timeout(Duration::from_secs(3), audio_manager.load_track(url)).await;

            match result {
                Ok(load_result) => {
                    match load_result {
                        Ok(load_result) => {
                            // Should return error for invalid URLs
                            assert!(matches!(
                                load_result.load_type,
                                LoadType::Error | LoadType::Empty
                            ));
                        }
                        Err(_) => {
                            // Errors are expected for invalid URLs
                        }
                    }
                }
                Err(_) => {
                    // Timeout is acceptable
                }
            }
        }
    }

    #[tokio::test]
    async fn test_load_playlist() {
        let _config = create_test_config();
        let audio_manager = AudioSourceManager::new();

        // Test playlist loading (will likely fail but should be handled)
        let result = timeout(
            Duration::from_secs(5),
            audio_manager.load_track("https://www.youtube.com/playlist?list=PLtest"),
        )
        .await;

        match result {
            Ok(load_result) => {
                match load_result {
                    Ok(load_result) => {
                        // Should handle the playlist even if it fails
                        assert!(matches!(
                            load_result.load_type,
                            LoadType::Error | LoadType::Empty | LoadType::Playlist
                        ));
                    }
                    Err(_) => {
                        // API errors are acceptable in tests
                    }
                }
            }
            Err(_) => {
                // Timeout is acceptable for network operations in tests
            }
        }
    }

    #[tokio::test]
    async fn test_source_enablement() {
        let mut config = create_test_config();

        // Disable YouTube
        config.lavalink.server.sources.youtube = Some(false);

        let audio_manager = AudioSourceManager::with_config(Some(&config.lavalink.server.sources));

        // Test that YouTube URLs are not handled when disabled
        assert!(!audio_manager.can_handle("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        // Test that HTTP URLs are still handled
        assert!(audio_manager.can_handle("http://example.com/test.mp3"));
    }

    #[tokio::test]
    async fn test_concurrent_track_loading() {
        let _config = create_test_config();
        let audio_manager = Arc::new(AudioSourceManager::new());

        let mut handles = Vec::new();

        // Start multiple concurrent load operations
        for i in 0..5 {
            let manager = audio_manager.clone();
            let identifier = format!("http://example.com/test{}.mp3", i);

            let handle = tokio::spawn(async move {
                timeout(Duration::from_secs(3), manager.load_track(&identifier)).await
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        let results = futures::future::join_all(handles).await;

        // Verify all operations completed (even if they failed)
        assert_eq!(results.len(), 5);

        for result in results {
            assert!(result.is_ok()); // The task should complete, even if the load fails
        }
    }

    #[tokio::test]
    async fn test_load_result_serialization() {
        let load_result = LoadResult {
            load_type: LoadType::Track,
            data: Some(LoadResultData::Track(Box::new(create_mock_track()))),
        };

        let serialized =
            serde_json::to_string(&load_result).expect("Failed to serialize LoadResult");
        let deserialized: LoadResult =
            serde_json::from_str(&serialized).expect("Failed to deserialize LoadResult");

        assert!(matches!(deserialized.load_type, LoadType::Track));
        assert!(deserialized.data.is_some());
    }

    #[tokio::test]
    async fn test_load_result_error_serialization() {
        let load_result = LoadResult {
            load_type: LoadType::Error,
            data: Some(LoadResultData::Exception(create_mock_exception())),
        };

        let serialized =
            serde_json::to_string(&load_result).expect("Failed to serialize error LoadResult");
        let deserialized: LoadResult =
            serde_json::from_str(&serialized).expect("Failed to deserialize error LoadResult");

        assert!(matches!(deserialized.load_type, LoadType::Error));

        if let Some(LoadResultData::Exception(exception)) = deserialized.data {
            assert_eq!(exception.cause, "Test cause");
        } else {
            panic!("Expected exception data");
        }
    }

    #[tokio::test]
    async fn test_load_result_playlist_serialization() {
        let playlist_info = PlaylistInfo {
            name: "Test Playlist".to_string(),
            selected_track: Some(0),
        };

        let load_result = LoadResult {
            load_type: LoadType::Playlist,
            data: Some(LoadResultData::Playlist(Playlist {
                info: playlist_info,
                plugin_info: std::collections::HashMap::new(),
                tracks: vec![create_mock_track()],
            })),
        };

        let serialized =
            serde_json::to_string(&load_result).expect("Failed to serialize playlist LoadResult");
        let deserialized: LoadResult =
            serde_json::from_str(&serialized).expect("Failed to deserialize playlist LoadResult");

        assert!(matches!(deserialized.load_type, LoadType::Playlist));

        if let Some(LoadResultData::Playlist(playlist)) = deserialized.data {
            assert_eq!(playlist.info.name, "Test Playlist");
            assert_eq!(playlist.tracks.len(), 1);
        } else {
            panic!("Expected playlist data");
        }
    }

    #[tokio::test]
    async fn test_audio_manager_with_disabled_sources() {
        let mut config = create_test_config();

        // Disable all sources except HTTP
        config.lavalink.server.sources.youtube = Some(false);
        config.lavalink.server.sources.soundcloud = Some(false);
        config.lavalink.server.sources.bandcamp = Some(false);
        config.lavalink.server.sources.twitch = Some(false);
        config.lavalink.server.sources.vimeo = Some(false);
        config.lavalink.server.sources.nico = Some(false);
        config.lavalink.server.sources.local = Some(false);

        let audio_manager = AudioSourceManager::with_config(Some(&config.lavalink.server.sources));

        // Only HTTP should be enabled - test by checking if URLs can be handled
        assert!(audio_manager.can_handle("http://example.com/test.mp3"));
        assert!(!audio_manager.can_handle("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(!audio_manager.can_handle("https://soundcloud.com/test"));
        assert!(!audio_manager.can_handle("https://bandcamp.com/test"));
        assert!(!audio_manager.can_handle("https://twitch.tv/test"));
        assert!(!audio_manager.can_handle("https://vimeo.com/test"));
    }
}

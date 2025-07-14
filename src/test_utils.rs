// Test utilities and common test infrastructure
// This module provides shared testing utilities across the codebase

use crate::config::{
    FiltersConfig, LavalinkConfig, LavalinkInnerConfig, LavalinkServerConfig, PluginsConfig,
    ServerConfig, SourcesConfig,
};

/// Create a test configuration for use in tests
pub fn create_test_config() -> LavalinkConfig {
    LavalinkConfig {
        server: ServerConfig {
            port: 2333,
            address: "127.0.0.1".to_string(),
            http2: None,
        },
        lavalink: LavalinkServerConfig {
            server: LavalinkInnerConfig {
                password: "youshallnotpass".to_string(),
                sources: SourcesConfig {
                    youtube: Some(true),
                    bandcamp: Some(true),
                    soundcloud: Some(true),
                    twitch: Some(true),
                    vimeo: Some(true),
                    nico: Some(true),
                    http: Some(true),
                    local: Some(false),
                },
                filters: FiltersConfig {
                    volume: Some(true),
                    equalizer: Some(true),
                    karaoke: Some(true),
                    timescale: Some(true),
                    tremolo: Some(true),
                    vibrato: Some(true),
                    distortion: Some(true),
                    rotation: Some(true),
                    channel_mix: Some(true),
                    low_pass: Some(true),
                },
                buffer_duration_ms: Some(400),
                frame_buffer_duration_ms: Some(5000),
                opus_encoding_quality: Some(10),
                resampling_quality: None,
                track_stuck_threshold_ms: Some(10000),
                use_seek_ghosting: Some(true),
                youtube_playlist_load_limit: Some(6),
                player_update_interval: Some(5),
                youtube_search_enabled: Some(true),
                soundcloud_search_enabled: Some(true),
                gc_warnings: Some(true),
                ratelimit: None,
                youtube_config: None,
                http_config: None,
                timeouts: None,
                discord_bot_token: None,
            },
            plugins: Some(PluginsConfig::default()),
        },
        metrics: None,
        sentry: None,
        logging: None,
        #[cfg(feature = "plugins")]
        plugins: None,
    }
}

/// Test helper for JSON serialization/deserialization validation
#[cfg(feature = "rest-api")]
pub fn test_json_roundtrip<T>(json_str: &str) -> anyhow::Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    // Deserialize from JSON string
    let deserialized: T = serde_json::from_str(json_str)?;

    // Serialize back to JSON
    let serialized = serde_json::to_string(&deserialized)?;

    // Deserialize again to ensure consistency
    let deserialized_again: T = serde_json::from_str(&serialized)?;

    // Ensure both deserializations are equal
    assert_eq!(deserialized, deserialized_again);

    Ok(deserialized)
}

/// Mock track data for testing
pub fn create_mock_track() -> crate::protocol::Track {
    use std::collections::HashMap;

    crate::protocol::Track {
        encoded: "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==".to_string(),
        info: crate::protocol::TrackInfo {
            identifier: "dQw4w9WgXcQ".to_string(),
            is_seekable: true,
            author: "RickAstleyVEVO".to_string(),
            length: 212000,
            is_stream: false,
            position: 0,
            title: "Rick Astley - Never Gonna Give You Up".to_string(),
            uri: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
            artwork_url: None,
            isrc: None,
            source_name: "youtube".to_string(),
        },
        #[cfg(feature = "plugins")]
        plugin_info: HashMap::new(),
        #[cfg(feature = "rest-api")]
        user_data: HashMap::new(),
    }
}

/// Mock exception for testing
pub fn create_mock_exception() -> crate::protocol::Exception {
    crate::protocol::Exception {
        message: Some("Test exception".to_string()),
        severity: crate::protocol::Severity::Common,
        cause: "Test cause".to_string(),
    }
}

/// Test assertion macros
#[macro_export]
macro_rules! assert_json_eq {
    ($left:expr, $right:expr) => {
        let left_val: serde_json::Value =
            serde_json::from_str($left).expect("Failed to parse left JSON");
        let right_val: serde_json::Value =
            serde_json::from_str($right).expect("Failed to parse right JSON");
        assert_eq!(left_val, right_val);
    };
}

#[macro_export]
macro_rules! assert_contains_fields {
    ($json:expr, $($field:expr),+) => {
        let value: serde_json::Value = serde_json::from_str($json).expect("Failed to parse JSON");
        if let serde_json::Value::Object(obj) = value {
            $(
                assert!(obj.contains_key($field), "Missing field: {}", $field);
            )+
        } else {
            panic!("Expected JSON object");
        }
    };
}

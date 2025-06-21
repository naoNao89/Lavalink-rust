// Test utilities and common test infrastructure
// This module provides shared testing utilities across the codebase

use crate::{

    config::{
        FiltersConfig, LavalinkConfig, LavalinkInnerConfig, LavalinkServerConfig, PluginsConfig,
        ServerConfig, SourcesConfig,
    },
    player::PlayerManager,
    plugin::PluginManager,
    server::AppState,
};
use axum::http::HeaderValue;
use serde_json::Value;
use std::sync::Arc;

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
            },
            plugins: Some(PluginsConfig::default()),
        },
        metrics: None,
        sentry: None,
        logging: None,
        plugins: None,
    }
}

/// Create a test app state for integration tests
pub async fn create_test_app_state() -> Arc<AppState> {
    let config = create_test_config();
    let sessions = Arc::new(dashmap::DashMap::new());
    let stats_collector = Arc::new(crate::server::StatsCollector::new());

    let info = crate::protocol::Info {
        version: crate::protocol::Version {
            semver: env!("CARGO_PKG_VERSION").to_string(),
            major: 4,
            minor: 0,
            patch: 0,
            pre_release: None,
            build: None,
        },
        build_time: 0,
        git: crate::protocol::Git {
            branch: "test".to_string(),
            commit: "test".to_string(),
            commit_time: 0,
        },
        jvm: "N/A - Rust".to_string(),
        lavaplayer: "N/A - Native Rust".to_string(),
        source_managers: vec![
            "http".to_string(),
            "youtube".to_string(),
            "soundcloud".to_string(),
        ],
        filters: vec![
            "volume".to_string(),
            "equalizer".to_string(),
            "karaoke".to_string(),
        ],
        plugins: crate::protocol::Plugins { plugins: vec![] },
    };


    let player_manager = Arc::new(PlayerManager::new());

    let plugin_config = config.lavalink.plugins.clone().unwrap_or_default();
    let plugin_manager = PluginManager::with_config(plugin_config);
    let plugin_manager = Arc::new(std::sync::RwLock::new(plugin_manager));

    Arc::new(AppState {
        config,
        sessions,
        stats_collector,
        info,

        player_manager,
        plugin_manager,
    })
}

/// Create authorization header for tests
pub fn create_auth_header() -> HeaderValue {
    HeaderValue::from_static("youshallnotpass")
}

/// Create invalid authorization header for tests
pub fn create_invalid_auth_header() -> HeaderValue {
    HeaderValue::from_static("wrongpassword")
}

/// Test helper for JSON serialization/deserialization validation
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

/// Test helper for validating JSON structure
pub fn validate_json_structure(json_str: &str, expected_fields: &[&str]) -> anyhow::Result<Value> {
    let value: Value = serde_json::from_str(json_str)?;

    if let Value::Object(obj) = &value {
        for field in expected_fields {
            assert!(obj.contains_key(*field), "Missing field: {}", field);
        }
    } else {
        anyhow::bail!("Expected JSON object, got: {:?}", value);
    }

    Ok(value)
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
        plugin_info: HashMap::new(),
        user_data: HashMap::new(),
    }
}

/// Mock player state for testing
pub fn create_mock_player_state() -> crate::protocol::PlayerState {
    use chrono::Utc;
    crate::protocol::PlayerState {
        time: Utc::now(),
        position: 1000,
        connected: true,
        ping: 10,
    }
}

/// Mock voice state for testing
pub fn create_mock_voice_state() -> crate::protocol::VoiceState {
    crate::protocol::VoiceState {
        token: "test_token".to_string(),
        endpoint: "test_endpoint".to_string(),
        session_id: "test_session_id".to_string(),
    }
}

/// Mock filters for testing
pub fn create_mock_filters() -> crate::protocol::Filters {
    use crate::protocol::Omissible;
    crate::protocol::Filters {
        volume: Omissible::Present(1.0),
        equalizer: Omissible::Omitted,
        karaoke: Omissible::Present(None),
        timescale: Omissible::Present(None),
        tremolo: Omissible::Present(None),
        vibrato: Omissible::Present(None),
        rotation: Omissible::Present(None),
        distortion: Omissible::Present(None),
        channel_mix: Omissible::Present(None),
        low_pass: Omissible::Present(None),
        plugin_filters: std::collections::HashMap::new(),
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

/// Performance testing utilities
pub mod perf {
    use std::time::{Duration, Instant};

    /// Measure execution time of a function
    pub fn measure_time<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Run a function multiple times and return average duration
    pub fn benchmark<F>(f: F, iterations: usize) -> Duration
    where
        F: Fn(),
    {
        let mut total_duration = Duration::new(0, 0);

        for _ in 0..iterations {
            let start = Instant::now();
            f();
            total_duration += start.elapsed();
        }

        total_duration / iterations as u32
    }
}

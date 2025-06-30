use crate::config::LavalinkConfig;
use anyhow::{anyhow, Result};
use axum::http::HeaderMap;

/// Authenticate a request using the Authorization header
pub fn authenticate_request(headers: &HeaderMap, config: &LavalinkConfig) -> Result<()> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| anyhow!("Missing Authorization header"))?;

    let auth_value = auth_header
        .to_str()
        .map_err(|_| anyhow!("Invalid Authorization header"))?;

    let expected_password = &config.lavalink.server.password;

    if auth_value != expected_password {
        return Err(anyhow!("Invalid password"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        FiltersConfig, LavalinkConfig, LavalinkInnerConfig, LavalinkServerConfig, SourcesConfig,
    };
    use axum::http::HeaderValue;

    fn create_test_config() -> LavalinkConfig {
        LavalinkConfig {
            server: crate::config::ServerConfig {
                port: 2333,
                address: "0.0.0.0".to_string(),
                http2: None,
            },
            lavalink: LavalinkServerConfig {
                server: LavalinkInnerConfig {
                    password: "testpassword".to_string(),
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
                plugins: None,
            },
            metrics: None,
            sentry: None,
            logging: None,
            plugins: None,
        }
    }

    #[test]
    fn test_valid_authentication() {
        let config = create_test_config();
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("testpassword"));

        assert!(authenticate_request(&headers, &config).is_ok());
    }

    #[test]
    fn test_invalid_password() {
        let config = create_test_config();
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("wrongpassword"));

        assert!(authenticate_request(&headers, &config).is_err());
    }

    #[test]
    fn test_missing_header() {
        let config = create_test_config();
        let headers = HeaderMap::new();

        assert!(authenticate_request(&headers, &config).is_err());
    }
}

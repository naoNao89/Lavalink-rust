//! SoundCloud API integration for Lavalink-rust
//! 
//! This module provides a complete SoundCloud API client that implements
//! the official SoundCloud API v2 with OAuth 2.1 Client Credentials authentication.

use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use url::Url;

use crate::protocol::{Exception, LoadResult, LoadResultData, LoadType, Severity, Track};

/// SoundCloud API client configuration
#[derive(Debug, Clone)]
pub struct SoundCloudConfig {
    pub client_id: String,
    pub client_secret: String,
    pub api_base_url: String,
    pub auth_base_url: String,
}

impl Default for SoundCloudConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            api_base_url: "https://api.soundcloud.com".to_string(),
            auth_base_url: "https://secure.soundcloud.com".to_string(),
        }
    }
}

/// OAuth 2.1 token response from SoundCloud
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    scope: String,
    refresh_token: Option<String>,
}

/// SoundCloud track information from API
#[derive(Debug, Deserialize)]
struct SoundCloudTrack {
    id: u64,
    title: String,
    description: Option<String>,
    duration: u64, // in milliseconds
    permalink_url: String,
    stream_url: Option<String>,
    download_url: Option<String>,
    artwork_url: Option<String>,
    user: SoundCloudUser,
    genre: Option<String>,
    tag_list: Option<String>,
    playback_count: Option<u64>,
    access: Option<String>, // "playable", "preview", "blocked"
}

/// SoundCloud user information
#[derive(Debug, Deserialize)]
struct SoundCloudUser {
    id: u64,
    username: String,
    permalink: String,
    avatar_url: Option<String>,
}

/// SoundCloud search response
#[derive(Debug, Deserialize)]
struct SoundCloudSearchResponse {
    collection: Vec<SoundCloudTrack>,
    next_href: Option<String>,
    query_urn: Option<String>,
}

/// SoundCloud stream response
#[derive(Debug, Deserialize)]
struct SoundCloudStreamResponse {
    url: String,
    #[serde(rename = "type")]
    stream_type: String,
}

/// Cached authentication token
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: SystemTime,
}

/// SoundCloud API client with OAuth 2.1 authentication
pub struct SoundCloudApiClient {
    config: SoundCloudConfig,
    client: Client,
    cached_token: RwLock<Option<CachedToken>>,
}

impl SoundCloudApiClient {
    /// Create a new SoundCloud API client
    pub fn new(config: SoundCloudConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Lavalink-rust/4.0.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            cached_token: RwLock::new(None),
        }
    }

    /// Get a valid access token, refreshing if necessary
    async fn get_access_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let token_guard = self.cached_token.read().await;
            if let Some(ref cached) = *token_guard {
                if cached.expires_at > SystemTime::now() {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Need to get a new token
        self.refresh_access_token().await
    }

    /// Refresh the access token using Client Credentials flow
    async fn refresh_access_token(&self) -> Result<String> {
        debug!("Refreshing SoundCloud access token");

        let auth_url = format!("{}/oauth/token", self.config.auth_base_url);
        
        // Prepare Basic Auth header
        let credentials = format!("{}:{}", self.config.client_id, self.config.client_secret);
        let encoded_credentials = base64::engine::general_purpose::STANDARD.encode(credentials);

        let response = self
            .client
            .post(&auth_url)
            .header("Authorization", format!("Basic {}", encoded_credentials))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "SoundCloud authentication failed: {} - {}",
                status,
                body
            ));
        }

        let token_response: TokenResponse = response.json().await?;
        
        // Calculate expiration time (subtract 60 seconds for safety)
        let expires_at = SystemTime::now() + Duration::from_secs(token_response.expires_in.saturating_sub(60));
        
        let cached_token = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at,
        };

        // Cache the token
        {
            let mut token_guard = self.cached_token.write().await;
            *token_guard = Some(cached_token);
        }

        info!("Successfully refreshed SoundCloud access token");
        Ok(token_response.access_token)
    }

    /// Resolve a SoundCloud URL to get track information
    pub async fn resolve_url(&self, url: &str) -> Result<SoundCloudTrack> {
        let access_token = self.get_access_token().await?;
        
        let resolve_url = format!("{}/resolve", self.config.api_base_url);
        
        let response = self
            .client
            .get(&resolve_url)
            .header("Authorization", format!("OAuth {}", access_token))
            .query(&[("url", url)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "SoundCloud resolve failed: {} - {}",
                status,
                body
            ));
        }

        let track: SoundCloudTrack = response.json().await?;
        Ok(track)
    }

    /// Search for tracks on SoundCloud
    pub async fn search_tracks(&self, query: &str, limit: Option<u32>) -> Result<Vec<SoundCloudTrack>> {
        let access_token = self.get_access_token().await?;
        
        let search_url = format!("{}/tracks", self.config.api_base_url);
        let limit = limit.unwrap_or(20).min(200); // SoundCloud max is 200
        
        let response = self
            .client
            .get(&search_url)
            .header("Authorization", format!("OAuth {}", access_token))
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("access", "playable"), // Only get playable tracks
                ("linked_partitioning", "true"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "SoundCloud search failed: {} - {}",
                status,
                body
            ));
        }

        let search_response: SoundCloudSearchResponse = response.json().await?;
        Ok(search_response.collection)
    }

    /// Get stream URL for a track
    pub async fn get_stream_url(&self, track_id: u64) -> Result<String> {
        let access_token = self.get_access_token().await?;
        
        let stream_url = format!("{}/tracks/{}/stream", self.config.api_base_url, track_id);
        
        let response = self
            .client
            .get(&stream_url)
            .header("Authorization", format!("OAuth {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "SoundCloud stream URL failed: {} - {}",
                status,
                body
            ));
        }

        let stream_response: SoundCloudStreamResponse = response.json().await?;
        Ok(stream_response.url)
    }

    /// Convert SoundCloud track to Lavalink Track
    pub async fn to_lavalink_track(&self, sc_track: &SoundCloudTrack) -> Result<Track> {
        // Get stream URL if available
        let stream_url = if sc_track.access.as_deref() == Some("playable") {
            match self.get_stream_url(sc_track.id).await {
                Ok(url) => Some(url),
                Err(e) => {
                    warn!("Failed to get stream URL for track {}: {}", sc_track.id, e);
                    None
                }
            }
        } else {
            None
        };

        // Create track identifier (use stream URL if available, otherwise permalink)
        let identifier = stream_url.unwrap_or_else(|| sc_track.permalink_url.clone());

        // Create Lavalink track
        let track = Track {
            encoded: base64::engine::general_purpose::STANDARD.encode(&identifier), // Simple encoding for now
            info: crate::protocol::TrackInfo {
                identifier: identifier.clone(),
                seekable: true,
                author: sc_track.user.username.clone(),
                length: sc_track.duration,
                stream: false, // SoundCloud tracks are not live streams
                position: 0,
                title: sc_track.title.clone(),
                uri: Some(sc_track.permalink_url.clone()),
                artwork_url: sc_track.artwork_url.clone(),
                isrc: None, // SoundCloud doesn't provide ISRC
                source_name: "soundcloud".to_string(),
            },
            plugin_info: None,
            user_data: None,
        };

        Ok(track)
    }
}

/// Validate if a URL is a valid SoundCloud URL
pub fn is_valid_soundcloud_url(url: &str) -> bool {
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            return host == "soundcloud.com" 
                || host == "www.soundcloud.com" 
                || host == "snd.sc";
        }
    }
    false
}

/// Extract track ID from SoundCloud URL if possible
pub fn extract_track_id(url: &str) -> Option<String> {
    // This would require additional parsing logic
    // For now, return None and use the resolve endpoint
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundcloud_url_validation() {
        assert!(is_valid_soundcloud_url("https://soundcloud.com/artist/track"));
        assert!(is_valid_soundcloud_url("https://www.soundcloud.com/artist/track"));
        assert!(is_valid_soundcloud_url("https://snd.sc/abc123"));
        assert!(!is_valid_soundcloud_url("https://youtube.com/watch?v=123"));
        assert!(!is_valid_soundcloud_url("not-a-url"));
    }

    #[tokio::test]
    async fn test_soundcloud_client_creation() {
        let config = SoundCloudConfig::default();
        let client = SoundCloudApiClient::new(config);
        
        // Just test that we can create the client
        assert!(client.cached_token.read().await.is_none());
    }
}

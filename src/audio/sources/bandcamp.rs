//! Bandcamp web scraping integration for Lavalink-rust
//!
//! This module provides Bandcamp track and album loading through web scraping
//! since Bandcamp doesn't provide a public API.

use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, warn};
use url::Url;

use crate::protocol::{
    Exception, LoadResult, LoadResultData, LoadType, Severity, Track, TrackInfo,
};

/// Bandcamp web scraper for track and album information
pub struct BandcampScraper {
    client: Client,
}

impl BandcampScraper {
    /// Create a new Bandcamp scraper
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Load a track from a Bandcamp URL
    pub async fn load_track(&self, url: &str) -> Result<Track> {
        debug!("Loading Bandcamp track: {}", url);

        // Fetch the page
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch Bandcamp page: {}",
                response.status()
            ));
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Extract track data from the page
        self.extract_track_from_html(&document, url).await
    }

    /// Search for tracks on Bandcamp
    pub async fn search_tracks(&self, query: &str, limit: Option<u32>) -> Result<Vec<Track>> {
        debug!("Searching Bandcamp for: {}", query);

        let search_url = format!(
            "https://bandcamp.com/search?q={}",
            urlencoding::encode(query)
        );
        let response = self.client.get(&search_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Bandcamp search failed: {}", response.status()));
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        self.extract_search_results(&document, limit.unwrap_or(20))
            .await
    }

    /// Extract track information from Bandcamp page HTML
    async fn extract_track_from_html(&self, document: &Html, page_url: &str) -> Result<Track> {
        // Try to find the track data in the page's JavaScript
        let script_selector = Selector::parse("script[type='application/ld+json']").unwrap();

        // Look for JSON-LD structured data first
        for script_element in document.select(&script_selector) {
            if let Some(script_content) = script_element.text().next() {
                if let Ok(json_data) = serde_json::from_str::<Value>(script_content) {
                    if let Some(track) = self.parse_json_ld_track(&json_data, page_url)? {
                        return Ok(track);
                    }
                }
            }
        }

        // Fallback to parsing HTML elements
        self.extract_track_from_html_elements(document, page_url)
            .await
    }

    /// Parse track from JSON-LD structured data
    fn parse_json_ld_track(&self, json_data: &Value, page_url: &str) -> Result<Option<Track>> {
        if json_data["@type"] == "MusicRecording" {
            let title = json_data["name"]
                .as_str()
                .unwrap_or("Unknown Title")
                .to_string();
            let artist = json_data["byArtist"]["name"]
                .as_str()
                .unwrap_or("Unknown Artist")
                .to_string();

            // Duration might be in ISO 8601 format (PT1M30S) or seconds
            let duration = if let Some(duration_str) = json_data["duration"].as_str() {
                self.parse_duration(duration_str).unwrap_or(0)
            } else {
                0
            };

            let artwork_url = json_data["image"].as_str().map(|s| s.to_string());

            let track = Track {
                encoded: base64::engine::general_purpose::STANDARD.encode(page_url),
                info: TrackInfo {
                    identifier: page_url.to_string(),
                    seekable: true,
                    author: artist,
                    length: duration,
                    stream: false,
                    position: 0,
                    title,
                    uri: Some(page_url.to_string()),
                    artwork_url,
                    isrc: None,
                    source_name: "bandcamp".to_string(),
                },
                plugin_info: None,
                user_data: None,
            };

            return Ok(Some(track));
        }

        Ok(None)
    }

    /// Extract track from HTML elements (fallback method)
    async fn extract_track_from_html_elements(
        &self,
        document: &Html,
        page_url: &str,
    ) -> Result<Track> {
        // Try to extract title from meta tags or page title
        let title = self
            .extract_title(document)
            .unwrap_or_else(|| "Unknown Title".to_string());
        let artist = self
            .extract_artist(document)
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let artwork_url = self.extract_artwork_url(document);

        let track = Track {
            encoded: base64::engine::general_purpose::STANDARD.encode(page_url),
            info: TrackInfo {
                identifier: page_url.to_string(),
                seekable: true,
                author: artist,
                length: 0, // Duration not available from HTML parsing
                stream: false,
                position: 0,
                title,
                uri: Some(page_url.to_string()),
                artwork_url,
                isrc: None,
                source_name: "bandcamp".to_string(),
            },
            plugin_info: None,
            user_data: None,
        };

        Ok(track)
    }

    /// Extract search results from Bandcamp search page
    async fn extract_search_results(&self, document: &Html, limit: u32) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();

        // Bandcamp search results are in .searchresult elements
        let result_selector = Selector::parse(".searchresult").unwrap();
        let link_selector = Selector::parse(".heading a").unwrap();
        let artist_selector = Selector::parse(".subhead").unwrap();

        for (index, result_element) in document.select(&result_selector).enumerate() {
            if index >= limit as usize {
                break;
            }

            // Extract track/album URL
            if let Some(link_element) = result_element.select(&link_selector).next() {
                if let Some(href) = link_element.value().attr("href") {
                    let full_url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https:{}", href)
                    };

                    // Extract title
                    let title = link_element.text().collect::<String>().trim().to_string();

                    // Extract artist
                    let artist = result_element
                        .select(&artist_selector)
                        .next()
                        .map(|el| el.text().collect::<String>().trim().to_string())
                        .unwrap_or_else(|| "Unknown Artist".to_string());

                    let track = Track {
                        encoded: base64::engine::general_purpose::STANDARD.encode(&full_url),
                        info: TrackInfo {
                            identifier: full_url.clone(),
                            seekable: true,
                            author: artist,
                            length: 0,
                            stream: false,
                            position: 0,
                            title,
                            uri: Some(full_url),
                            artwork_url: None,
                            isrc: None,
                            source_name: "bandcamp".to_string(),
                        },
                        plugin_info: None,
                        user_data: None,
                    };

                    tracks.push(track);
                }
            }
        }

        Ok(tracks)
    }

    /// Extract title from HTML document
    fn extract_title(&self, document: &Html) -> Option<String> {
        // Try meta property first
        let meta_selector = Selector::parse("meta[property='og:title']").unwrap();
        if let Some(meta_element) = document.select(&meta_selector).next() {
            if let Some(content) = meta_element.value().attr("content") {
                return Some(content.to_string());
            }
        }

        // Try page title
        let title_selector = Selector::parse("title").unwrap();
        if let Some(title_element) = document.select(&title_selector).next() {
            let title_text = title_element.text().collect::<String>();
            // Bandcamp titles are often in format "Title | Artist"
            if let Some(title_part) = title_text.split(" | ").next() {
                return Some(title_part.trim().to_string());
            }
        }

        None
    }

    /// Extract artist from HTML document
    fn extract_artist(&self, document: &Html) -> Option<String> {
        // Try meta property
        let meta_selector = Selector::parse("meta[property='og:site_name']").unwrap();
        if let Some(meta_element) = document.select(&meta_selector).next() {
            if let Some(content) = meta_element.value().attr("content") {
                return Some(content.to_string());
            }
        }

        // Try extracting from page title
        let title_selector = Selector::parse("title").unwrap();
        if let Some(title_element) = document.select(&title_selector).next() {
            let title_text = title_element.text().collect::<String>();
            // Bandcamp titles are often in format "Title | Artist"
            if let Some(artist_part) = title_text.split(" | ").nth(1) {
                return Some(artist_part.trim().to_string());
            }
        }

        None
    }

    /// Extract artwork URL from HTML document
    fn extract_artwork_url(&self, document: &Html) -> Option<String> {
        let meta_selector = Selector::parse("meta[property='og:image']").unwrap();
        if let Some(meta_element) = document.select(&meta_selector).next() {
            if let Some(content) = meta_element.value().attr("content") {
                return Some(content.to_string());
            }
        }

        None
    }

    /// Parse duration from various formats
    fn parse_duration(&self, duration_str: &str) -> Option<u64> {
        // Handle ISO 8601 duration format (PT1M30S)
        if duration_str.starts_with("PT") {
            let mut total_seconds = 0u64;
            let duration_part = &duration_str[2..]; // Remove "PT"

            // Simple parsing for minutes and seconds
            if let Some(m_pos) = duration_part.find('M') {
                if let Ok(minutes) = duration_part[..m_pos].parse::<u64>() {
                    total_seconds += minutes * 60;
                }

                let remaining = &duration_part[m_pos + 1..];
                if let Some(s_pos) = remaining.find('S') {
                    if let Ok(seconds) = remaining[..s_pos].parse::<u64>() {
                        total_seconds += seconds;
                    }
                }
            } else if let Some(s_pos) = duration_part.find('S') {
                if let Ok(seconds) = duration_part[..s_pos].parse::<u64>() {
                    total_seconds += seconds;
                }
            }

            return Some(total_seconds * 1000); // Convert to milliseconds
        }

        // Handle direct seconds format
        if let Ok(seconds) = duration_str.parse::<u64>() {
            return Some(seconds * 1000); // Convert to milliseconds
        }

        None
    }
}

impl Default for BandcampScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate if a URL is a valid Bandcamp URL
pub fn is_valid_bandcamp_url(url: &str) -> bool {
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            return host.ends_with(".bandcamp.com") || host == "bandcamp.com";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandcamp_url_validation() {
        assert!(is_valid_bandcamp_url(
            "https://artist.bandcamp.com/track/song"
        ));
        assert!(is_valid_bandcamp_url(
            "https://artist.bandcamp.com/album/album-name"
        ));
        assert!(is_valid_bandcamp_url("https://bandcamp.com/search?q=test"));
        assert!(!is_valid_bandcamp_url(
            "https://soundcloud.com/artist/track"
        ));
        assert!(!is_valid_bandcamp_url("not-a-url"));
    }

    #[tokio::test]
    async fn test_bandcamp_scraper_creation() {
        let scraper = BandcampScraper::new();
        // Just test that we can create the scraper
        assert!(scraper.client.timeout().is_some());
    }
}

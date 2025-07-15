#[cfg(feature = "server")]
use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "server")]
use tokio::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LavalinkConfig {
    pub server: ServerConfig,
    pub lavalink: LavalinkServerConfig,
    pub metrics: Option<MetricsConfig>,
    pub sentry: Option<SentryConfig>,
    pub logging: Option<LoggingConfig>,
    #[cfg(feature = "plugins")]
    pub plugins: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub address: String,
    pub http2: Option<Http2Config>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Http2Config {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LavalinkServerConfig {
    pub server: LavalinkInnerConfig,
    pub plugins: Option<PluginsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LavalinkInnerConfig {
    pub password: String,
    pub sources: SourcesConfig,
    pub filters: FiltersConfig,
    #[serde(rename = "bufferDurationMs")]
    pub buffer_duration_ms: Option<u32>,
    #[serde(rename = "frameBufferDurationMs")]
    pub frame_buffer_duration_ms: Option<u32>,
    #[serde(rename = "opusEncodingQuality")]
    pub opus_encoding_quality: Option<u8>,
    #[serde(rename = "resamplingQuality")]
    pub resampling_quality: Option<ResamplingQuality>,
    #[serde(rename = "trackStuckThresholdMs")]
    pub track_stuck_threshold_ms: Option<u64>,
    #[serde(rename = "useSeekGhosting")]
    pub use_seek_ghosting: Option<bool>,
    #[serde(rename = "youtubePlaylistLoadLimit")]
    pub youtube_playlist_load_limit: Option<u32>,
    #[serde(rename = "youtubeSearchEnabled")]
    pub youtube_search_enabled: Option<bool>,
    #[serde(rename = "soundcloudSearchEnabled")]
    pub soundcloud_search_enabled: Option<bool>,
    #[serde(rename = "youtubeConfig")]
    pub youtube_config: Option<YoutubeConfig>,
    #[serde(rename = "playerUpdateInterval")]
    pub player_update_interval: Option<u32>,
    #[serde(rename = "gc-warnings")]
    pub gc_warnings: Option<bool>,
    pub ratelimit: Option<RateLimitConfig>,
    #[serde(rename = "httpConfig")]
    pub http_config: Option<HttpConfig>,
    pub timeouts: Option<TimeoutsConfig>,
    /// Discord bot token for voice connections (optional)
    #[serde(rename = "discordBotToken")]
    pub discord_bot_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SourcesConfig {
    pub youtube: Option<bool>,
    pub bandcamp: Option<bool>,
    pub soundcloud: Option<bool>,
    pub twitch: Option<bool>,
    pub vimeo: Option<bool>,
    pub nico: Option<bool>,
    pub http: Option<bool>,
    pub local: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FiltersConfig {
    pub volume: Option<bool>,
    pub equalizer: Option<bool>,
    pub karaoke: Option<bool>,
    pub timescale: Option<bool>,
    pub tremolo: Option<bool>,
    pub vibrato: Option<bool>,
    pub distortion: Option<bool>,
    pub rotation: Option<bool>,
    #[serde(rename = "channelMix")]
    pub channel_mix: Option<bool>,
    #[serde(rename = "lowPass")]
    pub low_pass: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResamplingQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    #[serde(rename = "ipBlocks")]
    pub ip_blocks: Option<Vec<String>>,
    #[serde(rename = "excludedIps")]
    pub excluded_ips: Option<Vec<String>>,
    pub strategy: Option<String>,
    #[serde(rename = "searchTriggersFail")]
    pub search_triggers_fail: Option<bool>,
    #[serde(rename = "retryLimit")]
    pub retry_limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YoutubeConfig {
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpConfig {
    #[serde(rename = "proxyHost")]
    pub proxy_host: Option<String>,
    #[serde(rename = "proxyPort")]
    pub proxy_port: Option<u16>,
    #[serde(rename = "proxyUser")]
    pub proxy_user: Option<String>,
    #[serde(rename = "proxyPassword")]
    pub proxy_password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutsConfig {
    #[serde(rename = "connectTimeoutMs")]
    pub connect_timeout_ms: Option<u64>,
    #[serde(rename = "connectionRequestTimeoutMs")]
    pub connection_request_timeout_ms: Option<u64>,
    #[serde(rename = "socketTimeoutMs")]
    pub socket_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginsConfig {
    pub plugins: Option<Vec<PluginDependency>>,
    #[serde(rename = "pluginsDir")]
    pub plugins_dir: Option<String>,
    #[serde(rename = "defaultPluginRepository")]
    pub default_plugin_repository: Option<String>,
    #[serde(rename = "defaultPluginSnapshotRepository")]
    pub default_plugin_snapshot_repository: Option<String>,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            plugins: None,
            plugins_dir: Some("./plugins".to_string()),
            default_plugin_repository: Some("https://maven.lavalink.dev/releases".to_string()),
            default_plugin_snapshot_repository: Some(
                "https://maven.lavalink.dev/snapshots".to_string(),
            ),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginDependency {
    pub dependency: String,
    pub repository: Option<String>,
    pub snapshot: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub prometheus: Option<PrometheusConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrometheusConfig {
    pub enabled: bool,
    pub endpoint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentryConfig {
    pub dsn: Option<String>,
    pub environment: Option<String>,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub file: Option<LogFileConfig>,
    pub level: Option<LogLevelConfig>,
    pub request: Option<RequestLoggingConfig>,
    pub logback: Option<LogbackConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogFileConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogLevelConfig {
    pub root: Option<String>,
    pub lavalink: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestLoggingConfig {
    pub enabled: Option<bool>,
    #[serde(rename = "includeClientInfo")]
    pub include_client_info: Option<bool>,
    #[serde(rename = "includeHeaders")]
    pub include_headers: Option<bool>,
    #[serde(rename = "includeQueryString")]
    pub include_query_string: Option<bool>,
    #[serde(rename = "includePayload")]
    pub include_payload: Option<bool>,
    #[serde(rename = "maxPayloadLength")]
    pub max_payload_length: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogbackConfig {
    pub rollingpolicy: Option<RollingPolicyConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RollingPolicyConfig {
    #[serde(rename = "max-file-size")]
    pub max_file_size: Option<String>,
    #[serde(rename = "max-history")]
    pub max_history: Option<u32>,
}

impl LavalinkConfig {
    pub async fn load<P: AsRef<Path>>(
        #[cfg_attr(not(feature = "server"), allow(unused_variables))] path: P,
    ) -> Result<Self> {
        #[cfg(feature = "server")]
        {
            let content = fs::read_to_string(path.as_ref()).await.with_context(|| {
                format!("Failed to read config file: {}", path.as_ref().display())
            })?;

            #[cfg(feature = "rest-api")]
            {
                let config: LavalinkConfig = serde_yaml::from_str(&content)
                    .with_context(|| "Failed to parse YAML configuration")?;
                Ok(config)
            }
            #[cfg(not(feature = "rest-api"))]
            {
                anyhow::bail!("YAML parsing requires 'rest-api' feature")
            }
        }
        #[cfg(not(feature = "server"))]
        {
            anyhow::bail!("Config loading requires 'server' feature")
        }
    }
}

impl Default for LavalinkConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 2333,
                address: "0.0.0.0".to_string(),
                http2: Some(Http2Config { enabled: false }),
            },
            lavalink: LavalinkServerConfig {
                server: LavalinkInnerConfig {
                    password: "youshallnotpass".to_string(),
                    sources: SourcesConfig {
                        youtube: Some(false),
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
                    resampling_quality: Some(ResamplingQuality::Low),
                    track_stuck_threshold_ms: Some(10000),
                    use_seek_ghosting: Some(true),
                    youtube_playlist_load_limit: Some(6),
                    youtube_search_enabled: Some(true),
                    soundcloud_search_enabled: Some(true),
                    youtube_config: None,
                    player_update_interval: Some(5),
                    gc_warnings: Some(true),
                    ratelimit: None,
                    http_config: None,
                    timeouts: Some(TimeoutsConfig {
                        connect_timeout_ms: Some(3000),
                        connection_request_timeout_ms: Some(3000),
                        socket_timeout_ms: Some(3000),
                    }),
                    discord_bot_token: None,
                },
                plugins: None,
            },
            metrics: Some(MetricsConfig {
                prometheus: Some(PrometheusConfig {
                    enabled: false,
                    endpoint: "/metrics".to_string(),
                }),
            }),
            sentry: None,
            logging: None,
            #[cfg(feature = "plugins")]
            plugins: None,
        }
    }
}

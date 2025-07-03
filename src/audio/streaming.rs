// Enhanced audio streaming implementation for Discord voice
// Provides robust streaming with monitoring, error recovery, and quality control

use anyhow::{anyhow, Result};
use songbird::input::{File as FileInput, HttpRequest, Input};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::audio::quality::{AudioQualityConfig, AudioQualityManager, QualityPreset};
use crate::protocol::Track;
use crate::voice::logging::{CorrelationId, PerformanceTimer, VoiceEvent, VoiceEventType};

/// Type alias for quality change callback
type QualityChangeCallback =
    Arc<RwLock<Option<Box<dyn Fn(QualityPreset, QualityPreset) + Send + Sync>>>>;

/// Audio streaming manager for Discord voice connections
#[allow(dead_code)] // Fields used in streaming management and quality control
pub struct AudioStreamingManager {
    /// Guild ID this manager belongs to
    guild_id: String,
    /// Current streaming session
    current_session: Arc<RwLock<Option<StreamingSession>>>,
    /// Stream metrics collector
    metrics: Arc<Mutex<StreamMetrics>>,
    /// Retry configuration
    retry_config: RetryConfig,
    /// Correlation ID for tracking operations
    correlation_id: CorrelationId,
    /// Quality manager for dynamic adjustments
    quality_manager: Arc<RwLock<Option<AudioQualityManager>>>,
    /// Quality change notification callback
    quality_change_callback: QualityChangeCallback,
}

/// Active streaming session information
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in session management
pub struct StreamingSession {
    /// Track being streamed
    pub track: Track,
    /// Quality configuration
    pub quality_config: AudioQualityConfig,
    /// Session start time
    pub start_time: Instant,
    /// Current stream state
    pub state: StreamState,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Last error encountered
    pub last_error: Option<String>,
    /// Stream health score (0-100)
    pub health_score: u8,
}

/// Stream state enumeration
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Variants used in stream state management
pub enum StreamState {
    /// Stream is initializing
    Initializing,
    /// Stream is buffering
    Buffering,
    /// Stream is actively playing
    Playing,
    /// Stream encountered an error
    Error,
    /// Stream is recovering from error
    Recovering,
    /// Stream has ended
    Ended,
}

/// Stream quality and performance metrics
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in connection quality calculations
pub struct StreamMetrics {
    /// Stream duration in seconds
    pub duration_seconds: u64,
    /// Number of buffer underruns
    pub buffer_underruns: u32,
    /// Number of connection drops
    pub connection_drops: u32,
    /// Quality degradation events
    pub quality_degradations: u32,
    /// Last update timestamp
    pub last_update: Instant,
}

impl Default for StreamMetrics {
    fn default() -> Self {
        Self {
            duration_seconds: 0,
            buffer_underruns: 0,
            connection_drops: 0,
            quality_degradations: 0,
            last_update: Instant::now(),
        }
    }
}

/// Retry configuration for stream recovery
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f32,
}

/// Stream creation options
#[derive(Debug, Clone)]
pub struct StreamOptions {
    /// Quality configuration
    pub quality_config: AudioQualityConfig,
    /// Enable stream monitoring
    pub enable_monitoring: bool,
}

/// Stream quality monitoring data
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in external API responses
pub struct StreamQualityData {
    /// Current effective bitrate (kbps)
    pub effective_bitrate: u32,
    /// Buffer health percentage (0-100)
    pub buffer_health: u8,
    /// Encoding performance score (0-100)
    pub encoding_performance: u8,
    /// Stream stability score (0-100)
    pub stream_stability: u8,
    /// Connection quality score (0-100)
    pub connection_quality: u8,
    /// Last update timestamp
    pub last_update: Instant,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            quality_config: AudioQualityConfig::default(),
            enable_monitoring: true,
        }
    }
}

impl Default for StreamQualityData {
    fn default() -> Self {
        Self {
            effective_bitrate: 128,
            buffer_health: 100,
            encoding_performance: 100,
            stream_stability: 100,
            connection_quality: 100,
            last_update: Instant::now(),
        }
    }
}

#[allow(dead_code)] // Methods used in streaming management and external APIs
impl AudioStreamingManager {
    /// Create a new audio streaming manager
    pub fn new(guild_id: String) -> Self {
        let correlation_id = CorrelationId::new();

        info!("Creating audio streaming manager for guild {}", guild_id);

        Self {
            guild_id: guild_id.clone(),
            current_session: Arc::new(RwLock::new(None)),
            metrics: Arc::new(Mutex::new(StreamMetrics::default())),
            retry_config: RetryConfig::default(),
            correlation_id,
            quality_manager: Arc::new(RwLock::new(None)),
            quality_change_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with quality manager integration
    pub fn with_quality_manager(guild_id: String, quality_manager: AudioQualityManager) -> Self {
        let correlation_id = CorrelationId::new();

        info!(
            "Creating audio streaming manager with quality integration for guild {}",
            guild_id
        );

        Self {
            guild_id: guild_id.clone(),
            current_session: Arc::new(RwLock::new(None)),
            metrics: Arc::new(Mutex::new(StreamMetrics::default())),
            retry_config: RetryConfig::default(),
            correlation_id,
            quality_manager: Arc::new(RwLock::new(Some(quality_manager))),
            quality_change_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Start streaming a track with enhanced error handling and monitoring
    pub async fn start_stream(&self, track: Track, options: StreamOptions) -> Result<Input> {
        let operation_correlation_id = CorrelationId::new();
        let timer =
            PerformanceTimer::start("audio_stream_start", operation_correlation_id.clone(), None);

        info!(
            "Starting audio stream for track '{}' in guild {} with quality preset {:?}",
            track.info.title, self.guild_id, options.quality_config.quality_preset
        );

        VoiceEvent::new(
            operation_correlation_id.clone(),
            VoiceEventType::StreamStart,
            self.guild_id.clone(),
        )
        .with_detail("track_title", &track.info.title)
        .with_detail(
            "quality_preset",
            &format!("{:?}", options.quality_config.quality_preset),
        )
        .log();

        // Create streaming session
        let session = StreamingSession {
            track: track.clone(),
            quality_config: options.quality_config.clone(),
            start_time: Instant::now(),
            state: StreamState::Initializing,
            retry_count: 0,
            last_error: None,
            health_score: 100,
        };

        // Store the session
        *self.current_session.write().await = Some(session);

        // Create audio input with retry logic
        let input = self
            .create_stream_input_with_retry(&track, &options)
            .await?;

        // Update session state to playing
        if let Some(ref mut session) = *self.current_session.write().await {
            session.state = StreamState::Playing;
        }

        // Start monitoring if enabled
        if options.enable_monitoring {
            self.start_stream_monitoring().await;
        }

        timer.complete_with_context(
            true,
            [
                ("track_title".to_string(), track.info.title.clone()),
                (
                    "quality_preset".to_string(),
                    format!("{:?}", options.quality_config.quality_preset),
                ),
            ]
            .into(),
        );

        info!(
            "Successfully started audio stream for track '{}' in guild {}",
            track.info.title, self.guild_id
        );

        Ok(input)
    }

    /// Create audio input with retry logic and enhanced error handling
    async fn create_stream_input_with_retry(
        &self,
        track: &Track,
        options: &StreamOptions,
    ) -> Result<Input> {
        let mut retry_count = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            match self
                .create_enhanced_audio_input(track, &options.quality_config)
                .await
            {
                Ok(input) => {
                    if retry_count > 0 {
                        info!(
                            "Successfully created audio input for track '{}' after {} retries",
                            track.info.title, retry_count
                        );
                    }
                    return Ok(input);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count > self.retry_config.max_retries {
                        error!(
                            "Failed to create audio input for track '{}' after {} retries: {}",
                            track.info.title,
                            retry_count - 1,
                            e
                        );

                        // Update session with error
                        if let Some(ref mut session) = *self.current_session.write().await {
                            session.state = StreamState::Error;
                            session.last_error = Some(e.to_string());
                            session.retry_count = retry_count - 1;
                        }

                        return Err(anyhow!(
                            "Failed to create audio input after {} retries: {}",
                            retry_count - 1,
                            e
                        ));
                    }

                    warn!(
                        "Failed to create audio input for track '{}' (attempt {}): {}. Retrying in {:?}",
                        track.info.title, retry_count, e, delay
                    );

                    // Update session state
                    if let Some(ref mut session) = *self.current_session.write().await {
                        session.state = StreamState::Recovering;
                        session.retry_count = retry_count;
                        session.last_error = Some(e.to_string());
                    }

                    sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f32 * self.retry_config.backoff_multiplier)
                                as u64,
                        ),
                        self.retry_config.max_delay,
                    );
                }
            }
        }
    }

    /// Create enhanced audio input with quality control and validation
    async fn create_enhanced_audio_input(
        &self,
        track: &Track,
        quality_config: &AudioQualityConfig,
    ) -> Result<Input> {
        let uri = track
            .info
            .uri
            .as_ref()
            .ok_or_else(|| anyhow!("Track has no URI: {}", track.info.title))?;

        debug!(
            "Creating enhanced audio input for URI: {} with {}kbps bitrate",
            uri, quality_config.bitrate
        );

        // Validate URI format
        self.validate_audio_uri(uri)?;

        // Create input based on source type
        let input = if uri.starts_with("http://") || uri.starts_with("https://") {
            self.create_http_stream_input(uri, quality_config).await?
        } else if uri.starts_with("file://") || std::path::Path::new(uri).exists() {
            self.create_file_stream_input(uri, quality_config).await?
        } else {
            return Err(anyhow!("Unsupported audio source type: {}", uri));
        };

        debug!(
            "Successfully created enhanced audio input for URI: {} with quality preset {:?}",
            uri, quality_config.quality_preset
        );

        Ok(input)
    }

    /// Create HTTP stream input with timeout and validation
    async fn create_http_stream_input(
        &self,
        uri: &str,
        quality_config: &AudioQualityConfig,
    ) -> Result<Input> {
        debug!("Creating HTTP stream input for URI: {}", uri);

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Lavalink-rust/4.0.0")
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        // Validate URL accessibility with HEAD request
        let head_response = timeout(Duration::from_secs(10), client.head(uri).send())
            .await
            .map_err(|_| anyhow!("HTTP HEAD request timeout for URI: {}", uri))?
            .map_err(|e| anyhow!("HTTP HEAD request failed for URI {}: {}", uri, e))?;

        if !head_response.status().is_success() {
            return Err(anyhow!(
                "HTTP resource not accessible: {} (status: {})",
                uri,
                head_response.status()
            ));
        }

        // Check content type if available
        if let Some(content_type) = head_response.headers().get("content-type") {
            let content_type_str = content_type.to_str().unwrap_or("");
            if !self.is_supported_audio_content_type(content_type_str) {
                warn!(
                    "Potentially unsupported content type for URI {}: {}",
                    uri, content_type_str
                );
            }
        }

        // Create HTTP input
        let http_input = HttpRequest::new(client, uri.to_string());

        info!(
            "Created HTTP stream input for URI: {} with {}kbps target bitrate",
            uri, quality_config.bitrate
        );

        Ok(Input::from(http_input))
    }

    /// Create file stream input with validation
    async fn create_file_stream_input(
        &self,
        uri: &str,
        quality_config: &AudioQualityConfig,
    ) -> Result<Input> {
        let file_path = if uri.starts_with("file://") {
            uri.strip_prefix("file://").unwrap_or(uri)
        } else {
            uri
        };

        debug!("Creating file stream input for path: {}", file_path);

        // Validate file exists and is readable
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            return Err(anyhow!("Audio file does not exist: {}", file_path));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", file_path));
        }

        // Check file extension
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_str().unwrap_or("");
            if !self.is_supported_audio_extension(ext_str) {
                warn!("Potentially unsupported audio file extension: {}", ext_str);
            }
        }

        // Create file input
        let file_input = FileInput::new(file_path.to_string());

        info!(
            "Created file stream input for path: {} with {}kbps target bitrate",
            file_path, quality_config.bitrate
        );

        Ok(Input::from(file_input))
    }

    /// Validate audio URI format and accessibility
    fn validate_audio_uri(&self, uri: &str) -> Result<()> {
        if uri.is_empty() {
            return Err(anyhow!("Empty URI provided"));
        }

        if uri.len() > 2048 {
            return Err(anyhow!("URI too long: {} characters", uri.len()));
        }

        // Basic URL validation for HTTP sources
        if uri.starts_with("http://") || uri.starts_with("https://") {
            url::Url::parse(uri).map_err(|e| anyhow!("Invalid HTTP URL format: {}", e))?;
        }

        Ok(())
    }

    /// Check if content type is supported for audio streaming
    fn is_supported_audio_content_type(&self, content_type: &str) -> bool {
        let supported_types = [
            "audio/mpeg",
            "audio/mp3",
            "audio/wav",
            "audio/wave",
            "audio/flac",
            "audio/ogg",
            "audio/vorbis",
            "audio/aac",
            "audio/mp4",
            "audio/m4a",
            "application/ogg",
        ];

        supported_types
            .iter()
            .any(|&supported| content_type.contains(supported))
    }

    /// Check if file extension is supported for audio streaming
    fn is_supported_audio_extension(&self, extension: &str) -> bool {
        let supported_extensions = ["mp3", "wav", "flac", "ogg", "aac", "m4a", "mp4", "wma"];

        supported_extensions
            .iter()
            .any(|&supported| extension.eq_ignore_ascii_case(supported))
    }

    /// Start stream monitoring in background
    async fn start_stream_monitoring(&self) {
        let session = self.current_session.clone();
        let metrics = self.metrics.clone();
        let guild_id = self.guild_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Check if session is still active
                let session_guard = session.read().await;
                if session_guard.is_none() {
                    break;
                }

                if let Some(ref current_session) = *session_guard {
                    if matches!(
                        current_session.state,
                        StreamState::Ended | StreamState::Error
                    ) {
                        break;
                    }

                    // Update metrics
                    let mut metrics_guard = metrics.lock().await;
                    metrics_guard.duration_seconds = current_session.start_time.elapsed().as_secs();
                    metrics_guard.last_update = Instant::now();

                    debug!(
                        "Stream monitoring update for guild {}: duration={}s, health={}",
                        guild_id, metrics_guard.duration_seconds, current_session.health_score
                    );
                }

                drop(session_guard);
            }

            debug!("Stream monitoring ended for guild {}", guild_id);
        });
    }

    /// Get current streaming session information
    pub async fn get_current_session(&self) -> Option<StreamingSession> {
        self.current_session.read().await.clone()
    }

    /// Get current stream metrics
    pub async fn get_stream_metrics(&self) -> StreamMetrics {
        self.metrics.lock().await.clone()
    }

    /// Stop current stream
    pub async fn stop_stream(&self) -> Result<()> {
        info!("Stopping audio stream for guild {}", self.guild_id);

        if let Some(ref mut session) = *self.current_session.write().await {
            session.state = StreamState::Ended;

            VoiceEvent::new(
                self.correlation_id.clone(),
                VoiceEventType::StreamEnd,
                self.guild_id.clone(),
            )
            .with_detail("track_title", &session.track.info.title)
            .with_detail(
                "duration_seconds",
                &session.start_time.elapsed().as_secs().to_string(),
            )
            .log();
        }

        *self.current_session.write().await = None;

        info!("Audio stream stopped for guild {}", self.guild_id);
        Ok(())
    }

    /// Check if currently streaming
    pub async fn is_streaming(&self) -> bool {
        if let Some(ref session) = *self.current_session.read().await {
            matches!(session.state, StreamState::Playing | StreamState::Buffering)
        } else {
            false
        }
    }

    /// Get stream health score (0-100)
    pub async fn get_stream_health(&self) -> u8 {
        if let Some(ref session) = *self.current_session.read().await {
            session.health_score
        } else {
            0
        }
    }

    /// Set quality manager for dynamic quality adjustments
    pub async fn set_quality_manager(&self, quality_manager: AudioQualityManager) {
        info!(
            "Setting quality manager for streaming manager in guild {}",
            self.guild_id
        );
        *self.quality_manager.write().await = Some(quality_manager);
    }

    /// Set quality change notification callback
    pub async fn set_quality_change_callback<F>(&self, callback: F)
    where
        F: Fn(QualityPreset, QualityPreset) + Send + Sync + 'static,
    {
        *self.quality_change_callback.write().await = Some(Box::new(callback));
    }

    /// Get current stream quality data
    pub async fn get_stream_quality_data(&self) -> StreamQualityData {
        if let Some(quality_manager) = self.quality_manager.read().await.as_ref() {
            let quality_metrics = quality_manager.get_quality_metrics().await;
            StreamQualityData {
                effective_bitrate: quality_metrics.effective_bitrate,
                buffer_health: quality_metrics.buffer_health,
                encoding_performance: quality_metrics.encoding_performance,
                stream_stability: quality_metrics.stream_stability,
                connection_quality: self.calculate_connection_quality().await,
                last_update: quality_metrics.last_update,
            }
        } else {
            StreamQualityData::default()
        }
    }

    /// Trigger quality adjustment based on current stream conditions
    pub async fn trigger_quality_adjustment(&self) -> Result<()> {
        if let Some(quality_manager) = self.quality_manager.write().await.as_mut() {
            // Update quality manager with current stream metrics
            self.update_quality_manager_metrics(quality_manager).await?;

            // Get current preset before adjustment
            let current_preset = quality_manager.get_current_preset();

            // Trigger adjustment
            quality_manager.trigger_quality_adjustment().await?;

            // Check if preset changed and notify
            let new_preset = quality_manager.get_current_preset();
            if current_preset != new_preset {
                self.notify_quality_change(current_preset, new_preset, "Stream conditions")
                    .await;
            }
        }

        Ok(())
    }

    /// Apply quality preset change to current stream
    pub async fn apply_quality_preset(&self, preset: QualityPreset) -> Result<()> {
        if let Some(quality_manager) = self.quality_manager.write().await.as_mut() {
            let current_preset = quality_manager.get_current_preset();

            // Apply the preset change
            quality_manager.apply_preset(preset)?;

            info!(
                "Applied quality preset change for guild {}: {:?} -> {:?}",
                self.guild_id, current_preset, preset
            );

            // Notify about the change
            self.notify_quality_change(current_preset, preset, "Manual adjustment")
                .await;

            // Update current session if active
            if let Some(session) = self.current_session.write().await.as_mut() {
                session.quality_config = quality_manager.get_config().clone();
            }
        }

        Ok(())
    }

    /// Calculate connection quality score based on stream metrics
    async fn calculate_connection_quality(&self) -> u8 {
        let metrics = self.metrics.lock().await;

        // Base score starts at 100
        let mut score = 100u8;

        // Reduce score based on connection drops
        if metrics.connection_drops > 0 {
            score = score.saturating_sub(metrics.connection_drops.min(50) as u8);
        }

        // Reduce score based on buffer underruns
        if metrics.buffer_underruns > 0 {
            score = score.saturating_sub((metrics.buffer_underruns * 2).min(30) as u8);
        }

        // Consider quality degradations
        if metrics.quality_degradations > 0 {
            score = score.saturating_sub((metrics.quality_degradations * 3).min(40) as u8);
        }

        score
    }

    /// Update quality manager with current stream metrics
    async fn update_quality_manager_metrics(
        &self,
        quality_manager: &mut AudioQualityManager,
    ) -> Result<()> {
        let metrics = self.metrics.lock().await;
        let session = self.current_session.read().await;

        if let Some(session) = session.as_ref() {
            // Calculate quality metrics from stream data
            let buffer_health = if metrics.buffer_underruns == 0 {
                100
            } else {
                (100 - (metrics.buffer_underruns * 10).min(100)) as u8
            };

            let stream_stability = if metrics.connection_drops == 0 {
                100
            } else {
                (100 - (metrics.connection_drops * 15).min(100)) as u8
            };

            let encoding_performance = session.health_score;

            // Update quality manager metrics
            let _ = quality_manager
                .update_quality_metrics(
                    session.quality_config.bitrate,
                    buffer_health,
                    encoding_performance,
                    stream_stability,
                )
                .await;
        }

        Ok(())
    }

    /// Notify about quality change
    async fn notify_quality_change(&self, from: QualityPreset, to: QualityPreset, reason: &str) {
        if let Some(callback) = self.quality_change_callback.read().await.as_ref() {
            callback(from, to);
        }

        info!(
            "Quality change notification for guild {}: {:?} -> {:?} ({})",
            self.guild_id, from, to, reason
        );

        // Log the quality change event
        let mut details = std::collections::HashMap::new();
        details.insert("from_preset".to_string(), format!("{from:?}"));
        details.insert("to_preset".to_string(), format!("{to:?}"));
        details.insert("reason".to_string(), reason.to_string());

        let event = VoiceEvent {
            correlation_id: self.correlation_id.clone(),
            event_type: VoiceEventType::AudioQualityChange,
            guild_id: self.guild_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            details,
            metrics: None,
        };

        debug!("Quality change event: {:?}", event);
    }
}

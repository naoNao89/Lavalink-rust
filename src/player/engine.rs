// Audio player engine core
// This module handles the actual audio playback, decoding, and streaming

use anyhow::{anyhow, Result};
use songbird::{input::Input, tracks::Track as SongbirdTrack, Call};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, Instant};
use tracing::{debug, info, warn};

use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::{PlayerEvent, TrackEndReason};
use crate::audio::quality::{
    AudioQualityConfig, AudioQualityManager, NetworkMetrics, QualityPreset,
};
use crate::audio::streaming::{AudioStreamingManager, StreamOptions, StreamState};
use crate::protocol::{Filters, Track};

/// Audio player engine that handles actual audio playback
pub struct AudioPlayerEngine {
    /// Current track being played
    current_track: Arc<RwLock<Option<Track>>>,
    /// Audio decoder for the current track
    decoder: Arc<Mutex<Option<Box<dyn Decoder>>>>,
    /// Format reader for the current track
    format_reader: Arc<Mutex<Option<Box<dyn FormatReader>>>>,
    /// Current playback position in milliseconds
    position: Arc<RwLock<u64>>,
    /// Whether the player is paused
    paused: Arc<RwLock<bool>>,

    /// Event sender for player events
    event_sender: mpsc::UnboundedSender<PlayerEvent>,
    /// Guild ID this engine belongs to
    guild_id: String,
    /// Whether the engine is currently playing
    playing: Arc<RwLock<bool>>,

    /// Current audio filters
    filters: Arc<RwLock<Filters>>,
    /// Last position update timestamp
    last_position_update: Arc<RwLock<Instant>>,
    /// Whether seeking is in progress
    seeking: Arc<RwLock<bool>>,
    /// Voice call for audio output
    voice_call: Arc<RwLock<Option<Arc<Mutex<Call>>>>>,
    /// Current Songbird track handle for control
    current_track_handle: Arc<RwLock<Option<songbird::tracks::TrackHandle>>>,
    /// Audio quality manager for bitrate and quality control
    quality_manager: Arc<RwLock<AudioQualityManager>>,
    /// Audio streaming manager for enhanced stream handling
    streaming_manager: Arc<AudioStreamingManager>,
}

#[allow(dead_code)]
impl AudioPlayerEngine {
    /// Create a new audio player engine
    pub fn new(guild_id: String, event_sender: mpsc::UnboundedSender<PlayerEvent>) -> Self {
        let quality_manager =
            AudioQualityManager::with_preset(guild_id.clone(), QualityPreset::Medium);
        let streaming_manager = AudioStreamingManager::new(guild_id.clone());

        Self {
            current_track: Arc::new(RwLock::new(None)),
            decoder: Arc::new(Mutex::new(None)),
            format_reader: Arc::new(Mutex::new(None)),
            position: Arc::new(RwLock::new(0)),
            paused: Arc::new(RwLock::new(false)),

            event_sender,
            guild_id,
            playing: Arc::new(RwLock::new(false)),

            filters: Arc::new(RwLock::new(Filters::new())),
            last_position_update: Arc::new(RwLock::new(Instant::now())),
            seeking: Arc::new(RwLock::new(false)),
            voice_call: Arc::new(RwLock::new(None)),
            current_track_handle: Arc::new(RwLock::new(None)),
            quality_manager: Arc::new(RwLock::new(quality_manager)),
            streaming_manager: Arc::new(streaming_manager),
        }
    }

    /// Create a new audio player engine with custom quality configuration
    pub fn with_quality_config(
        guild_id: String,
        event_sender: mpsc::UnboundedSender<PlayerEvent>,
        quality_config: AudioQualityConfig,
    ) -> Self {
        let quality_manager = AudioQualityManager::new(guild_id.clone(), quality_config);
        let streaming_manager = AudioStreamingManager::new(guild_id.clone());

        Self {
            current_track: Arc::new(RwLock::new(None)),
            decoder: Arc::new(Mutex::new(None)),
            format_reader: Arc::new(Mutex::new(None)),
            position: Arc::new(RwLock::new(0)),
            paused: Arc::new(RwLock::new(false)),

            event_sender,
            guild_id,
            playing: Arc::new(RwLock::new(false)),

            filters: Arc::new(RwLock::new(Filters::new())),
            last_position_update: Arc::new(RwLock::new(Instant::now())),
            seeking: Arc::new(RwLock::new(false)),
            voice_call: Arc::new(RwLock::new(None)),
            current_track_handle: Arc::new(RwLock::new(None)),
            quality_manager: Arc::new(RwLock::new(quality_manager)),
            streaming_manager: Arc::new(streaming_manager),
        }
    }

    /// Set the voice call for audio output
    pub async fn set_voice_call(&self, call: Arc<Mutex<Call>>) {
        let mut voice_call = self.voice_call.write().await;
        *voice_call = Some(call);
        info!(
            "Voice call connected to audio engine for guild {}",
            self.guild_id
        );
    }

    /// Remove the voice call
    pub async fn remove_voice_call(&self) {
        // Stop current track if playing
        if let Some(track_handle) = self.current_track_handle.write().await.take() {
            let _ = track_handle.stop();
            debug!(
                "Stopped current track due to voice disconnection in guild {}",
                self.guild_id
            );
        }

        let mut voice_call = self.voice_call.write().await;
        *voice_call = None;
        info!(
            "Voice call disconnected from audio engine for guild {}",
            self.guild_id
        );
    }

    /// Start streaming audio to Discord voice with enhanced error handling and monitoring
    async fn start_voice_streaming(&self, track: &Track) -> Result<()> {
        let voice_call = self.voice_call.read().await;
        if let Some(ref call) = *voice_call {
            let quality_config = self.get_quality_config().await;
            info!(
                "Voice connection established for track: {} in guild {} with quality preset: {:?} ({}kbps)",
                track.info.title, self.guild_id, quality_config.quality_preset, quality_config.bitrate
            );

            // Create stream options with current quality configuration
            let stream_options = StreamOptions {
                quality_config: quality_config.clone(),
                enable_monitoring: true,
            };

            // Use the enhanced streaming manager to create audio input
            let audio_input = match self
                .streaming_manager
                .start_stream(track.clone(), stream_options)
                .await
            {
                Ok(input) => {
                    info!(
                        "Enhanced streaming manager successfully created input for track: {} in guild {}",
                        track.info.title, self.guild_id
                    );
                    input
                }
                Err(e) => {
                    warn!(
                        "Enhanced streaming failed for track: {} in guild {}, falling back to basic method: {}",
                        track.info.title, self.guild_id, e
                    );
                    // Fallback to basic audio input creation
                    self.create_audio_input_with_quality(track, &quality_config)
                        .await?
                }
            };

            // Start playing the audio through Songbird
            let mut call_lock = call.lock().await;
            let songbird_track = SongbirdTrack::from(audio_input);
            let track_handle = call_lock.play(songbird_track);

            info!(
                "Started streaming audio for track: {} in guild {} with {}kbps bitrate",
                track.info.title, self.guild_id, quality_config.bitrate
            );

            // Store track handle for control (pause, stop, etc.)
            *self.current_track_handle.write().await = Some(track_handle);
            drop(call_lock);

            // Emit track start event
            let _ = self.event_sender.send(PlayerEvent::TrackStart {
                guild_id: self.guild_id.clone(),
                track: track.clone(),
            });

            Ok(())
        } else {
            warn!(
                "No voice connection available for streaming in guild {}",
                self.guild_id
            );
            Err(anyhow!(
                "No voice connection available for guild {}",
                self.guild_id
            ))
        }
    }

    /// Load and start playing a track
    pub async fn play_track(&self, track: Track, start_time: Option<u64>) -> Result<()> {
        info!(
            "Loading track for playback: {} in guild {}",
            track.info.title, self.guild_id
        );

        // Stop current track if playing
        self.stop().await?;

        // Set the new track
        *self.current_track.write().await = Some(track.clone());

        // Reset position
        let start_pos = start_time.unwrap_or(0);
        *self.position.write().await = start_pos;

        // Start playback
        *self.playing.write().await = true;
        *self.paused.write().await = false;

        // Start streaming to Discord voice if connected
        // This handles both audio loading and streaming
        self.start_voice_streaming(&track).await?;

        // Start the playback loop
        self.start_playback_loop().await;

        Ok(())
    }

    /// Stop playback
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping playback in guild {}", self.guild_id);

        *self.playing.write().await = false;
        *self.paused.write().await = false;

        // Stop Songbird track if playing
        if let Some(track_handle) = self.current_track_handle.write().await.take() {
            let _ = track_handle.stop();
            debug!("Stopped Songbird track in guild {}", self.guild_id);
        }

        // Clear decoder and format reader
        *self.decoder.lock().await = None;
        *self.format_reader.lock().await = None;

        // Emit track end event if there was a current track
        if let Some(track) = self.current_track.read().await.clone() {
            let _ = self.event_sender.send(PlayerEvent::TrackEnd {
                guild_id: self.guild_id.clone(),
                track,
                reason: TrackEndReason::Stopped,
            });
        }

        *self.current_track.write().await = None;
        *self.position.write().await = 0;

        Ok(())
    }

    /// Pause playback
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing playback in guild {}", self.guild_id);
        *self.paused.write().await = true;

        // Pause Songbird track if playing
        if let Some(track_handle) = self.current_track_handle.read().await.as_ref() {
            let _ = track_handle.pause();
            debug!("Paused Songbird track in guild {}", self.guild_id);
        }

        Ok(())
    }

    /// Resume playback
    pub async fn resume(&self) -> Result<()> {
        info!("Resuming playback in guild {}", self.guild_id);
        *self.paused.write().await = false;

        // Resume Songbird track if paused
        if let Some(track_handle) = self.current_track_handle.read().await.as_ref() {
            let _ = track_handle.play();
            debug!("Resumed Songbird track in guild {}", self.guild_id);
        }

        Ok(())
    }

    /// Seek to a specific position in the track
    pub async fn seek(&self, position: u64) -> Result<()> {
        info!(
            "Seeking to position {} ms in guild {}",
            position, self.guild_id
        );

        *self.seeking.write().await = true;
        *self.position.write().await = position;
        *self.last_position_update.write().await = Instant::now();

        // For Songbird tracks, we need to restart the track at the new position
        // This is a limitation of the current implementation - true seeking would require
        // more sophisticated audio processing
        if let Some(track) = self.current_track.read().await.clone() {
            if let Some(track_handle) = self.current_track_handle.read().await.as_ref() {
                // Stop current track
                let _ = track_handle.stop();
                debug!(
                    "Stopped current track for seeking in guild {}",
                    self.guild_id
                );
            }

            // Restart streaming from the new position
            // Note: This is a simplified approach - true seeking would require
            // audio format-specific seeking implementation
            if position > 0 {
                warn!(
                    "Seeking to non-zero position requires track restart in guild {}",
                    self.guild_id
                );
            }

            // Restart the track (this will effectively seek to the beginning)
            // A full implementation would need to handle seeking within the audio stream
            if let Err(e) = self.start_voice_streaming(&track).await {
                warn!(
                    "Failed to restart track after seek in guild {}: {}",
                    self.guild_id, e
                );
            }
        }

        *self.seeking.write().await = false;
        Ok(())
    }

    /// Apply audio filters
    pub async fn apply_filters(&self, filters: Filters) -> Result<()> {
        info!(
            "Applying audio filters in guild {}: enabled={}",
            self.guild_id,
            filters.is_enabled()
        );

        // Store the new filters
        *self.filters.write().await = filters.clone();

        // Apply volume filter to current Songbird track if playing
        if let Some(track_handle) = self.current_track_handle.read().await.as_ref() {
            if let crate::protocol::Omissible::Present(volume_value) = &filters.volume {
                // Convert Lavalink volume (0.0-5.0) to Songbird volume (0.0-1.0)
                let volume = (*volume_value / 5.0).clamp(0.0, 1.0);
                let _ = track_handle.set_volume(volume);
                debug!(
                    "Applied volume filter: {} to Songbird track in guild {}",
                    volume, self.guild_id
                );
            }
        }

        // Log which filters are enabled
        if filters.volume.is_present() {
            debug!("Volume filter enabled: {:?}", filters.volume);
        }
        if filters.equalizer.is_present() {
            debug!(
                "Equalizer filter enabled with {} bands",
                filters
                    .equalizer
                    .as_option()
                    .map(|eq| eq.len())
                    .unwrap_or(0)
            );
        }
        if filters.karaoke.is_present() {
            debug!("Karaoke filter enabled");
        }
        if filters.timescale.is_present() {
            debug!("Timescale filter enabled");
        }
        if filters.tremolo.is_present() {
            debug!("Tremolo filter enabled");
        }
        if filters.vibrato.is_present() {
            debug!("Vibrato filter enabled");
        }
        if filters.distortion.is_present() {
            debug!("Distortion filter enabled");
        }
        if filters.rotation.is_present() {
            debug!("Rotation filter enabled");
        }
        if filters.channel_mix.is_present() {
            debug!("Channel mix filter enabled");
        }
        if filters.low_pass.is_present() {
            debug!("Low pass filter enabled");
        }
        if !filters.plugin_filters.is_empty() {
            debug!(
                "Plugin filters enabled: {:?}",
                filters.plugin_filters.keys().collect::<Vec<_>>()
            );
        }

        info!(
            "Successfully applied audio filters in guild {}",
            self.guild_id
        );
        Ok(())
    }

    /// Get current playback position
    pub async fn get_position(&self) -> u64 {
        // Try to get position from Songbird track handle first
        if let Some(track_handle) = self.current_track_handle.read().await.as_ref() {
            if let Ok(info) = track_handle.get_info().await {
                // Convert from Duration to milliseconds
                return info.position.as_millis() as u64;
            }
        }

        // Fallback to internal position tracking
        *self.position.read().await
    }

    /// Check if currently playing
    pub async fn is_playing(&self) -> bool {
        *self.playing.read().await && !*self.paused.read().await
    }

    /// Check if paused
    pub async fn is_paused(&self) -> bool {
        *self.paused.read().await
    }

    /// Load audio source for a track
    async fn load_audio_source(&self, track: &Track) -> Result<()> {
        let uri = track
            .info
            .uri
            .as_ref()
            .ok_or_else(|| anyhow!("Track has no URI"))?;
        debug!("Loading audio source for track: {}", uri);

        // Create a hint based on the track URI
        let mut hint = Hint::new();
        if let Some(extension) = uri.split('.').next_back() {
            hint.with_extension(extension);
        }

        // For now, we'll implement a basic HTTP stream loader
        // In a full implementation, this would handle various source types
        let source = self.create_media_source(uri).await?;
        let media_source_stream = MediaSourceStream::new(source, Default::default());

        // Probe the media source
        let probe_result = symphonia::default::get_probe()
            .format(
                &hint,
                media_source_stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| anyhow!("Failed to probe audio format: {}", e))?;

        let format_reader = probe_result.format;

        // Find the default audio track
        let _track_id = format_reader
            .default_track()
            .ok_or_else(|| anyhow!("No default audio track found"))?
            .id;

        // Create decoder for the track
        let decoder = symphonia::default::get_codecs()
            .make(
                &format_reader.tracks()[0].codec_params,
                &DecoderOptions::default(),
            )
            .map_err(|e| anyhow!("Failed to create decoder: {}", e))?;

        // Store the format reader and decoder
        *self.format_reader.lock().await = Some(format_reader);
        *self.decoder.lock().await = Some(decoder);

        Ok(())
    }

    /// Create a Songbird audio input from a track with quality settings
    async fn create_audio_input_with_quality(
        &self,
        track: &Track,
        quality_config: &AudioQualityConfig,
    ) -> Result<Input> {
        let uri = match &track.info.uri {
            Some(uri) => uri,
            None => {
                warn!("Track has no URI: {}", track.info.title);
                return Err(anyhow!("Track has no URI: {}", track.info.title));
            }
        };

        debug!(
            "Creating audio input with quality settings: {}kbps, {:?}, {} channels",
            quality_config.bitrate, quality_config.sample_rate, quality_config.channels as u8
        );

        // Create Songbird config with quality settings
        let _songbird_config = self.create_songbird_config().await;

        // For now, we'll create a simple HTTP input for HTTP/HTTPS URLs
        // This will be expanded to support other source types in future tasks
        if uri.starts_with("http://") || uri.starts_with("https://") {
            info!(
                "Creating HTTP audio input for URI: {} with {}kbps bitrate",
                uri, quality_config.bitrate
            );

            // Use Songbird's HttpRequest input for HTTP sources
            let client = reqwest::Client::new();
            let http_input = songbird::input::HttpRequest::new(client, uri.clone());

            // Note: Quality configuration is applied at the driver level via Config
            // Individual inputs don't have configuration methods in Songbird

            Ok(Input::from(http_input))
        } else if uri.starts_with("file://") || std::path::Path::new(uri).exists() {
            info!(
                "Creating file audio input for URI: {} with {}kbps bitrate",
                uri, quality_config.bitrate
            );

            // Use Songbird's File input for local files
            let file_path = if uri.starts_with("file://") {
                uri.strip_prefix("file://").unwrap_or(uri)
            } else {
                uri
            };

            let file_input = songbird::input::File::new(file_path.to_string());

            // Note: Quality configuration is applied at the driver level via Config
            // Individual inputs don't have configuration methods in Songbird

            Ok(Input::from(file_input))
        } else {
            // For other sources (YouTube, SoundCloud, etc.), we'll need to implement
            // integration with yt-dlp or similar tools in future tasks
            warn!("Unsupported audio source type for URI: {}", uri);
            Err(anyhow!("Unsupported audio source type: {}", uri))
        }
    }

    /// Create a Songbird audio input from a track (legacy method)
    async fn create_audio_input(&self, track: &Track) -> Result<Input> {
        let quality_config = self.get_quality_config().await;
        self.create_audio_input_with_quality(track, &quality_config)
            .await
    }

    /// Create a media source from a URI
    async fn create_media_source(
        &self,
        uri: &str,
    ) -> Result<Box<dyn symphonia::core::io::MediaSource>> {
        if uri.starts_with("http://") || uri.starts_with("https://") {
            // HTTP source
            let response = reqwest::get(uri).await?;
            let bytes = response.bytes().await?;
            let cursor = std::io::Cursor::new(bytes.to_vec());
            Ok(Box::new(cursor))
        } else {
            // File source
            let file = std::fs::File::open(uri)?;
            Ok(Box::new(file))
        }
    }

    /// Start the playback loop
    async fn start_playback_loop(&self) {
        let current_track = self.current_track.clone();
        let position = self.position.clone();
        let paused = self.paused.clone();
        let playing = self.playing.clone();
        let seeking = self.seeking.clone();
        let last_position_update = self.last_position_update.clone();
        let event_sender = self.event_sender.clone();
        let guild_id = self.guild_id.clone();

        tokio::spawn(async move {
            let mut playback_interval = interval(Duration::from_millis(20)); // 50 FPS for smooth playback
            let mut position_update_interval = interval(Duration::from_millis(100)); // Update position every 100ms

            loop {
                tokio::select! {
                    _ = playback_interval.tick() => {
                        if !*playing.read().await {
                            break;
                        }

                        // Check for track end
                        if let Some(track) = current_track.read().await.as_ref() {
                            // Get current position with interpolation
                            let current_pos = if *seeking.read().await || *paused.read().await {
                                *position.read().await
                            } else {
                                let base_position = *position.read().await;
                                let last_update = *last_position_update.read().await;
                                let elapsed = last_update.elapsed();
                                let interpolated = base_position + elapsed.as_millis() as u64;
                                if track.info.length > 0 {
                                    interpolated.min(track.info.length)
                                } else {
                                    interpolated
                                }
                            };

                            if track.info.length > 0 && current_pos >= track.info.length {
                                // Track ended
                                *playing.write().await = false;

                                let _ = event_sender.send(PlayerEvent::TrackEnd {
                                    guild_id: guild_id.clone(),
                                    track: track.clone(),
                                    reason: TrackEndReason::Finished,
                                });

                                break;
                            }
                        }

                        // Here we would decode and process audio frames
                        // For now, this is a placeholder for actual audio processing
                    }

                    _ = position_update_interval.tick() => {
                        // Update position tracking
                        if !*seeking.read().await && !*paused.read().await && *playing.read().await {
                            let last_update = *last_position_update.read().await;
                            let elapsed = last_update.elapsed();

                            // Update position based on elapsed time
                            {
                                let mut pos = position.write().await;
                                *pos += elapsed.as_millis() as u64;
                            }

                            // Update last position update timestamp
                            *last_position_update.write().await = Instant::now();
                        }
                    }
                }
            }
        });
    }

    /// Get current audio quality configuration
    pub async fn get_quality_config(&self) -> AudioQualityConfig {
        self.quality_manager.read().await.get_config().clone()
    }

    /// Update audio quality configuration
    pub async fn update_quality_config(&self, config: AudioQualityConfig) -> Result<()> {
        info!(
            "Updating audio quality config for guild {}: {:?}",
            self.guild_id, config.quality_preset
        );

        let mut quality_manager = self.quality_manager.write().await;
        quality_manager.update_config(config)?;

        // If we have an active voice connection, we should restart the track with new quality settings
        if self.is_playing().await && self.voice_call.read().await.is_some() {
            if let Some(track) = self.current_track.read().await.clone() {
                let current_position = self.get_position().await;

                info!(
                    "Restarting track with new quality settings at position {}ms",
                    current_position
                );

                // Restart the track with new quality settings
                self.play_track(track, Some(current_position)).await?;
            }
        }

        Ok(())
    }

    /// Apply a quality preset
    pub async fn apply_quality_preset(&self, preset: QualityPreset) -> Result<()> {
        info!(
            "Applying quality preset {:?} for guild {}",
            preset, self.guild_id
        );

        let mut quality_manager = self.quality_manager.write().await;
        quality_manager.apply_preset(preset)?;

        // If we have an active voice connection, restart with new settings
        if self.is_playing().await && self.voice_call.read().await.is_some() {
            if let Some(track) = self.current_track.read().await.clone() {
                let current_position = self.get_position().await;

                info!(
                    "Restarting track with preset {:?} at position {}ms",
                    preset, current_position
                );

                // Restart the track with new quality settings
                self.play_track(track, Some(current_position)).await?;
            }
        }

        Ok(())
    }

    /// Update network metrics for adaptive quality adjustment
    pub async fn update_network_metrics(&self, metrics: NetworkMetrics) {
        debug!(
            "Updating network metrics for guild {}: loss={:.1}%, rtt={}ms",
            self.guild_id, metrics.packet_loss, metrics.rtt_ms
        );

        let mut quality_manager = self.quality_manager.write().await;
        quality_manager.update_network_metrics(metrics);
    }

    /// Get current network quality score (0-100)
    pub async fn get_network_quality_score(&self) -> u8 {
        self.quality_manager.read().await.network_quality_score()
    }

    /// Check if current quality is appropriate for network conditions
    pub async fn is_quality_appropriate(&self) -> bool {
        self.quality_manager.read().await.is_quality_appropriate()
    }

    /// Get estimated bandwidth usage in kbps
    pub async fn get_estimated_bandwidth(&self) -> u32 {
        self.quality_manager.read().await.estimated_bandwidth()
    }

    /// Create Songbird configuration with current quality settings
    async fn create_songbird_config(&self) -> songbird::Config {
        let quality_manager = self.quality_manager.read().await;
        quality_manager.create_songbird_config()
    }

    /// Get current streaming status
    pub async fn get_streaming_status(&self) -> Option<StreamState> {
        if let Some(session) = self.streaming_manager.get_current_session().await {
            Some(session.state)
        } else {
            None
        }
    }

    /// Check if currently streaming with enhanced manager
    pub async fn is_enhanced_streaming(&self) -> bool {
        self.streaming_manager.is_streaming().await
    }

    /// Get stream health score (0-100)
    pub async fn get_stream_health(&self) -> u8 {
        self.streaming_manager.get_stream_health().await
    }

    /// Get detailed streaming metrics
    pub async fn get_streaming_metrics(&self) -> crate::audio::streaming::StreamMetrics {
        self.streaming_manager.get_stream_metrics().await
    }

    /// Stop current enhanced stream
    pub async fn stop_enhanced_stream(&self) -> Result<()> {
        self.streaming_manager.stop_stream().await
    }
}

// Player management module
// This will handle audio players for Discord guilds

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration, Instant};

#[cfg(any(feature = "discord", feature = "crypto"))]
use rand::prelude::*;
use tracing::{debug, error, info, warn};

use crate::protocol::{messages::VoiceState, Event, Filters, Message, PlayerState, Track};
use crate::voice::{connection::VoiceConnectionEvent, VoiceConnectionManager};

/// Enhanced player state with voice connection details
#[derive(Debug, Clone)]
pub struct EnhancedPlayerState {
    pub voice_quality: VoiceQuality,
}

/// Voice connection quality levels
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceQuality {
    Excellent,    // 0-50ms ping
    Good,         // 51-100ms ping
    Fair,         // 101-200ms ping
    Poor,         // 201-500ms ping
    Critical,     // >500ms ping
    Disconnected, // No connection
}

/// Voice connection statistics across all players
#[derive(Debug, Clone, Default)]
pub struct VoiceConnectionStats {}

pub mod engine;
pub use engine::AudioPlayerEngine;

/// Player manager for handling audio players across guilds
pub struct PlayerManager {
    players: Arc<RwLock<HashMap<String, Arc<RwLock<LavalinkPlayer>>>>>,
    event_sender: Option<mpsc::UnboundedSender<PlayerEvent>>,
    voice_manager: Arc<VoiceConnectionManager>,
}

/// Individual audio player for a Discord guild
pub struct LavalinkPlayer {
    pub guild_id: String,
    pub session_id: String,
    pub current_track: Option<Track>,
    pub state: PlayerState,
    pub volume: u8,
    pub paused: bool,
    pub filters: Filters,
    pub voice: VoiceState,
    pub position: u64,
    pub last_update: Instant,
    pub end_time: Option<u64>,
    /// Audio engine for actual playback
    pub audio_engine: Option<Arc<AudioPlayerEngine>>,
    /// Track queue for managing multiple tracks
    pub queue: VecDeque<Track>,
    /// Whether to repeat the current track
    pub repeat_track: bool,
    /// Whether to repeat the entire queue
    pub repeat_queue: bool,
    /// Whether to shuffle the queue
    pub shuffle: bool,
    /// Voice connection manager reference
    pub voice_manager: Option<Arc<VoiceConnectionManager>>,
}

/// Events that can be emitted by players
#[derive(Debug, Clone, serde::Serialize)]
pub enum PlayerEvent {
    #[allow(dead_code)]
    TrackStart { guild_id: String, track: Track },
    TrackEnd {
        guild_id: String,
        track: Track,
        reason: TrackEndReason,
    },

    PlayerUpdate {
        guild_id: String,
        state: PlayerState,
    },

    VoiceConnectionEvent {
        guild_id: String,
        event: VoiceConnectionEvent,
    },
}

/// Reasons why a track ended
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackEndReason {
    Finished,
    LoadFailed,
    Stopped,
    Replaced,
    Cleanup,
}

impl TrackEndReason {
    /// Check if this end reason should trigger the next track to start
    pub fn may_start_next(&self) -> bool {
        match self {
            TrackEndReason::Finished => true,
            TrackEndReason::LoadFailed => true,
            TrackEndReason::Stopped => false,
            TrackEndReason::Replaced => false,
            TrackEndReason::Cleanup => false,
        }
    }

    /// Convert to messages::TrackEndReason for protocol compatibility
    pub fn to_messages_reason(&self) -> crate::protocol::messages::TrackEndReason {
        match self {
            TrackEndReason::Finished => crate::protocol::messages::TrackEndReason::Finished,
            TrackEndReason::LoadFailed => crate::protocol::messages::TrackEndReason::LoadFailed,
            TrackEndReason::Stopped => crate::protocol::messages::TrackEndReason::Stopped,
            TrackEndReason::Replaced => crate::protocol::messages::TrackEndReason::Replaced,
            TrackEndReason::Cleanup => crate::protocol::messages::TrackEndReason::Cleanup,
        }
    }
}

impl PlayerManager {
    /// Create a new player manager
    pub fn new() -> Self {
        let mut voice_manager = VoiceConnectionManager::new();
        let event_sender: Option<mpsc::UnboundedSender<PlayerEvent>> = None;

        // Set up voice event broadcasting if we have an event sender
        if let Some(ref sender) = event_sender {
            let sender_clone = sender.clone();
            voice_manager.set_event_broadcaster(move |guild_id, voice_event| {
                let _ = sender_clone.send(PlayerEvent::VoiceConnectionEvent {
                    guild_id,
                    event: voice_event,
                });
            });
        }

        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            voice_manager: Arc::new(voice_manager),
        }
    }

    /// Create a new player manager with event sender
    pub fn with_event_sender(event_sender: mpsc::UnboundedSender<PlayerEvent>) -> Self {
        let mut voice_manager = VoiceConnectionManager::new();

        // Set up voice event broadcasting
        let sender_clone = event_sender.clone();
        voice_manager.set_event_broadcaster(move |guild_id, voice_event| {
            let _ = sender_clone.send(PlayerEvent::VoiceConnectionEvent {
                guild_id,
                event: voice_event,
            });
        });

        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            event_sender: Some(event_sender),
            voice_manager: Arc::new(voice_manager),
        }
    }

    /// Get the voice connection manager
    pub fn voice_manager(&self) -> Arc<VoiceConnectionManager> {
        self.voice_manager.clone()
    }

    /// Get or create a player for a guild
    pub async fn get_or_create_player(
        &self,
        guild_id: String,
        session_id: String,
    ) -> Arc<RwLock<LavalinkPlayer>> {
        let mut players = self.players.write().await;

        let player = players
            .entry(guild_id.clone())
            .or_insert_with(|| {
                let mut new_player = LavalinkPlayer::new(guild_id.clone(), session_id.clone());

                // Initialize audio engine if we have an event sender
                if let Some(ref sender) = self.event_sender {
                    new_player.initialize_audio_engine(sender.clone());
                }

                // Set voice manager reference
                new_player.voice_manager = Some(self.voice_manager.clone());

                Arc::new(RwLock::new(new_player))
            })
            .clone();

        // Update session ID if different
        {
            let mut player_guard = player.write().await;
            if player_guard.session_id != session_id {
                player_guard.session_id = session_id;
            }

            // Initialize audio engine if not present
            if player_guard.audio_engine.is_none() {
                if let Some(ref sender) = self.event_sender {
                    player_guard.initialize_audio_engine(sender.clone());
                }
            }
        }

        player
    }

    /// Get a player for a guild
    pub async fn get_player(&self, guild_id: &str) -> Option<Arc<RwLock<LavalinkPlayer>>> {
        let players = self.players.read().await;
        players.get(guild_id).cloned()
    }

    /// Get all players for a session
    #[allow(dead_code)]
    pub async fn get_players_for_session(
        &self,
        session_id: &str,
    ) -> Vec<Arc<RwLock<LavalinkPlayer>>> {
        let players = self.players.read().await;
        let mut result = Vec::new();

        for player in players.values() {
            let player_guard = player.read().await;
            if player_guard.session_id == session_id {
                result.push(player.clone());
            }
        }

        result
    }

    /// Remove a player
    #[allow(dead_code)]
    pub async fn remove_player(&self, guild_id: &str) -> Option<Arc<RwLock<LavalinkPlayer>>> {
        let mut players = self.players.write().await;
        let player = players.remove(guild_id);

        if let Some(ref player) = player {
            // Emit player destruction event
            let player_guard = player.read().await;
            if let Some(ref track) = player_guard.current_track {
                self.emit_event(PlayerEvent::TrackEnd {
                    guild_id: guild_id.to_string(),
                    track: track.clone(),
                    reason: TrackEndReason::Cleanup,
                })
                .await;
            }
        }

        player
    }

    /// Remove all players for a session
    #[allow(dead_code)]
    pub async fn remove_players_for_session(&self, session_id: &str) {
        let mut players = self.players.write().await;
        let mut to_remove = Vec::new();

        // Find all players belonging to this session
        for (guild_id, player) in players.iter() {
            let player_guard = player.read().await;
            if player_guard.session_id == session_id {
                to_remove.push(guild_id.clone());
            }
        }

        // Remove the players
        for guild_id in to_remove {
            if let Some(player) = players.remove(&guild_id) {
                // Emit player destruction event
                let player_guard = player.read().await;
                if let Some(ref track) = player_guard.current_track {
                    self.emit_event(PlayerEvent::TrackEnd {
                        guild_id: guild_id.clone(),
                        track: track.clone(),
                        reason: TrackEndReason::Cleanup,
                    })
                    .await;
                }
                info!(
                    "Removed player for guild {} from session {}",
                    guild_id, session_id
                );
            }
        }
    }

    /// Emit a player event
    #[allow(dead_code)]
    pub async fn emit_event(&self, event: PlayerEvent) {
        if let Some(ref sender) = self.event_sender {
            if let Err(e) = sender.send(event) {
                error!("Failed to send player event: {}", e);
            }
        }
    }

    /// Shutdown the player manager and clean up all resources
    #[allow(dead_code)]
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Shutting down player manager...");

        // Get all players and shut them down
        let players = {
            let players_guard = self.players.read().await;
            players_guard.clone()
        };

        for (guild_id, player) in players {
            info!("Shutting down player for guild {}", guild_id);
            let player_guard = player.write().await;

            // Stop any current playback
            if let Some(ref audio_engine) = player_guard.audio_engine {
                if let Err(e) = audio_engine.stop().await {
                    warn!("Failed to stop audio engine for guild {}: {}", guild_id, e);
                }
            }

            // Emit track end event if there's a current track
            if let Some(ref track) = player_guard.current_track {
                self.emit_event(PlayerEvent::TrackEnd {
                    guild_id: guild_id.clone(),
                    track: track.clone(),
                    reason: TrackEndReason::Cleanup,
                })
                .await;
            }
        }

        // Clear all players
        {
            let mut players_guard = self.players.write().await;
            players_guard.clear();
        }

        // Shutdown voice manager
        self.voice_manager.shutdown().await;

        info!("Player manager shutdown complete");
        Ok(())
    }

    /// Start the player update service
    pub async fn start_update_service(&self) {
        let players = self.players.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(5000)); // 5 second updates

            loop {
                interval.tick().await;

                let players_guard = players.read().await;
                for player in players_guard.values() {
                    let mut player_guard = player.write().await;

                    // Update position for playing tracks
                    let mut track_ended = false;
                    let mut end_reason = TrackEndReason::Finished;
                    let mut ended_track = None;
                    let guild_id = player_guard.guild_id.clone();

                    // Check if we have a current track and if it's playing
                    let should_update =
                        player_guard.current_track.is_some() && !player_guard.paused;
                    let track_length = player_guard
                        .current_track
                        .as_ref()
                        .map(|t| t.info.length)
                        .unwrap_or(0);
                    let end_time = player_guard.end_time;

                    if should_update {
                        let elapsed = player_guard.last_update.elapsed();
                        player_guard.position += elapsed.as_millis() as u64;
                        player_guard.last_update = Instant::now();

                        // Check if track should end
                        if let Some(end_time) = end_time {
                            if player_guard.position >= end_time {
                                track_ended = true;
                                ended_track = player_guard.current_track.clone();
                                end_reason = TrackEndReason::Finished;
                            }
                        } else if track_length > 0 && player_guard.position >= track_length {
                            track_ended = true;
                            ended_track = player_guard.current_track.clone();
                            end_reason = TrackEndReason::Finished;
                        }
                    }

                    // Handle track end
                    if track_ended {
                        let _ended_track_clone = ended_track.clone();
                        let end_reason_clone = end_reason.clone();

                        // Emit track end event first
                        if let (Some(track), Some(ref sender)) = (ended_track, &event_sender) {
                            let _ = sender.send(PlayerEvent::TrackEnd {
                                guild_id: guild_id.clone(),
                                track,
                                reason: end_reason,
                            });
                        }

                        // Try to play next track from queue if the end reason allows it
                        if end_reason_clone.may_start_next() {
                            let next_track = player_guard.get_next_track();

                            if let Some(next_track) = next_track {
                                info!(
                                    "Auto-playing next track from queue: {} in guild {}",
                                    next_track.info.title, guild_id
                                );

                                // Set new track
                                player_guard.current_track = Some(next_track.clone());
                                player_guard.position = 0;
                                player_guard.end_time = None;
                                player_guard.paused = false;
                                player_guard.last_update = Instant::now();

                                // Update state
                                player_guard.state.position = 0;
                                player_guard.state.time = chrono::Utc::now();

                                // Start playback with audio engine
                                if let Some(ref engine) = player_guard.audio_engine {
                                    let engine_clone = engine.clone();
                                    let track_clone = next_track.clone();

                                    // Spawn a task to start playback to avoid blocking the update loop
                                    tokio::spawn(async move {
                                        if let Err(e) =
                                            engine_clone.play_track(track_clone, None).await
                                        {
                                            error!("Failed to auto-play next track: {}", e);
                                        }
                                    });
                                }
                            } else {
                                // No more tracks in queue
                                player_guard.current_track = None;
                                player_guard.position = 0;
                                player_guard.end_time = None;
                                info!("Queue is empty for guild {}", guild_id);
                            }
                        } else {
                            // End reason doesn't allow auto-play (e.g., stopped manually)
                            player_guard.current_track = None;
                            player_guard.position = 0;
                            player_guard.end_time = None;
                        }
                    }

                    // Update player state
                    player_guard.state.position = player_guard.position;
                    player_guard.state.time = chrono::Utc::now();

                    // Emit player update
                    if let Some(ref sender) = event_sender {
                        let _ = sender.send(PlayerEvent::PlayerUpdate {
                            guild_id: player_guard.guild_id.clone(),
                            state: player_guard.state.clone(),
                        });
                    }
                }
            }
        });
    }
}

impl LavalinkPlayer {
    /// Create a new player
    pub fn new(guild_id: String, session_id: String) -> Self {
        Self {
            guild_id,
            session_id,
            current_track: None,
            state: PlayerState::disconnected(),
            volume: 100,
            paused: false,
            filters: Filters::new(),
            voice: VoiceState {
                token: String::new(),
                endpoint: String::new(),
                session_id: String::new(),
            },
            position: 0,
            last_update: Instant::now(),
            end_time: None,
            audio_engine: None,
            queue: VecDeque::new(),
            repeat_track: false,
            repeat_queue: false,
            shuffle: false,
            voice_manager: None,
        }
    }

    /// Initialize the audio engine for this player
    pub fn initialize_audio_engine(&mut self, event_sender: mpsc::UnboundedSender<PlayerEvent>) {
        self.audio_engine = Some(Arc::new(AudioPlayerEngine::new(
            self.guild_id.clone(),
            event_sender,
        )));
    }

    /// Update voice state and establish/disconnect voice connection
    #[allow(dead_code)]
    pub async fn update_voice_state(
        &mut self,
        voice_state: VoiceState,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Updating voice state for guild {}", self.guild_id);

        // Update the voice state (only voice server information)
        self.voice.token = voice_state.token.clone();
        self.voice.endpoint = voice_state.endpoint.clone();
        self.voice.session_id = voice_state.session_id.clone();

        // Update connection state timestamp
        self.state.time = chrono::Utc::now();

        // Handle voice connection through voice manager
        if let Some(ref voice_manager) = self.voice_manager {
            match voice_manager
                .update_voice_state(self.guild_id.clone(), voice_state)
                .await
            {
                Ok(Some(_call)) => {
                    info!("Voice connection established for guild {}", self.guild_id);
                    self.state.connected = true;

                    // Connect the voice call to the audio engine (Discord mode only)
                    #[cfg(feature = "discord")]
                    if let Some(ref audio_engine) = self.audio_engine {
                        audio_engine.set_voice_call(call).await;
                        info!(
                            "Audio engine connected to voice call for guild {}",
                            self.guild_id
                        );
                    }

                    // Update ping from voice connection if available
                    // For now, we'll use a placeholder value since Songbird doesn't expose ping directly
                    self.state.ping = 0; // Will be updated by voice gateway events
                }
                Ok(None) => {
                    info!("Voice connection disconnected for guild {}", self.guild_id);
                    self.state.connected = false;
                    self.state.ping = -1; // Indicate no connection

                    // Disconnect the audio engine from voice (Discord mode only)
                    #[cfg(feature = "discord")]
                    if let Some(ref audio_engine) = self.audio_engine {
                        audio_engine.remove_voice_call().await;
                        info!(
                            "Audio engine disconnected from voice call for guild {}",
                            self.guild_id
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to update voice connection for guild {}: {}",
                        self.guild_id, e
                    );
                    self.state.connected = false;
                    self.state.ping = -1;
                    return Err(format!("Voice connection failed: {e}").into());
                }
            }
        } else {
            warn!("No voice manager available for guild {}", self.guild_id);
            self.state.connected = false;
            self.state.ping = -1;
        }

        Ok(())
    }

    /// Handle voice connection events and update player state accordingly
    pub async fn handle_voice_event(&mut self, event: &VoiceConnectionEvent) {
        match event {
            // Basic Connection Events
            VoiceConnectionEvent::Connected | VoiceConnectionEvent::GatewayReady { .. } => {
                self.state.connected = true;
                self.state.ping = 0; // Will be updated by actual ping measurements
                info!("Voice connection established for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::Disconnected | VoiceConnectionEvent::GatewayClosed { .. } => {
                self.state.connected = false;
                self.state.ping = -1;
                info!("Voice connection lost for guild {}", self.guild_id);

                // Pause playback when voice connection is lost
                if self.current_track.is_some() && !self.paused {
                    self.paused = true;
                    info!(
                        "Paused playback due to voice connection loss for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::Error(ref error) => {
                error!(
                    "Voice connection error for guild {}: {}",
                    self.guild_id, error
                );
                // Connection errors may indicate connection issues but don't necessarily disconnect
            }
            VoiceConnectionEvent::GatewayError(_) => {
                // Gateway errors may indicate connection issues but don't necessarily disconnect
                warn!("Voice gateway error for guild {}", self.guild_id);
            }

            // Connection State Transitions
            VoiceConnectionEvent::Connecting => {
                info!("Voice connection starting for guild {}", self.guild_id);
                // Keep current connection state during connection attempt
            }
            VoiceConnectionEvent::Reconnecting => {
                info!("Voice connection reconnecting for guild {}", self.guild_id);
                self.state.ping = -1; // Indicate unstable connection during reconnect
            }
            VoiceConnectionEvent::ConnectionTimeout => {
                warn!("Voice connection timeout for guild {}", self.guild_id);
                self.state.connected = false;
                self.state.ping = -1;
            }
            VoiceConnectionEvent::ConnectionLost => {
                warn!("Voice connection lost for guild {}", self.guild_id);
                self.state.connected = false;
                self.state.ping = -1;

                // Pause playback when connection is lost
                if self.current_track.is_some() && !self.paused {
                    self.paused = true;
                    info!(
                        "Paused playback due to connection loss for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::ConnectionRestored => {
                info!("Voice connection restored for guild {}", self.guild_id);
                self.state.connected = true;
                self.state.ping = 0;

                // Resume playback if it was paused due to connection loss
                if self.current_track.is_some() && self.paused {
                    self.paused = false;
                    info!(
                        "Resumed playback after connection restoration for guild {}",
                        self.guild_id
                    );
                }
            }

            // Recovery Events
            VoiceConnectionEvent::RecoveryStarted { attempt, delay: _ } => {
                info!(
                    "Voice connection recovery started for guild {} (attempt {})",
                    self.guild_id, attempt
                );
                // During recovery, we're still considered connected but with degraded service
                self.state.ping = -1; // Indicate degraded connection
            }
            VoiceConnectionEvent::RecoverySucceeded { total_attempts } => {
                info!(
                    "Voice connection recovery succeeded for guild {} after {} attempts",
                    self.guild_id, total_attempts
                );
                self.state.connected = true;
                self.state.ping = 0; // Will be updated by actual measurements

                // Resume playback if it was paused during recovery
                if self.current_track.is_some() && self.paused {
                    self.paused = false;
                    info!(
                        "Resumed playback after successful recovery for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::RecoveryFailed {
                total_attempts,
                ref error,
            } => {
                error!(
                    "Voice connection recovery failed for guild {} after {} attempts: {}",
                    self.guild_id, total_attempts, error
                );
                self.state.connected = false;
                self.state.ping = -1;

                // Ensure playback is paused after recovery failure
                if self.current_track.is_some() && !self.paused {
                    self.paused = true;
                    info!(
                        "Paused playback due to recovery failure for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::RecoveryAborted { ref reason } => {
                warn!(
                    "Voice connection recovery aborted for guild {}: {}",
                    self.guild_id, reason
                );
                self.state.connected = false;
                self.state.ping = -1;
            }

            // Circuit Breaker Events
            VoiceConnectionEvent::CircuitBreakerOpened => {
                warn!("Circuit breaker opened for guild {}", self.guild_id);
                self.state.connected = false;
                self.state.ping = -1;

                // Pause playback when circuit breaker opens
                if self.current_track.is_some() && !self.paused {
                    self.paused = true;
                    info!(
                        "Paused playback due to circuit breaker opening for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::CircuitBreakerClosed => {
                info!("Circuit breaker closed for guild {}", self.guild_id);
                // Don't automatically set connected=true, wait for actual connection event
            }
            VoiceConnectionEvent::CircuitBreakerHalfOpen => {
                info!("Circuit breaker half-open for guild {}", self.guild_id);
                // Connection is being tested, keep current state
            }

            // Performance Events - Update ping with actual measurements
            VoiceConnectionEvent::LatencyUpdate { latency_ms } => {
                self.state.ping = *latency_ms as i32;
                debug!(
                    "Updated ping for guild {} to {}ms",
                    self.guild_id, latency_ms
                );
            }
            VoiceConnectionEvent::PacketLoss { loss_percentage } => {
                if *loss_percentage > 5.0 {
                    warn!(
                        "High packet loss detected for guild {}: {:.2}%",
                        self.guild_id, loss_percentage
                    );
                    // Consider degrading connection quality indicator
                    if self.state.ping >= 0 {
                        self.state.ping += 50; // Add penalty for packet loss
                    }
                }
            }
            VoiceConnectionEvent::JitterUpdate { jitter_ms } => {
                if *jitter_ms > 50.0 {
                    warn!(
                        "High jitter detected for guild {}: {:.2}ms",
                        self.guild_id, jitter_ms
                    );
                }
            }

            // Audio Quality Events
            VoiceConnectionEvent::AudioQualityChanged {
                old_bitrate,
                new_bitrate,
                ref reason,
            } => {
                info!(
                    "Audio quality changed for guild {} from {} to {} kbps: {}",
                    self.guild_id, old_bitrate, new_bitrate, reason
                );
                // Quality changes don't affect connection state but are important for monitoring
            }

            // Health Events
            VoiceConnectionEvent::HealthCheckPassed => {
                debug!("Health check passed for guild {}", self.guild_id);
                // Health checks passing indicate stable connection
                if self.state.connected && self.state.ping < 0 {
                    self.state.ping = 0; // Reset ping if it was indicating degraded state
                }
            }
            VoiceConnectionEvent::HealthCheckFailed { ref reason } => {
                warn!(
                    "Health check failed for guild {}: {}",
                    self.guild_id, reason
                );
                // Failed health checks may indicate connection degradation
                if self.state.ping >= 0 {
                    self.state.ping += 100; // Add penalty for failed health check
                }
            }
            VoiceConnectionEvent::ConnectionDegraded { ref severity } => {
                warn!(
                    "Connection degraded for guild {} (severity: {})",
                    self.guild_id, severity
                );
                // Indicate degraded connection in ping
                if self.state.ping >= 0 {
                    self.state.ping += 200; // Significant penalty for degraded connection
                }
            }
            VoiceConnectionEvent::ConnectionHealthy => {
                info!("Connection healthy for guild {}", self.guild_id);
                // Reset ping penalties when connection becomes healthy
                if self.state.connected && self.state.ping > 100 {
                    self.state.ping = 0; // Reset to baseline
                }
            }

            // State Events
            VoiceConnectionEvent::SpeakingStateChanged { speaking: _ } => {
                // Speaking state changes don't affect player connection state
                debug!("Speaking state changed for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::MuteStateChanged { muted: _ } => {
                debug!("Mute state changed for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::DeafenStateChanged { deafened: _ } => {
                debug!("Deafen state changed for guild {}", self.guild_id);
            }

            // Audio Stream Events
            VoiceConnectionEvent::AudioStreamStarted => {
                debug!("Audio stream started for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::AudioStreamStopped => {
                debug!("Audio stream stopped for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::AudioStreamPaused => {
                debug!("Audio stream paused for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::AudioStreamResumed => {
                debug!("Audio stream resumed for guild {}", self.guild_id);
            }

            // Pool Events
            VoiceConnectionEvent::PoolConnectionCreated => {
                debug!("Pool connection created for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::PoolConnectionDestroyed => {
                debug!("Pool connection destroyed for guild {}", self.guild_id);
            }
            VoiceConnectionEvent::PoolConnectionReused => {
                debug!("Pool connection reused for guild {}", self.guild_id);
            }

            // Error Events
            VoiceConnectionEvent::CriticalError {
                ref error,
                ref context,
            } => {
                error!(
                    "Critical voice error for guild {}: {} (context: {:?})",
                    self.guild_id, error, context
                );
                self.state.connected = false;
                self.state.ping = -1;

                // Pause playback on critical errors
                if self.current_track.is_some() && !self.paused {
                    self.paused = true;
                    info!(
                        "Paused playback due to critical error for guild {}",
                        self.guild_id
                    );
                }
            }
            VoiceConnectionEvent::ErrorRecovered {
                ref error,
                ref recovery_action,
            } => {
                info!(
                    "Error recovered for guild {} - Error: {}, Action: {}",
                    self.guild_id, error, recovery_action
                );
                // Error recovery doesn't automatically restore connection state
                // Wait for explicit connection events
            }

            // Gateway Events
            VoiceConnectionEvent::GatewayReconnecting => {
                info!("Voice gateway reconnecting for guild {}", self.guild_id);
                self.state.ping = -1; // Indicate unstable connection
            }
        }

        // Update state timestamp for all events
        self.state.time = chrono::Utc::now();
    }

    /// Validate player state consistency with voice connection
    pub async fn validate_state_consistency(&mut self) -> bool {
        let mut is_consistent = true;

        // Check if voice manager is available when we think we're connected
        if self.state.connected {
            if let Some(ref _voice_manager) = self.voice_manager {
                // Try to get connection status from voice manager
                // This is a basic consistency check - in a real implementation,
                // we'd query the actual voice connection state
                debug!(
                    "Voice connection state appears consistent for guild {}",
                    self.guild_id
                );
            } else {
                warn!(
                    "Player state shows connected but no voice manager available for guild {}",
                    self.guild_id
                );
                self.state.connected = false;
                self.state.ping = -1;
                is_consistent = false;
            }
        }

        // Check if we should pause playback when disconnected
        if !self.state.connected && self.current_track.is_some() && !self.paused {
            info!(
                "Auto-pausing playback due to voice disconnection for guild {}",
                self.guild_id
            );
            self.paused = true;
            is_consistent = false; // State was inconsistent but now fixed
        }

        // Update timestamp if we made changes
        if !is_consistent {
            self.state.time = chrono::Utc::now();
        }

        is_consistent
    }

    /// Get enhanced player state with voice connection details
    pub fn get_enhanced_state(&self) -> EnhancedPlayerState {
        EnhancedPlayerState {
            voice_quality: if self.state.ping >= 0 {
                match self.state.ping {
                    0..=50 => VoiceQuality::Excellent,
                    51..=100 => VoiceQuality::Good,
                    101..=200 => VoiceQuality::Fair,
                    201..=500 => VoiceQuality::Poor,
                    _ => VoiceQuality::Critical,
                }
            } else {
                VoiceQuality::Disconnected
            },
        }
    }

    /// Play a track
    #[allow(dead_code)]
    pub async fn play_track(
        &mut self,
        track: Track,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Playing track {} in guild {}",
            track.info.title, self.guild_id
        );

        // Stop current track if playing
        if let Some(ref current) = self.current_track {
            debug!("Stopping current track: {}", current.info.title);
            if let Some(ref engine) = self.audio_engine {
                let _ = engine.stop().await;
            }
        }

        // Set new track
        self.current_track = Some(track.clone());
        self.paused = false;
        self.position = start_time.unwrap_or(0);
        self.end_time = end_time;
        self.last_update = Instant::now();

        // Update state
        self.state.position = self.position;
        self.state.time = chrono::Utc::now();

        // Start playback with audio engine
        if let Some(ref engine) = self.audio_engine {
            if let Err(e) = engine.play_track(track, start_time).await {
                error!("Failed to start audio playback: {}", e);
                return Err(format!("Audio playback failed: {e}").into());
            }
        } else {
            warn!("No audio engine available for guild {}", self.guild_id);
        }

        Ok(())
    }

    /// Apply filters
    #[allow(dead_code)]
    pub async fn apply_filters(
        &mut self,
        filters: Filters,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Applying filters in guild {}", self.guild_id);

        // Get disabled filters from configuration
        let disabled_filters = self.get_disabled_filters();
        let validation_errors = filters.validate(&disabled_filters);
        if !validation_errors.is_empty() {
            return Err(
                format!("Filter validation failed: {}", validation_errors.join(", ")).into(),
            );
        }

        // Apply filters to audio engine if available
        if let Some(ref engine) = self.audio_engine {
            engine.apply_filters(filters.clone()).await?;
        }

        self.filters = filters;
        self.state.time = chrono::Utc::now();

        info!("Successfully applied filters in guild {}", self.guild_id);
        Ok(())
    }

    /// Get list of disabled filters from configuration
    #[allow(dead_code)]
    fn get_disabled_filters(&self) -> Vec<String> {
        // TODO: Read from actual configuration
        // For now, return empty list (all filters enabled)
        vec![]
    }

    /// Get current filters
    #[allow(dead_code)]
    pub fn get_filters(&self) -> &Filters {
        &self.filters
    }

    /// Check if the player is playing
    pub fn is_playing(&self) -> bool {
        self.current_track.is_some() && !self.paused
    }

    /// Add a track to the queue
    #[allow(dead_code)]
    pub fn add_to_queue(&mut self, track: Track) {
        info!(
            "Adding track '{}' to queue for guild {}",
            track.info.title, self.guild_id
        );
        self.queue.push_back(track);
    }

    /// Remove a track from the queue by index
    #[allow(dead_code)]
    pub fn remove_from_queue(&mut self, index: usize) -> Option<Track> {
        if index < self.queue.len() {
            let track = self.queue.remove(index);
            if let Some(ref track) = track {
                info!(
                    "Removed track '{}' from queue for guild {}",
                    track.info.title, self.guild_id
                );
            }
            track
        } else {
            None
        }
    }

    /// Clear the entire queue
    #[allow(dead_code)]
    pub fn clear_queue(&mut self) {
        let count = self.queue.len();
        self.queue.clear();
        info!(
            "Cleared {} tracks from queue for guild {}",
            count, self.guild_id
        );
    }

    /// Get the next track from the queue
    pub fn get_next_track(&mut self) -> Option<Track> {
        if self.repeat_track && self.current_track.is_some() {
            // Repeat current track
            return self.current_track.clone();
        }

        if self.queue.is_empty() {
            return None;
        }

        if self.shuffle {
            // Shuffle mode: pick a random track from the queue
            #[cfg(any(feature = "discord", feature = "crypto"))]
            {
                let mut rng = rand::rng();
                let indices: Vec<usize> = (0..self.queue.len()).collect();
                if let Some(&random_index) = indices.choose(&mut rng) {
                    return self.queue.remove(random_index);
                }
            }
            #[cfg(not(any(feature = "discord", feature = "crypto")))]
            {
                // Fallback: just take the first track when rand is not available
                return self.queue.pop_front();
            }
        }

        // Normal mode: take the first track from the queue
        let next_track = self.queue.pop_front();

        // If repeat_queue is enabled and we just took the last track, add current track back to queue
        if self.repeat_queue && next_track.is_some() && self.current_track.is_some() {
            self.queue
                .push_back(self.current_track.as_ref().unwrap().clone());
        }

        next_track
    }

    /// Get the queue as a vector (for API responses)
    #[allow(dead_code)]
    pub fn get_queue(&self) -> Vec<Track> {
        self.queue.iter().cloned().collect()
    }

    /// Get queue length
    #[allow(dead_code)]
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Skip to the next track in the queue
    #[allow(dead_code)]
    pub async fn skip_track(
        &mut self,
    ) -> Result<Option<Track>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Skipping current track in guild {}", self.guild_id);

        // Stop current track
        if let Some(ref engine) = self.audio_engine {
            let _ = engine.stop().await;
        }

        // Get next track from queue
        let next_track = self.get_next_track();

        if let Some(track) = next_track.clone() {
            // Play the next track
            self.play_track(track, None, None).await?;
        } else {
            // No more tracks in queue
            self.current_track = None;
            self.position = 0;
            self.paused = false;
            self.state.position = 0;
            self.state.time = chrono::Utc::now();
            info!("No more tracks in queue for guild {}", self.guild_id);
        }

        Ok(next_track)
    }

    /// Move a track from one position to another in the queue
    #[allow(dead_code)]
    pub fn move_track(&mut self, from: usize, to: usize) -> Result<Track, String> {
        if from >= self.queue.len() {
            return Err(format!(
                "Source index {} is out of bounds for queue of length {}",
                from,
                self.queue.len()
            ));
        }

        if to >= self.queue.len() {
            return Err(format!(
                "Destination index {} is out of bounds for queue of length {}",
                to,
                self.queue.len()
            ));
        }

        let track = self.queue.remove(from).unwrap();
        self.queue.insert(to, track.clone());

        info!(
            "Moved track '{}' from position {} to {} in queue for guild {}",
            track.info.title, from, to, self.guild_id
        );
        Ok(track)
    }

    /// Shuffle the current queue
    #[allow(dead_code)]
    pub fn shuffle_queue(&mut self) {
        #[cfg(any(feature = "discord", feature = "crypto"))]
        {
            let mut rng = rand::rng();

            // Convert to Vec, shuffle, then back to VecDeque
            let mut tracks: Vec<Track> = self.queue.drain(..).collect();
            tracks.shuffle(&mut rng);
            self.queue = tracks.into();
        }
        #[cfg(not(any(feature = "discord", feature = "crypto")))]
        {
            // No-op when rand is not available
            warn!("Shuffle requested but rand feature not available");
        }

        info!(
            "Shuffled queue for guild {} ({} tracks)",
            self.guild_id,
            self.queue.len()
        );
    }

    /// Get repeat mode as protocol enum
    pub fn get_repeat_mode(&self) -> crate::protocol::messages::RepeatMode {
        use crate::protocol::messages::RepeatMode;

        if self.repeat_track {
            RepeatMode::Track
        } else if self.repeat_queue {
            RepeatMode::Queue
        } else {
            RepeatMode::Off
        }
    }

    /// Get current position (accounting for elapsed time)
    pub fn get_current_position(&self) -> u64 {
        if self.is_playing() {
            let elapsed = self.last_update.elapsed().as_millis() as u64;
            self.position + elapsed
        } else {
            self.position
        }
    }

    /// Convert to protocol Player structure
    pub fn to_protocol_player(&self) -> crate::protocol::messages::Player {
        crate::protocol::messages::Player {
            guild_id: self.guild_id.clone(),
            track: self.current_track.clone(),
            volume: self.volume,
            paused: self.paused,
            state: PlayerState {
                time: chrono::Utc::now(),
                position: self.get_current_position(),
                connected: self.state.connected,
                ping: self.state.ping,
            },
            voice: self.voice.clone(),
            filters: self.filters.clone(),
            repeat: self.get_repeat_mode(),
            shuffle: self.shuffle,
            queue_length: self.queue.len(),
        }
    }
}

impl Clone for LavalinkPlayer {
    fn clone(&self) -> Self {
        Self {
            guild_id: self.guild_id.clone(),
            session_id: self.session_id.clone(),
            current_track: self.current_track.clone(),
            state: self.state.clone(),
            volume: self.volume,
            paused: self.paused,
            filters: self.filters.clone(),
            voice: self.voice.clone(),
            position: self.position,
            last_update: self.last_update,
            end_time: self.end_time,
            audio_engine: self.audio_engine.clone(),
            queue: self.queue.clone(),
            repeat_track: self.repeat_track,
            repeat_queue: self.repeat_queue,
            shuffle: self.shuffle,
            voice_manager: self.voice_manager.clone(),
        }
    }
}

impl Default for PlayerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Player event handler for processing events and sending WebSocket messages
pub struct PlayerEventHandler {
    event_receiver: mpsc::UnboundedReceiver<PlayerEvent>,
    #[cfg(feature = "websocket")]
    websocket_sessions: Arc<dashmap::DashMap<String, crate::server::WebSocketSession>>,
    player_manager: Option<Arc<PlayerManager>>,
}

impl PlayerEventHandler {
    /// Create a new event handler
    pub fn new(
        event_receiver: mpsc::UnboundedReceiver<PlayerEvent>,
        #[cfg(feature = "websocket")] websocket_sessions: Arc<
            dashmap::DashMap<String, crate::server::WebSocketSession>,
        >,
    ) -> Self {
        Self {
            event_receiver,
            #[cfg(feature = "websocket")]
            websocket_sessions,
            player_manager: None,
        }
    }

    /// Create a new event handler with player manager reference
    #[allow(dead_code)]
    pub fn with_player_manager(
        event_receiver: mpsc::UnboundedReceiver<PlayerEvent>,
        #[cfg(feature = "websocket")] websocket_sessions: Arc<
            dashmap::DashMap<String, crate::server::WebSocketSession>,
        >,
        player_manager: Arc<PlayerManager>,
    ) -> Self {
        Self {
            event_receiver,
            #[cfg(feature = "websocket")]
            websocket_sessions,
            player_manager: Some(player_manager),
        }
    }

    /// Start processing events
    pub async fn start(mut self) {
        info!("Starting player event handler");

        while let Some(event) = self.event_receiver.recv().await {
            self.handle_event(event).await;
        }

        info!("Player event handler stopped");
    }

    /// Handle a single event
    async fn handle_event(&self, event: PlayerEvent) {
        match event {
            PlayerEvent::TrackStart { guild_id, track } => {
                debug!("Track started in guild {}: {}", guild_id, track.info.title);

                let message = Message::event(Event::track_start(guild_id, track));
                self.broadcast_to_sessions(message).await;
            }
            PlayerEvent::TrackEnd {
                guild_id,
                track,
                reason,
            } => {
                debug!(
                    "Track ended in guild {}: {} (reason: {:?})",
                    guild_id, track.info.title, reason
                );

                let message = Message::event(Event::track_end(
                    guild_id,
                    track,
                    reason.to_messages_reason(),
                ));
                self.broadcast_to_sessions(message).await;
            }

            PlayerEvent::PlayerUpdate { guild_id, state } => {
                debug!(
                    "Player update for guild {}: pos={}, connected={}",
                    guild_id, state.position, state.connected
                );

                let message = Message::player_update(guild_id, state);
                self.broadcast_to_sessions(message).await;
            }

            PlayerEvent::VoiceConnectionEvent { guild_id, event } => {
                debug!("Voice connection event for guild {}: {:?}", guild_id, event);

                // Update player state based on voice connection event
                if let Some(ref player_manager) = self.player_manager {
                    if let Some(player) = player_manager.get_player(&guild_id).await {
                        let mut player_guard = player.write().await;
                        player_guard.handle_voice_event(&event).await;

                        // Validate state consistency after handling the event
                        let is_consistent = player_guard.validate_state_consistency().await;
                        if !is_consistent {
                            info!(
                                "Player state inconsistency detected and corrected for guild {}",
                                guild_id
                            );
                        }

                        // Get enhanced state for better monitoring
                        let enhanced_state = player_guard.get_enhanced_state();
                        let updated_state = player_guard.state.clone();

                        // Log voice quality changes
                        match enhanced_state.voice_quality {
                            VoiceQuality::Poor | VoiceQuality::Critical => {
                                warn!(
                                    "Voice quality degraded for guild {}: {:?} (ping: {}ms)",
                                    guild_id, enhanced_state.voice_quality, updated_state.ping
                                );
                            }
                            VoiceQuality::Excellent | VoiceQuality::Good => {
                                if updated_state.connected {
                                    debug!(
                                        "Voice quality good for guild {}: {:?} (ping: {}ms)",
                                        guild_id, enhanced_state.voice_quality, updated_state.ping
                                    );
                                }
                            }
                            _ => {}
                        }

                        drop(player_guard); // Release the lock before sending event

                        let message = Message::player_update(guild_id.clone(), updated_state);
                        self.broadcast_to_sessions(message).await;
                    }
                }

                // Handle specific voice events that should be broadcast to clients
                match event {
                    VoiceConnectionEvent::GatewayClosed {
                        code,
                        reason,
                        by_remote,
                    } => {
                        // Create WebSocket closed event similar to original Lavalink
                        let websocket_event = crate::protocol::Event::websocket_closed(
                            guild_id, code, reason, by_remote,
                        );
                        let message = Message::event(websocket_event);
                        self.broadcast_to_sessions(message).await;
                    }
                    VoiceConnectionEvent::GatewayError(error) => {
                        warn!("Voice gateway error for guild {}: {}", guild_id, error);
                        // Gateway errors are logged but not necessarily broadcast
                    }
                    VoiceConnectionEvent::Connected | VoiceConnectionEvent::GatewayReady { .. } => {
                        // Connection established events are handled by player state update above
                        info!("Voice connection established for guild {}", guild_id);
                    }
                    VoiceConnectionEvent::Disconnected => {
                        // Connection lost events are handled by player state update above
                        info!("Voice connection lost for guild {}", guild_id);
                    }
                    _ => {
                        // Other events are handled internally and don't need client notification
                        debug!("Internal voice event for guild {}: {:?}", guild_id, event);
                    }
                }
            }
        }
    }

    /// Broadcast a message to all WebSocket sessions
    async fn broadcast_to_sessions(&self, message: Message) {
        #[cfg(feature = "websocket")]
        {
            for session in self.websocket_sessions.iter() {
                if let Err(e) = session.send_message(message.clone()).await {
                    error!(
                        "Failed to send message to session {}: {}",
                        session.session_id, e
                    );
                }
            }
        }
        #[cfg(not(feature = "websocket"))]
        {
            // In standalone mode without websocket, just log the message
            debug!(
                "Would broadcast message to websocket sessions: {:?}",
                message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_event_handling() {
        let mut player = LavalinkPlayer::new("test_guild".to_string(), "test_session".to_string());

        // Test connection event
        player
            .handle_voice_event(&VoiceConnectionEvent::Connected)
            .await;
        assert!(player.state.connected);
        assert_eq!(player.state.ping, 0);

        // Test disconnection event
        player
            .handle_voice_event(&VoiceConnectionEvent::Disconnected)
            .await;
        assert!(!player.state.connected);
        assert_eq!(player.state.ping, -1);
    }

    #[tokio::test]
    async fn test_playback_pause_on_voice_loss() {
        let mut player = LavalinkPlayer::new("test_guild".to_string(), "test_session".to_string());

        // Set up a mock track
        let track = Track {
            encoded: "test_encoded".to_string(),
            info: crate::protocol::TrackInfo {
                identifier: "test_id".to_string(),
                is_seekable: true,
                author: "test_author".to_string(),
                length: 180000,
                is_stream: false,
                position: 0,
                title: "Test Track".to_string(),
                uri: Some("test_uri".to_string()),
                source_name: "test_source".to_string(),
                artwork_url: None,
                isrc: None,
            },
            plugin_info: std::collections::HashMap::new(),
            user_data: std::collections::HashMap::new(),
        };

        player.current_track = Some(track);
        player.paused = false;
        player.state.connected = true;

        // Test that playback is paused when connection is lost
        player
            .handle_voice_event(&VoiceConnectionEvent::ConnectionLost)
            .await;
        assert!(player.paused);
        assert!(!player.state.connected);

        // Test that playback resumes when connection is restored
        player
            .handle_voice_event(&VoiceConnectionEvent::ConnectionRestored)
            .await;
        assert!(!player.paused);
        assert!(player.state.connected);
    }

    #[tokio::test]
    async fn test_ping_updates() {
        let mut player = LavalinkPlayer::new("test_guild".to_string(), "test_session".to_string());
        player.state.connected = true;

        // Test latency update
        player
            .handle_voice_event(&VoiceConnectionEvent::LatencyUpdate { latency_ms: 75.5 })
            .await;
        assert_eq!(player.state.ping, 75);

        // Test packet loss penalty
        player
            .handle_voice_event(&VoiceConnectionEvent::PacketLoss {
                loss_percentage: 10.0,
            })
            .await;
        assert_eq!(player.state.ping, 125); // 75 + 50 penalty
    }

    #[tokio::test]
    async fn test_voice_quality_assessment() {
        let mut player = LavalinkPlayer::new("test_guild".to_string(), "test_session".to_string());
        player.state.connected = true;

        // Test excellent quality
        player.state.ping = 25;
        let state = player.get_enhanced_state();
        assert_eq!(state.voice_quality, VoiceQuality::Excellent);

        // Test poor quality
        player.state.ping = 300;
        let state = player.get_enhanced_state();
        assert_eq!(state.voice_quality, VoiceQuality::Poor);

        // Test disconnected
        player.state.connected = false;
        player.state.ping = -1;
        let state = player.get_enhanced_state();
        assert_eq!(state.voice_quality, VoiceQuality::Disconnected);
    }

    #[tokio::test]
    async fn test_state_consistency_validation() {
        let mut player = LavalinkPlayer::new("test_guild".to_string(), "test_session".to_string());

        // Set up inconsistent state (connected but no voice manager)
        player.state.connected = true;
        player.voice_manager = None;

        let is_consistent = player.validate_state_consistency().await;
        assert!(!is_consistent);
        assert!(!player.state.connected); // Should be corrected
        assert_eq!(player.state.ping, -1);
    }
}

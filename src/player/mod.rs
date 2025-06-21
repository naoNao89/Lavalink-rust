// Player management module
// This will handle audio players for Discord guilds

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration, Instant};

use tracing::{debug, error, info, warn};

use crate::protocol::{messages::VoiceState, Event, Filters, Message, PlayerState, Track};

pub mod engine;
pub use engine::AudioPlayerEngine;

/// Player manager for handling audio players across guilds
pub struct PlayerManager {
    players: Arc<RwLock<HashMap<String, Arc<RwLock<LavalinkPlayer>>>>>,
    event_sender: Option<mpsc::UnboundedSender<PlayerEvent>>,
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
}

/// Events that can be emitted by players
#[derive(Debug, Clone, serde::Serialize)]
pub enum PlayerEvent {
    TrackStart {
        guild_id: String,
        track: Track,
    },
    TrackEnd {
        guild_id: String,
        track: Track,
        reason: TrackEndReason,
    },

    PlayerUpdate {
        guild_id: String,
        state: PlayerState,
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

impl PlayerManager {
    /// Create a new player manager
    pub fn new() -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            event_sender: None,
        }
    }

    /// Create a new player manager with event sender
    pub fn with_event_sender(event_sender: mpsc::UnboundedSender<PlayerEvent>) -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            event_sender: Some(event_sender),
        }
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
    pub async fn emit_event(&self, event: PlayerEvent) {
        if let Some(ref sender) = self.event_sender {
            if let Err(e) = sender.send(event) {
                error!("Failed to send player event: {}", e);
            }
        }
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
                connected: false,
                ping: -1,
            },
            position: 0,
            last_update: Instant::now(),
            end_time: None,
            audio_engine: None,
            queue: VecDeque::new(),
            repeat_track: false,
            repeat_queue: false,
            shuffle: false,
        }
    }

    /// Initialize the audio engine for this player
    pub fn initialize_audio_engine(&mut self, event_sender: mpsc::UnboundedSender<PlayerEvent>) {
        self.audio_engine = Some(Arc::new(AudioPlayerEngine::new(
            self.guild_id.clone(),
            event_sender,
        )));
    }

    /// Play a track
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
                return Err(format!("Audio playback failed: {}", e).into());
            }
        } else {
            warn!("No audio engine available for guild {}", self.guild_id);
        }

        Ok(())
    }

    /// Apply filters
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
    fn get_disabled_filters(&self) -> Vec<String> {
        // TODO: Read from actual configuration
        // For now, return empty list (all filters enabled)
        vec![]
    }

    /// Get current filters
    pub fn get_filters(&self) -> &Filters {
        &self.filters
    }



    /// Check if the player is playing
    pub fn is_playing(&self) -> bool {
        self.current_track.is_some() && !self.paused
    }

    /// Add a track to the queue
    pub fn add_to_queue(&mut self, track: Track) {
        info!(
            "Adding track '{}' to queue for guild {}",
            track.info.title, self.guild_id
        );
        self.queue.push_back(track);
    }

    /// Remove a track from the queue by index
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
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            let indices: Vec<usize> = (0..self.queue.len()).collect();
            if let Some(&random_index) = indices.choose(&mut rng) {
                return self.queue.remove(random_index);
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
    pub fn get_queue(&self) -> Vec<Track> {
        self.queue.iter().cloned().collect()
    }

    /// Get queue length
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Skip to the next track in the queue
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
    pub fn shuffle_queue(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();

        // Convert to Vec, shuffle, then back to VecDeque
        let mut tracks: Vec<Track> = self.queue.drain(..).collect();
        tracks.shuffle(&mut rng);
        self.queue = tracks.into();

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
    websocket_sessions: Arc<dashmap::DashMap<String, crate::server::WebSocketSession>>,
}

impl PlayerEventHandler {
    /// Create a new event handler
    pub fn new(
        event_receiver: mpsc::UnboundedReceiver<PlayerEvent>,
        websocket_sessions: Arc<dashmap::DashMap<String, crate::server::WebSocketSession>>,
    ) -> Self {
        Self {
            event_receiver,
            websocket_sessions,
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

                let message = Message::event(Event::track_end(guild_id, track, reason));
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
        }
    }

    /// Broadcast a message to all WebSocket sessions
    async fn broadcast_to_sessions(&self, message: Message) {
        for session in self.websocket_sessions.iter() {
            if let Err(e) = session.send_message(message.clone()).await {
                error!(
                    "Failed to send message to session {}: {}",
                    session.session_id, e
                );
            }
        }
    }
}

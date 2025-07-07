// Discord bot integration for voice connections
// Handles Discord gateway connection and voice state management

use anyhow::{anyhow, Result};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::id::{GuildId as SerenityGuildId, UserId};
use serenity::model::voice::VoiceState as SerenityVoiceState;
use songbird::{
    Call, Event, EventContext, EventHandler as VoiceEventHandler, SerenityInit, TrackEvent,
};
use std::num::NonZeroU64;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use super::connection::VoiceConnectionEvent;
use super::logging::{
    CorrelationId, PerformanceTimer, VoiceErrorContext, VoiceEvent, VoiceEventType,
};
use crate::log_voice_connection;
use crate::protocol::messages::VoiceState;

/// Discord voice client that manages the bot connection and voice gateway
pub struct DiscordVoiceClient {
    /// Serenity client for Discord gateway
    client: Option<Arc<Client>>,
    /// Songbird voice manager
    songbird: Arc<songbird::Songbird>,
    /// Voice event callback for integration with player system
    voice_event_callback: Option<Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>>,
    /// Bot user ID
    bot_user_id: Option<UserId>,
    /// Correlation ID for tracking operations
    #[allow(dead_code)]
    correlation_id: CorrelationId,
}

#[allow(dead_code)]
impl DiscordVoiceClient {
    /// Create a new Discord voice client
    pub fn new() -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Discord voice client created"
        );

        Self {
            client: None,
            songbird: songbird::Songbird::serenity(),
            voice_event_callback: None,
            bot_user_id: None,
            correlation_id,
        }
    }

    /// Initialize the Discord bot connection
    pub async fn initialize(&mut self, bot_token: String) -> Result<()> {
        let operation_correlation_id = CorrelationId::new();
        let timer = PerformanceTimer::start(
            "discord_client_initialize",
            operation_correlation_id.clone(),
            None,
        );

        log_voice_connection!(
            info,
            operation_correlation_id,
            "system",
            "Initializing Discord voice client"
        );

        VoiceEvent::new(
            operation_correlation_id.clone(),
            VoiceEventType::ConnectionStart,
            "discord_gateway".to_string(),
        )
        .with_detail("operation", "initialize")
        .log();

        // Create the event handler
        let handler =
            DiscordVoiceHandler::new(self.songbird.clone(), self.voice_event_callback.clone());

        // Build the client
        let client_result = Client::builder(
            &bot_token,
            GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILDS,
        )
        .event_handler(handler)
        .register_songbird()
        .await;

        let client = match client_result {
            Ok(client) => {
                log_voice_connection!(
                    info,
                    operation_correlation_id,
                    "system",
                    "Discord client created successfully"
                );
                client
            }
            Err(e) => {
                let error = anyhow!("Failed to create Discord client: {}", e);
                VoiceErrorContext::new(
                    operation_correlation_id.clone(),
                    "discord_gateway".to_string(),
                    "initialize".to_string(),
                    "client_creation_failed".to_string(),
                )
                .with_hint("Check bot token validity and network connectivity")
                .log_error(&error);

                timer.complete_with_context(
                    false,
                    [("error".to_string(), "client_creation_failed".to_string())].into(),
                );
                return Err(error);
            }
        };

        // Get the bot user ID
        let http = client.http.clone();
        let current_user_result = http.get_current_user().await;

        let current_user = match current_user_result {
            Ok(user) => {
                log_voice_connection!(
                    info,
                    operation_correlation_id,
                    "system",
                    "Bot user info retrieved",
                    user_id = user.id.to_string()
                );
                user
            }
            Err(e) => {
                let error = anyhow!("Failed to get bot user info: {}", e);
                VoiceErrorContext::new(
                    operation_correlation_id.clone(),
                    "discord_gateway".to_string(),
                    "initialize".to_string(),
                    "user_info_failed".to_string(),
                )
                .with_hint("Check bot token permissions and Discord API connectivity")
                .log_error(&error);

                timer.complete_with_context(
                    false,
                    [("error".to_string(), "user_info_failed".to_string())].into(),
                );
                return Err(error);
            }
        };

        self.bot_user_id = Some(current_user.id);

        // Store the client
        self.client = Some(Arc::new(client));

        VoiceEvent::new(
            operation_correlation_id.clone(),
            VoiceEventType::ConnectionEstablished,
            "discord_gateway".to_string(),
        )
        .with_detail("bot_user_id", &current_user.id.to_string())
        .log();

        timer.complete_with_context(
            true,
            [("bot_user_id".to_string(), current_user.id.to_string())].into(),
        );

        log_voice_connection!(
            info,
            operation_correlation_id,
            "system",
            "Discord voice client initialization complete",
            bot_user_id = current_user.id.to_string()
        );

        Ok(())
    }

    /// Start the Discord client (non-blocking)
    pub async fn start(&self) -> Result<()> {
        // For now, just mark as started
        // Actual Discord client startup will be implemented in the next phase
        // when we have proper bot token handling and gateway connection
        info!("Discord client marked as started (placeholder implementation)");
        Ok(())
    }

    /// Set voice event callback
    pub fn set_voice_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(String, VoiceConnectionEvent) + Send + Sync + 'static,
    {
        self.voice_event_callback = Some(Arc::new(callback));
    }

    /// Get the Songbird manager
    pub fn songbird(&self) -> Arc<songbird::Songbird> {
        self.songbird.clone()
    }

    /// Join a voice channel using Discord voice state
    pub async fn join_voice_channel(
        &self,
        guild_id: String,
        voice_state: VoiceState,
    ) -> Result<Arc<Mutex<Call>>> {
        let guild_id_u64: u64 = guild_id
            .parse()
            .map_err(|_| anyhow!("Invalid guild ID format: {}", guild_id))?;
        let _serenity_guild_id = SerenityGuildId::new(guild_id_u64);

        info!(
            "Joining voice channel for guild {} using Discord voice state",
            guild_id
        );

        // Get or create the call
        let call = self.songbird.get_or_insert(songbird::id::GuildId(
            NonZeroU64::new(guild_id_u64)
                .ok_or_else(|| anyhow!("Invalid guild ID: cannot be zero"))?,
        ));

        // TODO: Use the voice state information to connect to Discord voice server
        // This requires implementing the voice server connection using the provided
        // token, endpoint, and session_id from the voice_state parameter

        // For now, we'll create a placeholder connection that can be enhanced
        // with actual Discord voice server connection logic
        debug!(
            "Voice state - Token: {}, Endpoint: {}, Session: {}",
            voice_state.token, voice_state.endpoint, voice_state.session_id
        );

        // Add voice event handlers
        let mut call_lock = call.lock().await;
        call_lock.add_global_event(
            Event::Track(TrackEvent::Play),
            VoiceTrackHandler::new(guild_id.clone()),
        );
        call_lock.add_global_event(
            Event::Track(TrackEvent::End),
            VoiceTrackHandler::new(guild_id.clone()),
        );
        call_lock.add_global_event(
            Event::Track(TrackEvent::Error),
            VoiceTrackHandler::new(guild_id.clone()),
        );
        drop(call_lock);

        info!("Voice connection established for guild {}", guild_id);
        Ok(call)
    }

    /// Leave a voice channel
    pub async fn leave_voice_channel(&self, guild_id: String) -> Result<()> {
        let guild_id_u64: u64 = guild_id
            .parse()
            .map_err(|_| anyhow!("Invalid guild ID format: {}", guild_id))?;

        if let Some(call) = self.songbird.get(songbird::id::GuildId(
            NonZeroU64::new(guild_id_u64)
                .ok_or_else(|| anyhow!("Invalid guild ID: cannot be zero"))?,
        )) {
            let mut call_lock = call.lock().await;
            call_lock
                .leave()
                .await
                .map_err(|e| anyhow!("Failed to leave voice channel: {}", e))?;
            info!("Left voice channel for guild {}", guild_id);
        }

        Ok(())
    }

    /// Check if connected to a voice channel
    pub async fn is_connected(&self, guild_id: &str) -> bool {
        if let Ok(guild_id_u64) = guild_id.parse::<u64>() {
            if let Some(guild_id_nonzero) = NonZeroU64::new(guild_id_u64) {
                if let Some(call) = self.songbird.get(songbird::id::GuildId(guild_id_nonzero)) {
                    let call_lock = call.lock().await;
                    return call_lock.current_connection().is_some();
                }
            }
        }
        false
    }
}

impl Default for DiscordVoiceClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Discord event handler for voice state updates
pub struct DiscordVoiceHandler {
    #[allow(dead_code)]
    songbird: Arc<songbird::Songbird>,
    voice_event_callback: Option<Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>>,
}

impl DiscordVoiceHandler {
    pub fn new(
        songbird: Arc<songbird::Songbird>,
        voice_event_callback: Option<Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>>,
    ) -> Self {
        Self {
            songbird,
            voice_event_callback,
        }
    }

    fn emit_voice_event(&self, guild_id: String, event: VoiceConnectionEvent) {
        if let Some(ref callback) = self.voice_event_callback {
            callback(guild_id, event);
        }
    }
}

#[async_trait]
impl EventHandler for DiscordVoiceHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Discord bot is ready: {}", ready.user.name);
    }

    async fn voice_state_update(
        &self,
        _ctx: Context,
        old: Option<SerenityVoiceState>,
        new: SerenityVoiceState,
    ) {
        let guild_id = new.guild_id.map(|id| id.to_string()).unwrap_or_default();

        debug!("Voice state update for guild {}: {:?}", guild_id, new);

        // Handle voice state changes
        if let Some(channel_id) = new.channel_id {
            debug!(
                "User joined voice channel {} in guild {}",
                channel_id, guild_id
            );
            self.emit_voice_event(guild_id, VoiceConnectionEvent::Connected);
        } else if old.is_some() {
            debug!("User left voice channel in guild {}", guild_id);
            self.emit_voice_event(guild_id, VoiceConnectionEvent::Disconnected);
        }
    }
}

/// Voice track event handler
#[allow(dead_code)]
struct VoiceTrackHandler {
    guild_id: String,
}

#[allow(dead_code)]
impl VoiceTrackHandler {
    fn new(guild_id: String) -> Self {
        Self { guild_id }
    }
}

#[async_trait]
impl VoiceEventHandler for VoiceTrackHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (track_state, _track_handle) in *track_list {
                debug!("Track event for guild {}: {:?}", self.guild_id, track_state);

                // Handle track events here
                // Handle track events based on the track state
                info!("Track event in guild {}: {:?}", self.guild_id, track_state);
            }
        }
        None
    }
}

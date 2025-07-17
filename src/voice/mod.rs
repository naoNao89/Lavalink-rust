// Voice integration module
// Handles voice connections with optional Discord integration

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(feature = "discord")]
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::protocol::messages::VoiceState;
use crate::voice::connection::VoiceConnectionEvent;
use crate::voice::koe::MediaConnectionTrait;

pub mod connection;
#[cfg(feature = "discord")]
pub mod discord;
pub mod koe;
pub mod koe_config;
pub mod logging;
pub mod monitoring;
pub mod pool;

#[cfg(test)]
mod voice_connection_tests;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod recovery_tests;

pub use connection::VoiceConnectionManager;
// Note: These exports are available for future use but currently unused
// pub use discord::{DiscordVoiceHandler, DiscordVoiceClient};
// pub use logging::{CorrelationId, PerformanceTimer, VoiceEvent, VoiceEventType, VoiceErrorContext};
// pub use monitoring::{VoiceConnectionMonitor, MonitoringConfig, HealthStatus, MonitoringAlert, MonitoringSummary};
// Note: Pool types are available but not currently used in the public API
// pub use pool::{VoiceConnectionPool, ConnectionPoolConfig, ConnectionMetrics};

/// Voice client wrapper that supports both Discord and standalone voice connections
pub struct VoiceClient {
    /// Songbird voice manager (Discord mode only)
    #[cfg(feature = "discord")]
    songbird: Option<Arc<songbird::Songbird>>,
    /// Active voice connections per guild
    #[allow(dead_code)]
    connections: Arc<RwLock<HashMap<String, VoiceConnectionType>>>,
    /// Connection pool for managing multiple servers
    #[allow(dead_code)]
    connection_pool: Option<Arc<pool::VoiceConnectionPool>>,
    /// Discord voice client for actual Discord integration
    #[cfg(feature = "discord")]
    discord_client: Option<Arc<RwLock<discord::DiscordVoiceClient>>>,
    /// Koe client for non-Discord operation (standalone)
    koe_client: Arc<RwLock<koe::KoeClient>>,
    /// Voice mode (Discord or Standalone)
    mode: VoiceMode,
}

/// Voice connection type enum to support both Discord and standalone connections
#[derive(Clone)]
pub enum VoiceConnectionType {
    #[cfg(feature = "discord")]
    Discord(Arc<Mutex<songbird::Call>>),
    Standalone(Arc<koe::MediaConnection>),
}

/// Voice mode enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiceMode {
    #[cfg(feature = "discord")]
    Discord,
    Standalone,
}

#[allow(dead_code)]
impl VoiceClient {
    /// Create a new voice client in standalone mode
    pub fn new() -> Self {
        Self::new_standalone()
    }

    /// Create a new voice client in standalone mode
    pub fn new_standalone() -> Self {
        info!("Creating voice client in standalone mode");
        Self {
            #[cfg(feature = "discord")]
            songbird: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: None,
            #[cfg(feature = "discord")]
            discord_client: None,
            koe_client: Arc::new(RwLock::new(koe::KoeClient::new())),
            mode: VoiceMode::Standalone,
        }
    }

    /// Create a new voice client in Discord mode (requires discord feature)
    #[cfg(feature = "discord")]
    pub fn new_discord() -> Self {
        info!("Creating voice client in Discord mode");
        Self {
            songbird: Some(songbird::Songbird::serenity()),
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: None,
            discord_client: Some(Arc::new(RwLock::new(discord::DiscordVoiceClient::new()))),
            koe_client: Arc::new(RwLock::new(koe::KoeClient::new())),
            mode: VoiceMode::Discord,
        }
    }

    /// Create a new voice client with connection pooling
    pub fn with_pool(config: pool::ConnectionPoolConfig) -> Self {
        let client = Arc::new(Self::new());
        let pool = Arc::new(pool::VoiceConnectionPool::with_config(
            client.clone(),
            config,
        ));

        Self {
            #[cfg(feature = "discord")]
            songbird: client.songbird.clone(),
            connections: client.connections.clone(),
            connection_pool: Some(pool),
            #[cfg(feature = "discord")]
            discord_client: client.discord_client.clone(),
            koe_client: client.koe_client.clone(),
            mode: client.mode,
        }
    }

    /// Get the Songbird manager (Discord mode only)
    #[cfg(feature = "discord")]
    pub fn songbird(&self) -> Option<Arc<songbird::Songbird>> {
        self.songbird.clone()
    }

    /// Get the current voice mode
    pub fn mode(&self) -> VoiceMode {
        self.mode
    }

    /// Initialize Discord bot connection (Discord mode only)
    #[cfg(feature = "discord")]
    pub async fn initialize_discord(&self, bot_token: String) -> Result<()> {
        if self.mode != VoiceMode::Discord {
            return Err(anyhow::anyhow!(
                "Cannot initialize Discord in standalone mode"
            ));
        }

        if let Some(ref discord_client_arc) = self.discord_client {
            let mut discord_client = discord_client_arc.write().await;
            discord_client.initialize(bot_token).await?;
            discord_client.start().await?;
            info!("Discord voice client initialized and started");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Discord client not available"))
        }
    }

    /// Initialize Discord bot connection (no-op in standalone mode)
    #[cfg(not(feature = "discord"))]
    pub async fn initialize_discord(&self, _bot_token: String) -> Result<()> {
        warn!("Discord initialization requested but discord feature is not enabled");
        warn!("Running in standalone mode - Discord functionality not available");
        Ok(())
    }

    /// Set voice event callback for voice integration
    pub async fn set_voice_event_callback<F>(&self, callback: F)
    where
        F: Fn(String, connection::VoiceConnectionEvent) + Send + Sync + 'static,
    {
        match self.mode {
            #[cfg(feature = "discord")]
            VoiceMode::Discord => {
                if let Some(ref discord_client_arc) = self.discord_client {
                    let mut discord_client = discord_client_arc.write().await;
                    discord_client.set_voice_event_callback(callback);
                }
            }
            VoiceMode::Standalone => {
                let mut koe_client = self.koe_client.write().await;
                // Convert String-based callback to u64-based callback
                let converted_callback = move |guild_id: u64, event: VoiceConnectionEvent| {
                    callback(guild_id.to_string(), event);
                };
                koe_client.set_voice_event_callback(converted_callback);
            }
        }
    }

    /// Join a voice channel
    pub async fn join_channel(
        &self,
        guild_id: String,
        voice_state: VoiceState,
        channel_id: u64,
        user_id: u64,
    ) -> Result<VoiceConnectionType> {
        info!(
            "Joining voice channel for guild {} in {:?} mode",
            guild_id, self.mode
        );

        // Use connection pool if available
        if let Some(ref _pool) = self.connection_pool {
            // Note: Pool needs to be updated to support VoiceConnectionType
            warn!(
                "Connection pool not yet updated for multi-mode support, using direct connection"
            );
        }

        // Use direct connection management
        self.join_channel_direct(guild_id, voice_state, channel_id, user_id)
            .await
    }

    /// Join a voice channel directly (without pool)
    pub async fn join_channel_direct(
        &self,
        guild_id: String,
        voice_state: VoiceState,
        _channel_id: u64,
        _user_id: u64,
    ) -> Result<VoiceConnectionType> {
        // Validate voice state
        if voice_state.token.is_empty()
            || voice_state.endpoint.is_empty()
            || voice_state.session_id.is_empty()
        {
            return Err(anyhow::anyhow!(
                "Invalid voice state: missing required fields"
            ));
        }

        // Parse guild ID
        let _guild_id_u64: u64 = guild_id
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid guild ID format"))?;

        let connection_result = match self.mode {
            #[cfg(feature = "discord")]
            VoiceMode::Discord => {
                if let Some(ref discord_client_arc) = self.discord_client {
                    let discord_client = discord_client_arc.read().await;
                    let call = discord_client
                        .join_voice_channel(guild_id.clone(), voice_state)
                        .await?;
                    VoiceConnectionType::Discord(call)
                } else {
                    return Err(anyhow::anyhow!("Discord client not available"));
                }
            }
            VoiceMode::Standalone => {
                let koe_client = self.koe_client.read().await;
                let connection = koe_client
                    .join_voice_channel(guild_id.clone(), voice_state)
                    .await?;
                VoiceConnectionType::Standalone(connection)
            }
        };

        // Store the connection
        let mut connections = self.connections.write().await;
        connections.insert(guild_id.clone(), connection_result.clone());

        info!("Successfully joined voice channel for guild {}", guild_id);
        Ok(connection_result)
    }

    /// Leave a voice channel
    pub async fn leave_channel(&self, guild_id: &str) -> Result<()> {
        info!(
            "Leaving voice channel for guild {} in {:?} mode",
            guild_id, self.mode
        );

        // Use connection pool if available
        if let Some(ref _pool) = self.connection_pool {
            warn!("Connection pool not yet updated for multi-mode support");
            // return pool.remove_connection(guild_id).await;
        }

        match self.mode {
            #[cfg(feature = "discord")]
            VoiceMode::Discord => {
                if let Some(ref discord_client_arc) = self.discord_client {
                    let discord_client = discord_client_arc.read().await;
                    discord_client
                        .leave_voice_channel(guild_id.to_string())
                        .await?;
                }
            }
            VoiceMode::Standalone => {
                let koe_client = self.koe_client.read().await;
                koe_client.leave_voice_channel(guild_id).await?;
            }
        }

        // Remove from local connections
        let mut connections = self.connections.write().await;
        connections.remove(guild_id);

        info!("Left voice channel for guild {}", guild_id);
        Ok(())
    }

    /// Get an active voice connection
    pub async fn get_connection(&self, guild_id: &str) -> Option<VoiceConnectionType> {
        let connections = self.connections.read().await;
        connections.get(guild_id).cloned()
    }

    /// Check if connected to a voice channel
    pub async fn is_connected(&self, guild_id: &str) -> bool {
        // Use connection pool if available
        if let Some(ref _pool) = self.connection_pool {
            warn!("Connection pool not yet updated for multi-mode support");
            // return pool.is_connected(guild_id).await;
        }

        match self.mode {
            #[cfg(feature = "discord")]
            VoiceMode::Discord => {
                if let Some(ref discord_client_arc) = self.discord_client {
                    let discord_client = discord_client_arc.read().await;
                    discord_client.is_connected(guild_id).await
                } else {
                    false
                }
            }
            VoiceMode::Standalone => {
                let koe_client = self.koe_client.read().await;
                koe_client.is_connected_str(guild_id).await
            }
        }
    }

    /// Get all active connections
    pub async fn get_all_connections(&self) -> Vec<String> {
        // Use connection pool if available
        if let Some(ref pool) = self.connection_pool {
            return pool.get_active_guilds().await;
        }

        // Fallback to direct connection list
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Get connection pool metrics (if pooling is enabled)
    pub async fn get_pool_metrics(&self) -> Option<pool::ConnectionMetrics> {
        if let Some(ref pool) = self.connection_pool {
            Some(pool.get_metrics().await)
        } else {
            None
        }
    }

    /// Cleanup idle connections (if pooling is enabled)
    pub async fn cleanup_idle_connections(&self) -> usize {
        if let Some(ref pool) = self.connection_pool {
            pool.cleanup_idle_connections().await
        } else {
            0
        }
    }

    /// Cleanup all connections
    pub async fn cleanup_all(&self) {
        info!("Cleaning up all voice connections");

        // Use connection pool if available
        if let Some(ref pool) = self.connection_pool {
            pool.cleanup_all().await;
            return;
        }

        // Fallback to direct cleanup
        let mut connections = self.connections.write().await;
        for (guild_id, connection) in connections.drain() {
            match connection {
                #[cfg(feature = "discord")]
                VoiceConnectionType::Discord(call) => {
                    let mut call_guard = call.lock().await;
                    if let Err(e) = call_guard.leave().await {
                        warn!(
                            "Error leaving Discord voice channel for guild {}: {}",
                            guild_id, e
                        );
                    }
                }
                VoiceConnectionType::Standalone(conn) => {
                    if let Err(e) = conn.disconnect().await {
                        warn!(
                            "Error disconnecting standalone voice connection for guild {}: {}",
                            guild_id, e
                        );
                    }
                }
            }
        }

        info!("All voice connections cleaned up");
    }
}

impl Default for VoiceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VoiceClient {
    fn drop(&mut self) {
        // Note: We can't use async in Drop, so cleanup should be called explicitly
        debug!("VoiceClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_client_creation() {
        let voice_client = VoiceClient::new();
        assert!(!voice_client.is_connected("123456789").await);
    }

    #[tokio::test]
    async fn test_voice_state_validation() {
        let voice_client = VoiceClient::new();

        // Test with empty voice state
        let empty_voice_state = VoiceState {
            token: String::new(),
            endpoint: String::new(),
            session_id: String::new(),
        };

        let result = voice_client
            .join_channel("123456789".to_string(), empty_voice_state, 0, 0)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_management() {
        let voice_client = VoiceClient::new();

        // Initially no connections
        assert_eq!(voice_client.get_all_connections().await.len(), 0);

        // Test connection tracking
        assert!(!voice_client.is_connected("123456789").await);
        assert!(voice_client.get_connection("123456789").await.is_none());
    }
}

// Discord voice integration module
// Handles voice connections using Songbird

use anyhow::Result;
use songbird::{Call, Songbird};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::protocol::messages::VoiceState;

pub mod connection;
pub mod discord;
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

/// Voice client wrapper for Discord voice connections
pub struct VoiceClient {
    /// Songbird voice manager
    #[allow(dead_code)]
    songbird: Arc<Songbird>,
    /// Active voice connections per guild
    #[allow(dead_code)]
    connections: Arc<RwLock<HashMap<String, Arc<Mutex<Call>>>>>,
    /// Connection pool for managing multiple servers
    #[allow(dead_code)]
    connection_pool: Option<Arc<pool::VoiceConnectionPool>>,
    /// Discord voice client for actual Discord integration
    discord_client: Arc<RwLock<discord::DiscordVoiceClient>>,
}

#[allow(dead_code)]
impl VoiceClient {
    /// Create a new voice client
    pub fn new() -> Self {
        Self {
            songbird: Songbird::serenity(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: None,
            discord_client: Arc::new(RwLock::new(discord::DiscordVoiceClient::new())),
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
            songbird: client.songbird.clone(),
            connections: client.connections.clone(),
            connection_pool: Some(pool),
            discord_client: client.discord_client.clone(),
        }
    }

    /// Get the Songbird manager
    pub fn songbird(&self) -> Arc<Songbird> {
        self.songbird.clone()
    }

    /// Initialize Discord bot connection
    pub async fn initialize_discord(&self, bot_token: String) -> Result<()> {
        let mut discord_client = self.discord_client.write().await;
        discord_client.initialize(bot_token).await?;
        discord_client.start().await?;
        info!("Discord voice client initialized and started");
        Ok(())
    }

    /// Set voice event callback for Discord integration
    pub async fn set_voice_event_callback<F>(&self, callback: F)
    where
        F: Fn(String, connection::VoiceConnectionEvent) + Send + Sync + 'static,
    {
        let mut discord_client = self.discord_client.write().await;
        discord_client.set_voice_event_callback(callback);
    }

    /// Join a voice channel
    pub async fn join_channel(
        &self,
        guild_id: String,
        voice_state: VoiceState,
        channel_id: u64,
        user_id: u64,
    ) -> Result<Arc<Mutex<Call>>> {
        info!("Joining voice channel for guild {}", guild_id);

        // Use connection pool if available
        if let Some(ref pool) = self.connection_pool {
            return pool.get_connection(guild_id, channel_id, user_id).await;
        }

        // Fallback to direct connection management
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
    ) -> Result<Arc<Mutex<Call>>> {
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

        // Use Discord client for actual voice connection
        let discord_client = self.discord_client.read().await;
        let call = discord_client
            .join_voice_channel(guild_id.clone(), voice_state)
            .await?;

        // Store the connection
        let mut connections = self.connections.write().await;
        connections.insert(guild_id.clone(), call.clone());

        info!("Successfully joined voice channel for guild {}", guild_id);
        Ok(call)
    }

    /// Leave a voice channel
    pub async fn leave_channel(&self, guild_id: &str) -> Result<()> {
        info!("Leaving voice channel for guild {}", guild_id);

        // Use connection pool if available
        if let Some(ref pool) = self.connection_pool {
            return pool.remove_connection(guild_id).await;
        }

        // Use Discord client to leave voice channel
        let discord_client = self.discord_client.read().await;
        discord_client
            .leave_voice_channel(guild_id.to_string())
            .await?;

        // Remove from local connections
        let mut connections = self.connections.write().await;
        connections.remove(guild_id);

        info!("Left voice channel for guild {}", guild_id);
        Ok(())
    }

    /// Get an active voice connection
    pub async fn get_connection(&self, guild_id: &str) -> Option<Arc<Mutex<Call>>> {
        let connections = self.connections.read().await;
        connections.get(guild_id).cloned()
    }

    /// Check if connected to a voice channel
    pub async fn is_connected(&self, guild_id: &str) -> bool {
        // Use connection pool if available
        if let Some(ref pool) = self.connection_pool {
            return pool.is_connected(guild_id).await;
        }

        // Use Discord client to check connection status
        let discord_client = self.discord_client.read().await;
        discord_client.is_connected(guild_id).await
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
        for (guild_id, call) in connections.drain() {
            let mut call_guard = call.lock().await;
            if let Err(e) = call_guard.leave().await {
                warn!("Error leaving voice channel for guild {}: {}", guild_id, e);
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

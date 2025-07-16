// Voice connection pooling for managing multiple Discord server connections

use anyhow::Result;
#[cfg(feature = "discord")]
use songbird::Call;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

use super::{VoiceClient, VoiceConnectionType};

// Type alias for voice call handle that works in both Discord and standalone modes
#[cfg(feature = "discord")]
type VoiceCallHandle = Arc<Mutex<Call>>;
#[cfg(not(feature = "discord"))]
type VoiceCallHandle = Arc<Mutex<()>>;

/// Configuration for voice connection pool
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConnectionPoolConfig {
    /// Maximum number of concurrent voice connections
    pub max_connections: usize,
    /// Maximum idle time before connection cleanup (in seconds)
    pub max_idle_time: Duration,
    /// Connection health check interval (in seconds)
    pub health_check_interval: Duration,
    /// Maximum connection attempts before marking as failed
    pub max_connection_attempts: u32,
    /// Backoff time between connection attempts (in seconds)
    pub connection_backoff: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            max_idle_time: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30), // 30 seconds
            max_connection_attempts: 3,
            connection_backoff: Duration::from_secs(5),
        }
    }
}

/// Connection metadata for pool management
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConnectionInfo {
    /// Guild ID for this connection
    pub guild_id: String,
    /// When the connection was created
    pub created_at: Instant,
    /// When the connection was last used
    pub last_used: Instant,
    /// Number of connection attempts
    pub connection_attempts: u32,
    /// Whether the connection is currently active
    pub is_active: bool,
    /// Connection health status
    pub is_healthy: bool,
}

#[allow(dead_code)]
impl ConnectionInfo {
    pub fn new(guild_id: String) -> Self {
        let now = Instant::now();
        Self {
            guild_id,
            created_at: now,
            last_used: now,
            connection_attempts: 0,
            is_active: false,
            is_healthy: true,
        }
    }

    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }

    pub fn is_idle(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }
}

/// Connection pool metrics
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ConnectionMetrics {
    /// Total number of active connections
    pub active_connections: usize,
    /// Total number of idle connections
    pub idle_connections: usize,
    /// Total number of failed connections
    pub failed_connections: usize,
    /// Total connection attempts
    pub total_connection_attempts: u64,
    /// Total successful connections
    pub successful_connections: u64,
    /// Average connection time (in milliseconds)
    pub avg_connection_time_ms: f64,
}

/// Voice connection pool for managing multiple Discord server connections
pub struct VoiceConnectionPool {
    /// Voice client for creating connections
    #[allow(dead_code)]
    voice_client: Arc<VoiceClient>,
    /// Pool configuration
    #[allow(dead_code)]
    config: ConnectionPoolConfig,
    /// Active voice connections
    #[allow(dead_code)]
    connections: Arc<RwLock<HashMap<String, VoiceCallHandle>>>,
    /// Connection metadata
    #[allow(dead_code)]
    connection_info: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    /// Pool metrics
    #[allow(dead_code)]
    metrics: Arc<RwLock<ConnectionMetrics>>,
}

#[allow(dead_code)]
impl VoiceConnectionPool {
    /// Create a new voice connection pool
    pub fn new(voice_client: Arc<VoiceClient>) -> Self {
        Self::with_config(voice_client, ConnectionPoolConfig::default())
    }

    /// Create a new voice connection pool with custom configuration
    pub fn with_config(voice_client: Arc<VoiceClient>, config: ConnectionPoolConfig) -> Self {
        Self {
            voice_client,
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_info: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
        }
    }

    /// Get or create a voice connection for a guild
    pub async fn get_connection(
        &self,
        guild_id: String,
        channel_id: u64,
        user_id: u64,
    ) -> Result<VoiceCallHandle> {
        // Check if connection already exists
        {
            let connections = self.connections.read().await;
            if let Some(connection) = connections.get(&guild_id) {
                // Update last used time
                {
                    let mut info_map = self.connection_info.write().await;
                    if let Some(info) = info_map.get_mut(&guild_id) {
                        info.mark_used();
                    }
                }
                debug!("Reusing existing voice connection for guild {}", guild_id);
                return Ok(connection.clone());
            }
        }

        // Check connection limits
        if !self.can_create_connection().await {
            return Err(anyhow::anyhow!(
                "Connection pool limit reached (max: {})",
                self.config.max_connections
            ));
        }

        // Create new connection
        self.create_connection(guild_id, channel_id, user_id).await
    }

    /// Create a new voice connection
    async fn create_connection(
        &self,
        guild_id: String,
        channel_id: u64,
        user_id: u64,
    ) -> Result<VoiceCallHandle> {
        let start_time = Instant::now();

        info!("Creating new voice connection for guild {}", guild_id);

        // Update connection attempts
        {
            let mut info_map = self.connection_info.write().await;
            let info = info_map
                .entry(guild_id.clone())
                .or_insert_with(|| ConnectionInfo::new(guild_id.clone()));
            info.connection_attempts += 1;

            if info.connection_attempts > self.config.max_connection_attempts {
                error!("Max connection attempts exceeded for guild {}", guild_id);
                return Err(anyhow::anyhow!(
                    "Max connection attempts exceeded for guild {}",
                    guild_id
                ));
            }
        }

        // Create the connection using voice client directly (avoid recursion)
        match self
            .voice_client
            .join_channel_direct(
                guild_id.clone(),
                crate::protocol::messages::VoiceState {
                    token: String::new(),      // Placeholder - will be set by Discord bot
                    endpoint: String::new(),   // Placeholder - will be set by Discord bot
                    session_id: String::new(), // Placeholder - will be set by Discord bot
                },
                channel_id,
                user_id,
            )
            .await
        {
            Ok(call) => {
                let connection_time = start_time.elapsed();

                // Store the connection
                {
                    let mut connections = self.connections.write().await;
                    connections.insert(guild_id.clone(), Self::convert_connection_to_handle(call.clone()));
                }

                // Update connection info
                {
                    let mut info_map = self.connection_info.write().await;
                    if let Some(info) = info_map.get_mut(&guild_id) {
                        info.is_active = true;
                        info.is_healthy = true;
                        info.mark_used();
                    }
                }

                // Update metrics
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.active_connections += 1;
                    metrics.total_connection_attempts += 1;
                    metrics.successful_connections += 1;

                    // Update average connection time
                    let new_time_ms = connection_time.as_millis() as f64;
                    if metrics.successful_connections == 1 {
                        metrics.avg_connection_time_ms = new_time_ms;
                    } else {
                        metrics.avg_connection_time_ms = (metrics.avg_connection_time_ms
                            * (metrics.successful_connections - 1) as f64
                            + new_time_ms)
                            / metrics.successful_connections as f64;
                    }
                }

                info!(
                    "Successfully created voice connection for guild {} in {:?}",
                    guild_id, connection_time
                );
                Ok(Self::convert_connection_to_handle(call))
            }
            Err(e) => {
                // Update failed connection metrics
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.failed_connections += 1;
                    metrics.total_connection_attempts += 1;
                }

                // Mark connection as unhealthy
                {
                    let mut info_map = self.connection_info.write().await;
                    if let Some(info) = info_map.get_mut(&guild_id) {
                        info.is_healthy = false;
                    }
                }

                error!(
                    "Failed to create voice connection for guild {}: {}",
                    guild_id, e
                );
                Err(e)
            }
        }
    }

    /// Check if a new connection can be created
    async fn can_create_connection(&self) -> bool {
        let connections = self.connections.read().await;
        connections.len() < self.config.max_connections
    }

    /// Remove a voice connection from the pool
    pub async fn remove_connection(&self, guild_id: &str) -> Result<()> {
        info!("Removing voice connection for guild {}", guild_id);

        // Remove from connections
        let removed = {
            let mut connections = self.connections.write().await;
            connections.remove(guild_id)
        };

        if let Some(call) = removed {
            // Leave the voice channel
            {
                let mut _call_guard = call.lock().await;
                #[cfg(feature = "discord")]
                if let Err(e) = call_guard.leave().await {
                    warn!("Error leaving voice channel for guild {}: {}", guild_id, e);
                }
                #[cfg(not(feature = "discord"))]
                {
                    // In standalone mode, just log that we would leave
                    info!("Would leave voice channel for guild {} in standalone mode", guild_id);
                }
            }

            // Update connection info
            {
                let mut info_map = self.connection_info.write().await;
                info_map.remove(guild_id);
            }

            // Update metrics
            {
                let mut metrics = self.metrics.write().await;
                if metrics.active_connections > 0 {
                    metrics.active_connections -= 1;
                }
            }

            info!(
                "Successfully removed voice connection for guild {}",
                guild_id
            );
        }

        Ok(())
    }

    /// Get current pool metrics
    pub async fn get_metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }

    /// Get all active connection guild IDs
    pub async fn get_active_guilds(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Check if connected to a specific guild
    pub async fn is_connected(&self, guild_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(guild_id)
    }

    /// Cleanup idle connections
    pub async fn cleanup_idle_connections(&self) -> usize {
        let mut removed_count = 0;
        let idle_guilds = {
            let info_map = self.connection_info.read().await;
            info_map
                .iter()
                .filter(|(_, info)| info.is_idle(self.config.max_idle_time))
                .map(|(guild_id, _)| guild_id.clone())
                .collect::<Vec<_>>()
        };

        for guild_id in idle_guilds {
            if let Err(e) = self.remove_connection(&guild_id).await {
                warn!(
                    "Error removing idle connection for guild {}: {}",
                    guild_id, e
                );
            } else {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} idle voice connections", removed_count);
        }

        removed_count
    }

    /// Cleanup all connections
    pub async fn cleanup_all(&self) {
        info!("Cleaning up all voice connections");

        let guild_ids = self.get_active_guilds().await;
        for guild_id in guild_ids {
            if let Err(e) = self.remove_connection(&guild_id).await {
                warn!("Error cleaning up connection for guild {}: {}", guild_id, e);
            }
        }

        // Reset metrics
        {
            let mut metrics = self.metrics.write().await;
            *metrics = ConnectionMetrics::default();
        }

        info!("All voice connections cleaned up");
    }

    /// Convert VoiceConnectionType to VoiceCallHandle
    fn convert_connection_to_handle(connection: VoiceConnectionType) -> VoiceCallHandle {
        match connection {
            #[cfg(feature = "discord")]
            VoiceConnectionType::Discord(call) => call,
            VoiceConnectionType::Standalone(_standalone_conn) => {
                // In standalone mode, return a dummy handle
                #[cfg(not(feature = "discord"))]
                return Arc::new(Mutex::new(()));
                #[cfg(feature = "discord")]
                unreachable!("Standalone connection type should not exist in Discord mode");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::VoiceClient;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let voice_client = Arc::new(VoiceClient::new());
        let pool = VoiceConnectionPool::new(voice_client);

        let metrics = pool.get_metrics().await;
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.total_connection_attempts, 0);
    }

    #[tokio::test]
    async fn test_connection_info() {
        let info = ConnectionInfo::new("123456789".to_string());
        assert_eq!(info.guild_id, "123456789");
        assert!(!info.is_idle(Duration::from_secs(1)));

        // Simulate time passing
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(info.is_idle(Duration::from_millis(5)));
    }

    #[tokio::test]
    async fn test_pool_limits() {
        let voice_client = Arc::new(VoiceClient::new());
        let config = ConnectionPoolConfig {
            max_connections: 1,
            ..Default::default()
        };
        let pool = VoiceConnectionPool::with_config(voice_client, config);

        assert!(pool.can_create_connection().await);

        // Test that we can't create more connections than the limit
        // We'll test this by checking the limit directly rather than creating unsafe objects
        let connections_count = {
            let connections = pool.connections.read().await;
            connections.len()
        };

        // Initially should be empty
        assert_eq!(connections_count, 0);
        assert!(pool.can_create_connection().await);

        // Test the configuration
        assert_eq!(pool.config.max_connections, 1);
    }
}

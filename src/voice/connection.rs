// Voice connection management for integrating with player system

use anyhow::Result;
use rand::Rng;
#[cfg(feature = "discord")]
use songbird::{error::ConnectionError as SongbirdConnectionError, Call};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::logging::{
    CorrelationId, PerformanceTimer, VoiceErrorContext, VoiceEvent, VoiceEventLogger,
    VoiceEventType,
};
use super::monitoring::VoiceConnectionMonitor;
use super::{pool, VoiceClient, VoiceConnectionType};
use crate::log_voice_connection;
use crate::protocol::messages::VoiceState;

// Type aliases for voice types that work in both Discord and standalone modes
#[cfg(feature = "discord")]
type VoiceCallHandle = Arc<Mutex<Call>>;
#[cfg(not(feature = "discord"))]
type VoiceCallHandle = Arc<Mutex<()>>;

#[cfg(feature = "discord")]
#[allow(dead_code)]
type VoiceConnectionError = SongbirdConnectionError;
#[cfg(not(feature = "discord"))]
#[allow(dead_code)]
type VoiceConnectionError = anyhow::Error;

/// Event subscription filter for voice connection events
#[derive(Debug, Clone)]
pub struct VoiceEventFilter {
    /// Guild IDs to filter by (None means all guilds)
    pub guild_ids: Option<Vec<String>>,
    /// Event types to include (None means all event types)
    pub event_types: Option<Vec<VoiceConnectionEventType>>,
    /// Minimum severity level
    pub min_severity: Option<EventSeverity>,
    /// Whether to include recovery events
    pub include_recovery: bool,
    /// Whether to include performance events
    pub include_performance: bool,
    /// Whether to include health events
    pub include_health: bool,
}

impl Default for VoiceEventFilter {
    fn default() -> Self {
        Self {
            guild_ids: None,
            event_types: None,
            min_severity: None,
            include_recovery: true,
            include_performance: true,
            include_health: true,
        }
    }
}

impl VoiceEventFilter {}

/// Event severity levels for filtering
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Event type categories for filtering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceConnectionEventType {
    Connection,
    Gateway,
    Audio,
    Performance,
    Health,
    Recovery,
    CircuitBreaker,
    Pool,
    Error,
}

/// Event subscription for voice connection events
pub struct VoiceEventSubscription {
    /// Event filter criteria
    pub filter: VoiceEventFilter,
    /// Callback function for handling events
    pub callback: Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>,
    /// Whether this subscription is active
    pub active: bool,
}

/// Event subscription manager for voice connections
pub struct VoiceEventSubscriptionManager {
    /// Active subscriptions
    subscriptions:
        Arc<tokio::sync::RwLock<std::collections::HashMap<String, VoiceEventSubscription>>>,
    /// Event history for replay
    event_history: Arc<tokio::sync::RwLock<VecDeque<(String, VoiceConnectionEvent, Instant)>>>,
    /// Maximum history size
    max_history_size: usize,
}

impl VoiceEventSubscriptionManager {
    /// Create a new event subscription manager
    pub fn new(max_history_size: usize) -> Self {
        Self {
            subscriptions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_history: Arc::new(tokio::sync::RwLock::new(VecDeque::new())),
            max_history_size,
        }
    }

    /// Subscribe to voice connection events
    pub async fn subscribe<F>(
        &self,
        id: String,
        filter: VoiceEventFilter,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(String, VoiceConnectionEvent) + Send + Sync + 'static,
    {
        let subscription = VoiceEventSubscription {
            filter,
            callback: Arc::new(callback),
            active: true,
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(id, subscription);
        Ok(())
    }

    /// Unsubscribe from voice connection events
    pub async fn unsubscribe(&self, id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(id);
        Ok(())
    }

    /// Publish an event to all matching subscriptions
    pub async fn publish_event(&self, guild_id: String, event: VoiceConnectionEvent) {
        // Add to history
        {
            let mut history = self.event_history.write().await;
            history.push_back((guild_id.clone(), event.clone(), Instant::now()));

            // Trim history if needed
            while history.len() > self.max_history_size {
                history.pop_front();
            }
        }

        // Notify subscribers
        let subscriptions = self.subscriptions.read().await;
        for subscription in subscriptions.values() {
            if subscription.active
                && self.event_matches_filter(&guild_id, &event, &subscription.filter)
            {
                (subscription.callback)(guild_id.clone(), event.clone());
            }
        }
    }

    /// Check if an event matches a filter
    fn event_matches_filter(
        &self,
        guild_id: &str,
        event: &VoiceConnectionEvent,
        filter: &VoiceEventFilter,
    ) -> bool {
        // Check guild ID filter
        if let Some(ref guild_ids) = filter.guild_ids {
            if !guild_ids.contains(&guild_id.to_string()) {
                return false;
            }
        }

        // Check event type filter
        if let Some(ref event_types) = filter.event_types {
            let event_type = self.categorize_event(event);
            if !event_types.contains(&event_type) {
                return false;
            }
        }

        // Check severity filter
        if let Some(ref min_severity) = filter.min_severity {
            let event_severity = self.get_event_severity(event);
            if event_severity < *min_severity {
                return false;
            }
        }

        // Check specific event category filters
        let event_type = self.categorize_event(event);
        match event_type {
            VoiceConnectionEventType::Recovery => filter.include_recovery,
            VoiceConnectionEventType::Performance => filter.include_performance,
            VoiceConnectionEventType::Health => filter.include_health,
            _ => true,
        }
    }

    /// Categorize an event by type
    fn categorize_event(&self, event: &VoiceConnectionEvent) -> VoiceConnectionEventType {
        match event {
            VoiceConnectionEvent::Connected
            | VoiceConnectionEvent::Disconnected
            | VoiceConnectionEvent::Connecting
            | VoiceConnectionEvent::Reconnecting
            | VoiceConnectionEvent::ConnectionTimeout
            | VoiceConnectionEvent::ConnectionLost
            | VoiceConnectionEvent::ConnectionRestored => VoiceConnectionEventType::Connection,

            VoiceConnectionEvent::GatewayReady { .. }
            | VoiceConnectionEvent::GatewayClosed { .. }
            | VoiceConnectionEvent::GatewayError(_)
            | VoiceConnectionEvent::GatewayReconnecting => VoiceConnectionEventType::Gateway,

            VoiceConnectionEvent::AudioStreamStarted
            | VoiceConnectionEvent::AudioStreamStopped
            | VoiceConnectionEvent::AudioStreamPaused
            | VoiceConnectionEvent::AudioStreamResumed
            | VoiceConnectionEvent::AudioQualityChanged { .. } => VoiceConnectionEventType::Audio,

            VoiceConnectionEvent::LatencyUpdate { .. }
            | VoiceConnectionEvent::PacketLoss { .. }
            | VoiceConnectionEvent::JitterUpdate { .. } => VoiceConnectionEventType::Performance,

            VoiceConnectionEvent::HealthCheckPassed
            | VoiceConnectionEvent::HealthCheckFailed { .. }
            | VoiceConnectionEvent::ConnectionDegraded { .. }
            | VoiceConnectionEvent::ConnectionHealthy => VoiceConnectionEventType::Health,

            VoiceConnectionEvent::RecoveryStarted { .. }
            | VoiceConnectionEvent::RecoverySucceeded { .. }
            | VoiceConnectionEvent::RecoveryFailed { .. }
            | VoiceConnectionEvent::RecoveryAborted { .. } => VoiceConnectionEventType::Recovery,

            VoiceConnectionEvent::CircuitBreakerOpened
            | VoiceConnectionEvent::CircuitBreakerClosed
            | VoiceConnectionEvent::CircuitBreakerHalfOpen => {
                VoiceConnectionEventType::CircuitBreaker
            }

            VoiceConnectionEvent::PoolConnectionCreated
            | VoiceConnectionEvent::PoolConnectionDestroyed
            | VoiceConnectionEvent::PoolConnectionReused => VoiceConnectionEventType::Pool,

            VoiceConnectionEvent::Error(_)
            | VoiceConnectionEvent::CriticalError { .. }
            | VoiceConnectionEvent::ErrorRecovered { .. } => VoiceConnectionEventType::Error,

            _ => VoiceConnectionEventType::Connection,
        }
    }

    /// Get the severity level of an event
    fn get_event_severity(&self, event: &VoiceConnectionEvent) -> EventSeverity {
        match event {
            VoiceConnectionEvent::CriticalError { .. } => EventSeverity::Critical,

            VoiceConnectionEvent::Error(_)
            | VoiceConnectionEvent::ConnectionTimeout
            | VoiceConnectionEvent::ConnectionLost
            | VoiceConnectionEvent::RecoveryFailed { .. }
            | VoiceConnectionEvent::CircuitBreakerOpened
            | VoiceConnectionEvent::GatewayError(_)
            | VoiceConnectionEvent::HealthCheckFailed { .. } => EventSeverity::Error,

            VoiceConnectionEvent::RecoveryStarted { .. }
            | VoiceConnectionEvent::ConnectionDegraded { .. }
            | VoiceConnectionEvent::AudioQualityChanged { .. }
            | VoiceConnectionEvent::PacketLoss { .. }
            | VoiceConnectionEvent::RecoveryAborted { .. } => EventSeverity::Warning,

            VoiceConnectionEvent::Connected
            | VoiceConnectionEvent::RecoverySucceeded { .. }
            | VoiceConnectionEvent::CircuitBreakerClosed
            | VoiceConnectionEvent::ConnectionRestored
            | VoiceConnectionEvent::AudioStreamStarted
            | VoiceConnectionEvent::HealthCheckPassed
            | VoiceConnectionEvent::ConnectionHealthy
            | VoiceConnectionEvent::ErrorRecovered { .. } => EventSeverity::Info,

            _ => EventSeverity::Debug,
        }
    }

    /// Get event history for a specific guild
    pub async fn get_event_history(
        &self,
        guild_id: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<(String, VoiceConnectionEvent, Instant)> {
        let history = self.event_history.read().await;
        let mut events: Vec<_> = history.iter().cloned().collect();

        // Filter by guild if specified
        if let Some(guild_id) = guild_id {
            events.retain(|(gid, _, _)| gid == guild_id);
        }

        // Apply limit if specified
        if let Some(limit) = limit {
            events.truncate(limit);
        }

        events
    }
}

/// Voice connection manager that integrates with the player system
pub struct VoiceConnectionManager {
    /// Voice client for Discord connections
    voice_client: Arc<VoiceClient>,
    /// Recovery configuration
    #[allow(dead_code)]
    recovery_config: RecoveryConfig,
    /// Recovery state per guild
    #[allow(dead_code)]
    recovery_states: Arc<tokio::sync::RwLock<std::collections::HashMap<String, RecoveryState>>>,
    /// Event broadcaster for sending events to player system
    event_broadcaster: Option<Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>>,
    /// Voice connection monitor for health checks and metrics
    #[allow(dead_code)]
    monitor: Option<Arc<tokio::sync::RwLock<VoiceConnectionMonitor>>>,
    /// Correlation ID for tracking operations
    #[allow(dead_code)]
    correlation_id: CorrelationId,
    /// Event subscription manager for advanced event handling
    subscription_manager: Arc<VoiceEventSubscriptionManager>,
}

#[allow(dead_code)]
impl VoiceConnectionManager {
    /// Create a new voice connection manager
    pub fn new() -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Voice connection manager created"
        );

        Self {
            voice_client: Arc::new(VoiceClient::new()),
            recovery_config: RecoveryConfig::default(),
            recovery_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_broadcaster: None,
            monitor: None,
            correlation_id,
            subscription_manager: Arc::new(VoiceEventSubscriptionManager::new(1000)), // Keep last 1000 events
        }
    }

    /// Set event handler for voice connection events
    pub fn set_event_broadcaster<F>(&mut self, broadcaster: F)
    where
        F: Fn(String, VoiceConnectionEvent) + Send + Sync + 'static,
    {
        self.event_broadcaster = Some(Arc::new(broadcaster));
    }

    /// Get the event subscription manager
    pub fn subscription_manager(&self) -> Arc<VoiceEventSubscriptionManager> {
        self.subscription_manager.clone()
    }

    /// Subscribe to voice connection events with a filter
    pub async fn subscribe_to_events<F>(
        &self,
        id: String,
        filter: VoiceEventFilter,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(String, VoiceConnectionEvent) + Send + Sync + 'static,
    {
        self.subscription_manager
            .subscribe(id, filter, callback)
            .await
    }

    /// Unsubscribe from voice connection events
    pub async fn unsubscribe_from_events(&self, id: &str) -> Result<()> {
        self.subscription_manager.unsubscribe(id).await
    }

    /// Get event history for a guild
    pub async fn get_event_history(
        &self,
        guild_id: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<(String, VoiceConnectionEvent, Instant)> {
        self.subscription_manager
            .get_event_history(guild_id, limit)
            .await
    }

    /// Create a new voice connection manager with connection pooling
    pub fn with_pool(pool_config: pool::ConnectionPoolConfig) -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Voice connection manager created with pool",
            max_connections = pool_config.max_connections
        );

        Self {
            voice_client: Arc::new(VoiceClient::with_pool(pool_config)),
            recovery_config: RecoveryConfig::default(),
            recovery_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_broadcaster: None,
            monitor: None,
            correlation_id,
            subscription_manager: Arc::new(VoiceEventSubscriptionManager::new(1000)),
        }
    }

    /// Create with existing voice client
    pub fn with_voice_client(voice_client: Arc<VoiceClient>) -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Voice connection manager created with existing client"
        );

        Self {
            voice_client,
            recovery_config: RecoveryConfig::default(),
            recovery_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_broadcaster: None,
            monitor: None,
            correlation_id,
            subscription_manager: Arc::new(VoiceEventSubscriptionManager::new(1000)),
        }
    }

    /// Create a new voice connection manager with custom recovery configuration
    pub fn with_recovery_config(recovery_config: RecoveryConfig) -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Voice connection manager created with recovery config",
            max_retries = recovery_config.max_retries
        );

        Self {
            voice_client: Arc::new(VoiceClient::new()),
            recovery_config,
            recovery_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_broadcaster: None,
            monitor: None,
            correlation_id,
            subscription_manager: Arc::new(VoiceEventSubscriptionManager::new(1000)),
        }
    }

    /// Create a new voice connection manager with both pooling and recovery configuration
    pub fn with_pool_and_recovery(
        pool_config: pool::ConnectionPoolConfig,
        recovery_config: RecoveryConfig,
    ) -> Self {
        let correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            correlation_id,
            "system",
            "Voice connection manager created with pool and recovery",
            max_connections = pool_config.max_connections,
            max_retries = recovery_config.max_retries
        );

        Self {
            voice_client: Arc::new(VoiceClient::with_pool(pool_config)),
            recovery_config,
            recovery_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            event_broadcaster: None,
            monitor: None,
            correlation_id,
            subscription_manager: Arc::new(VoiceEventSubscriptionManager::new(1000)),
        }
    }

    /// Get the voice client
    pub fn voice_client(&self) -> Arc<VoiceClient> {
        self.voice_client.clone()
    }

    /// Enable monitoring for voice connections
    pub async fn enable_monitoring(&mut self, monitor: VoiceConnectionMonitor) {
        self.monitor = Some(Arc::new(tokio::sync::RwLock::new(monitor)));
        info!("Voice connection monitoring enabled");
    }

    /// Disable monitoring for voice connections
    pub async fn disable_monitoring(&mut self) {
        if let Some(monitor) = &self.monitor {
            let mut monitor = monitor.write().await;
            monitor.stop_health_checks().await;
        }
        self.monitor = None;
        info!("Voice connection monitoring disabled");
    }

    /// Get monitoring status for a guild
    pub async fn get_monitoring_status(
        &self,
        guild_id: &str,
    ) -> Option<super::monitoring::HealthCheckResult> {
        if let Some(monitor) = &self.monitor {
            let monitor = monitor.read().await;
            monitor.get_health_status(guild_id).await
        } else {
            None
        }
    }

    /// Get monitoring summary for all guilds
    pub async fn get_monitoring_summary(&self) -> Option<super::monitoring::MonitoringSummary> {
        if let Some(monitor) = &self.monitor {
            let monitor = monitor.read().await;
            Some(monitor.get_monitoring_summary().await)
        } else {
            None
        }
    }

    /// Update voice state and establish connection if needed
    /// Validates voice state according to Lavalink v4 protocol requirements
    pub async fn update_voice_state(
        &self,
        guild_id: String,
        voice_state: VoiceState,
    ) -> Result<Option<VoiceCallHandle>> {
        let operation_correlation_id = CorrelationId::new();
        let timer = PerformanceTimer::start(
            "update_voice_state",
            operation_correlation_id.clone(),
            Some(guild_id.clone()),
        );

        log_voice_connection!(
            info,
            operation_correlation_id,
            guild_id,
            "Starting voice state update",
            endpoint = voice_state.endpoint.as_str(),
            has_token = !voice_state.token.is_empty(),
            session_id = voice_state.session_id.as_str()
        );

        // Create voice event for state update start
        VoiceEvent::new(
            operation_correlation_id.clone(),
            VoiceEventType::ConnectionStart,
            guild_id.clone(),
        )
        .with_detail("endpoint", &voice_state.endpoint)
        .with_detail("session_id", &voice_state.session_id)
        .log();

        // Validate voice state completeness (following original Lavalink logic)
        if voice_state.endpoint.is_empty()
            || voice_state.token.is_empty()
            || voice_state.session_id.is_empty()
        {
            let error_context = VoiceErrorContext::new(
                operation_correlation_id.clone(),
                guild_id.clone(),
                "update_voice_state".to_string(),
                "validation_error".to_string(),
            )
            .with_context("endpoint", &voice_state.endpoint)
            .with_context("has_token", &(!voice_state.token.is_empty()).to_string())
            .with_context("session_id", &voice_state.session_id)
            .with_hint(
                "Ensure Discord sends complete voice state with endpoint, token, and session_id",
            );

            let error = anyhow::anyhow!(
                "Partial Lavalink voice state: endpoint={}, token={}, sessionId={}",
                voice_state.endpoint,
                if voice_state.token.is_empty() {
                    "empty"
                } else {
                    "present"
                },
                voice_state.session_id
            );

            error_context.log_error(&error);
            timer.complete_with_context(
                false,
                [("error".to_string(), "validation_failed".to_string())].into(),
            );
            return Err(error);
        }

        // Check if we need to destroy existing connection (following original Lavalink logic)
        if let Some(_existing_call) = self.voice_client.get_connection(&guild_id).await {
            log_voice_connection!(
                info,
                operation_correlation_id,
                guild_id,
                "Destroying existing connection before creating new one"
            );

            if let Err(e) = self.disconnect_voice_channel(&guild_id).await {
                log_voice_connection!(
                    warn,
                    operation_correlation_id,
                    guild_id,
                    "Error destroying existing connection",
                    error = e.to_string()
                );
            }
        }

        // Connect to voice channel with validated state
        let result = self
            .connect_voice_channel(guild_id.clone(), voice_state)
            .await;

        match &result {
            Ok(Some(_)) => {
                VoiceEvent::new(
                    operation_correlation_id.clone(),
                    VoiceEventType::ConnectionEstablished,
                    guild_id.clone(),
                )
                .log();
                timer.complete_with_context(
                    true,
                    [("result".to_string(), "success".to_string())].into(),
                );
            }
            Ok(None) => {
                log_voice_connection!(
                    warn,
                    operation_correlation_id,
                    guild_id,
                    "Voice connection returned None"
                );
                timer.complete_with_context(
                    false,
                    [("result".to_string(), "none".to_string())].into(),
                );
            }
            Err(e) => {
                VoiceEvent::new(
                    operation_correlation_id.clone(),
                    VoiceEventType::ConnectionFailed,
                    guild_id.clone(),
                )
                .with_detail("error", &e.to_string())
                .log();
                timer.complete_with_context(false, [("error".to_string(), e.to_string())].into());
            }
        }

        result
    }

    /// Connect to a voice channel with automatic retry and recovery
    async fn connect_voice_channel(
        &self,
        guild_id: String,
        voice_state: VoiceState,
    ) -> Result<Option<VoiceCallHandle>> {
        let operation_correlation_id = CorrelationId::new();

        log_voice_connection!(
            info,
            operation_correlation_id,
            guild_id,
            "Starting voice channel connection"
        );

        // Check if already connected
        if self.voice_client.is_connected(&guild_id).await {
            log_voice_connection!(
                debug,
                operation_correlation_id,
                guild_id,
                "Already connected to voice channel"
            );
            // Convert VoiceConnectionType to VoiceCallHandle
            if let Some(connection) = self.voice_client.get_connection(&guild_id).await {
                return Ok(Some(Self::convert_connection_to_handle(connection)));
            } else {
                return Ok(None);
            }
        }

        // Check circuit breaker state
        if self.should_close_circuit_breaker(&guild_id).await {
            log_voice_connection!(
                info,
                operation_correlation_id,
                guild_id,
                "Attempting to close circuit breaker"
            );

            VoiceEvent::new(
                operation_correlation_id.clone(),
                VoiceEventType::CircuitBreakerClosed,
                guild_id.clone(),
            )
            .log();
            self.handle_voice_event(&guild_id, VoiceConnectionEvent::CircuitBreakerClosed)
                .await;
        } else {
            let recovery_states = self.recovery_states.read().await;
            if let Some(state) = recovery_states.get(&guild_id) {
                if state.circuit_breaker_open {
                    log_voice_connection!(
                        warn,
                        operation_correlation_id,
                        guild_id,
                        "Circuit breaker is open, refusing connection"
                    );

                    VoiceEvent::new(
                        operation_correlation_id.clone(),
                        VoiceEventType::CircuitBreakerOpen,
                        guild_id.clone(),
                    )
                    .with_detail("reason", "connection_refused")
                    .log();
                    self.handle_voice_event(&guild_id, VoiceConnectionEvent::CircuitBreakerOpened)
                        .await;

                    let error = anyhow::anyhow!("Circuit breaker is open for guild {}", guild_id);
                    VoiceErrorContext::new(
                        operation_correlation_id,
                        guild_id.clone(),
                        "connect_voice_channel".to_string(),
                        "circuit_breaker_open".to_string(),
                    )
                    .with_hint("Wait for circuit breaker to reset or manually reset it")
                    .with_context(
                        "consecutive_failures",
                        &state.consecutive_failures.to_string(),
                    )
                    .log_error(&error);

                    return Err(error);
                }
            }
        }

        // Attempt connection with retry logic
        self.connect_with_retry(guild_id, voice_state).await
    }

    /// Connect with retry logic and exponential backoff
    async fn connect_with_retry(
        &self,
        guild_id: String,
        voice_state: VoiceState,
    ) -> Result<Option<VoiceCallHandle>> {
        let mut last_error = None;

        for attempt in 0..=self.recovery_config.max_retries {
            // Calculate delay for this attempt (skip delay on first attempt)
            if attempt > 0 {
                let delay = self.calculate_backoff_delay(attempt - 1);
                self.handle_voice_event(
                    &guild_id,
                    VoiceConnectionEvent::RecoveryStarted { attempt, delay },
                )
                .await;

                debug!(
                    "Retrying connection for guild {} (attempt {}/{}) after delay {:?}",
                    guild_id, attempt, self.recovery_config.max_retries, delay
                );
                sleep(delay).await;
            }

            // Attempt the actual connection
            match self
                .attempt_single_connection(&guild_id, &voice_state)
                .await
            {
                Ok(call) => {
                    if attempt > 0 {
                        self.handle_voice_event(
                            &guild_id,
                            VoiceConnectionEvent::RecoverySucceeded {
                                total_attempts: attempt + 1,
                            },
                        )
                        .await;
                    }
                    self.update_recovery_state_on_success(&guild_id).await;

                    // Start monitoring for this guild if monitoring is enabled
                    if let Some(monitor) = &self.monitor {
                        let mut monitor = monitor.write().await;
                        if let Err(e) = monitor.start_monitoring(guild_id.clone()).await {
                            warn!("Failed to start monitoring for guild {}: {}", guild_id, e);
                        }
                    }

                    return Ok(Some(call));
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("{}", e));
                    let error_type = self.classify_error(&e);

                    // Don't retry certain types of errors
                    match error_type {
                        VoiceErrorType::Authentication
                        | VoiceErrorType::Configuration
                        | VoiceErrorType::Permanent => {
                            warn!(
                                "Non-retryable error for guild {}: {:?} - {}",
                                guild_id, error_type, e
                            );
                            self.update_recovery_state_on_failure(&guild_id).await;
                            break;
                        }
                        VoiceErrorType::ResourceExhaustion => {
                            // For resource exhaustion, use longer delays
                            warn!("Resource exhaustion for guild {}, will retry with longer delay: {}", guild_id, e);
                        }
                        VoiceErrorType::Temporary => {
                            debug!("Temporary error for guild {}, will retry: {}", guild_id, e);
                        }
                    }

                    self.update_recovery_state_on_failure(&guild_id).await;

                    // Check if circuit breaker should be opened
                    if self.should_open_circuit_breaker(&guild_id).await {
                        self.handle_voice_event(
                            &guild_id,
                            VoiceConnectionEvent::CircuitBreakerOpened,
                        )
                        .await;
                        break;
                    }
                }
            }
        }

        // All retry attempts failed
        if let Some(error) = last_error {
            self.handle_voice_event(
                &guild_id,
                VoiceConnectionEvent::RecoveryFailed {
                    total_attempts: self.recovery_config.max_retries + 1,
                    error: error.to_string(),
                },
            )
            .await;
            Err(error)
        } else {
            let error = anyhow::anyhow!("Connection failed for unknown reason");
            self.handle_voice_event(
                &guild_id,
                VoiceConnectionEvent::RecoveryFailed {
                    total_attempts: self.recovery_config.max_retries + 1,
                    error: error.to_string(),
                },
            )
            .await;
            Err(error)
        }
    }

    /// Attempt a single connection without retry logic
    async fn attempt_single_connection(
        &self,
        guild_id: &str,
        voice_state: &VoiceState,
    ) -> Result<VoiceCallHandle> {
        // TODO: Get actual channel_id and user_id from Discord bot context
        let channel_id = 0; // Placeholder - should come from Discord bot
        let user_id = 0; // Placeholder - should come from Discord bot

        match self
            .voice_client
            .join_channel(
                guild_id.to_string(),
                voice_state.clone(),
                channel_id,
                user_id,
            )
            .await
        {
            Ok(connection) => {
                info!(
                    "Successfully connected to voice channel for guild {}",
                    guild_id
                );
                Ok(Self::convert_connection_to_handle(connection))
            }
            Err(e) => {
                debug!("Connection attempt failed for guild {}: {}", guild_id, e);
                Err(e)
            }
        }
    }

    /// Disconnect from a voice channel
    async fn disconnect_voice_channel(&self, guild_id: &str) -> Result<()> {
        if self.voice_client.is_connected(guild_id).await {
            info!("Disconnecting from voice channel for guild {}", guild_id);
            self.voice_client.leave_channel(guild_id).await?;
        }

        // Stop monitoring for this guild if monitoring is enabled
        if let Some(monitor) = &self.monitor {
            let mut monitor = monitor.write().await;
            if let Err(e) = monitor.stop_monitoring(guild_id).await {
                warn!("Failed to stop monitoring for guild {}: {}", guild_id, e);
            }
        }

        Ok(())
    }

    /// Get voice connection for a guild
    pub async fn get_voice_connection(&self, guild_id: &str) -> Option<VoiceCallHandle> {
        self.voice_client
            .get_connection(guild_id)
            .await
            .map(Self::convert_connection_to_handle)
    }

    /// Check if connected to voice channel
    pub async fn is_voice_connected(&self, guild_id: &str) -> bool {
        self.voice_client.is_connected(guild_id).await
    }

    /// Get all active voice connections
    pub async fn get_active_connections(&self) -> Vec<String> {
        self.voice_client.get_all_connections().await
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
                unreachable!("Discord connection type should not exist in standalone mode");
            }
        }
    }

    /// Cleanup all voice connections
    pub async fn cleanup_all_connections(&self) {
        info!("Cleaning up all voice connections");
        self.voice_client.cleanup_all().await;
    }

    /// Shutdown the voice connection manager
    pub async fn shutdown(&self) {
        info!("Shutting down voice connection manager");

        // Cleanup all connections
        self.cleanup_all_connections().await;

        // Clear recovery states
        {
            let mut recovery_states = self.recovery_states.write().await;
            recovery_states.clear();
        }

        info!("Voice connection manager shutdown complete");
    }

    /// Get connection pool metrics (if pooling is enabled)
    pub async fn get_pool_metrics(&self) -> Option<pool::ConnectionMetrics> {
        self.voice_client.get_pool_metrics().await
    }

    /// Cleanup idle connections (if pooling is enabled)
    pub async fn cleanup_idle_connections(&self) -> usize {
        self.voice_client.cleanup_idle_connections().await
    }

    /// Get recovery state for a guild
    pub async fn get_recovery_state(&self, guild_id: &str) -> Option<RecoveryState> {
        let recovery_states = self.recovery_states.read().await;
        recovery_states.get(guild_id).cloned()
    }

    /// Reset recovery state for a guild
    pub async fn reset_recovery_state(&self, guild_id: &str) {
        let mut recovery_states = self.recovery_states.write().await;
        recovery_states.remove(guild_id);
        info!("Reset recovery state for guild {}", guild_id);
    }

    /// Get recovery configuration
    pub fn get_recovery_config(&self) -> &RecoveryConfig {
        &self.recovery_config
    }

    /// Update recovery configuration
    pub fn update_recovery_config(&mut self, config: RecoveryConfig) {
        self.recovery_config = config;
        info!("Updated recovery configuration");
    }

    /// Force close circuit breaker for a guild
    pub async fn force_close_circuit_breaker(&self, guild_id: &str) {
        let mut recovery_states = self.recovery_states.write().await;
        if let Some(state) = recovery_states.get_mut(guild_id) {
            if state.circuit_breaker_open {
                state.circuit_breaker_open = false;
                state.circuit_breaker_open_time = None;
                state.consecutive_failures = 0;
                info!("Forcibly closed circuit breaker for guild {}", guild_id);
                drop(recovery_states);
                self.handle_voice_event(guild_id, VoiceConnectionEvent::CircuitBreakerClosed)
                    .await;
            }
        }
    }

    /// Get all guilds with open circuit breakers
    pub async fn get_open_circuit_breakers(&self) -> Vec<String> {
        let recovery_states = self.recovery_states.read().await;
        recovery_states
            .iter()
            .filter(|(_, state)| state.circuit_breaker_open)
            .map(|(guild_id, _)| guild_id.clone())
            .collect()
    }

    /// Get recovery statistics
    pub async fn get_recovery_statistics(&self) -> RecoveryStatistics {
        let recovery_states = self.recovery_states.read().await;
        let mut stats = RecoveryStatistics::default();

        for (_, state) in recovery_states.iter() {
            stats.total_guilds += 1;
            stats.total_retries += state.total_retries as u64;

            if state.circuit_breaker_open {
                stats.open_circuit_breakers += 1;
            }

            if state.consecutive_failures > 0 {
                stats.guilds_with_failures += 1;
            }

            stats.max_consecutive_failures = stats
                .max_consecutive_failures
                .max(state.consecutive_failures);
        }

        stats
    }

    /// Classify Songbird connection error for recovery strategy
    fn classify_error(&self, error: &anyhow::Error) -> VoiceErrorType {
        #[cfg(feature = "discord")]
        {
            // Check if the error is a Songbird ConnectionError
            if let Some(songbird_error) = error.downcast_ref::<SongbirdConnectionError>() {
                return match songbird_error {
                    SongbirdConnectionError::TimedOut => VoiceErrorType::Temporary,
                    SongbirdConnectionError::Io(_) => VoiceErrorType::Temporary,
                    SongbirdConnectionError::Ws(_) => VoiceErrorType::Temporary,
                    SongbirdConnectionError::InterconnectFailure(_) => VoiceErrorType::Temporary,
                    SongbirdConnectionError::AttemptDiscarded => VoiceErrorType::Temporary,
                    SongbirdConnectionError::EndpointUrl => VoiceErrorType::Configuration,
                    SongbirdConnectionError::IllegalDiscoveryResponse => {
                        VoiceErrorType::Configuration
                    }
                    SongbirdConnectionError::IllegalIp => VoiceErrorType::Configuration,
                    SongbirdConnectionError::CryptoModeInvalid => VoiceErrorType::Authentication,
                    SongbirdConnectionError::CryptoModeUnavailable => {
                        VoiceErrorType::Authentication
                    }
                    SongbirdConnectionError::Crypto(_) => VoiceErrorType::Authentication,
                    SongbirdConnectionError::CryptoInvalidLength => VoiceErrorType::Authentication,
                    SongbirdConnectionError::Json(_) => VoiceErrorType::Temporary,
                    // Handle any future variants as temporary errors
                    _ => VoiceErrorType::Temporary,
                };
            }
        }

        // For other error types (or in standalone mode), check the error message for common patterns
        let error_msg = error.to_string().to_lowercase();
        if error_msg.contains("timeout") || error_msg.contains("network") {
            VoiceErrorType::Temporary
        } else if error_msg.contains("permission") || error_msg.contains("unauthorized") {
            VoiceErrorType::Authentication
        } else if error_msg.contains("rate limit") || error_msg.contains("too many") {
            VoiceErrorType::ResourceExhaustion
        } else if error_msg.contains("invalid") || error_msg.contains("malformed") {
            VoiceErrorType::Configuration
        } else {
            // Default to temporary for unknown errors
            VoiceErrorType::Temporary
        }
    }

    /// Calculate backoff delay with exponential backoff and jitter
    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.recovery_config.initial_backoff.as_millis() as f64;
        let multiplier = self.recovery_config.backoff_multiplier;
        let max_delay = self.recovery_config.max_backoff.as_millis() as f64;

        // Exponential backoff
        let delay = base_delay * multiplier.powi(attempt as i32);
        let delay = delay.min(max_delay);

        // Add jitter to prevent thundering herd
        let jitter_range = delay * self.recovery_config.jitter_factor;
        let mut rng = rand::rng();
        let jitter = (rng.random::<f64>() - 0.5) * 2.0 * jitter_range;
        let final_delay = (delay + jitter).max(0.0) as u64;

        Duration::from_millis(final_delay)
    }

    /// Check if circuit breaker should be opened
    async fn should_open_circuit_breaker(&self, guild_id: &str) -> bool {
        let recovery_states = self.recovery_states.read().await;
        if let Some(state) = recovery_states.get(guild_id) {
            state.consecutive_failures >= self.recovery_config.circuit_breaker_threshold
        } else {
            false
        }
    }

    /// Check if circuit breaker should be closed (reset)
    async fn should_close_circuit_breaker(&self, guild_id: &str) -> bool {
        let recovery_states = self.recovery_states.read().await;
        if let Some(state) = recovery_states.get(guild_id) {
            if let Some(open_time) = state.circuit_breaker_open_time {
                open_time.elapsed() >= self.recovery_config.circuit_breaker_reset_timeout
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Update recovery state after a failure
    async fn update_recovery_state_on_failure(&self, guild_id: &str) {
        let mut recovery_states = self.recovery_states.write().await;
        let state = recovery_states.entry(guild_id.to_string()).or_default();

        state.consecutive_failures += 1;
        state.last_failure = Some(Instant::now());
        state.total_retries += 1;

        // Check if circuit breaker should be opened
        if state.consecutive_failures >= self.recovery_config.circuit_breaker_threshold
            && !state.circuit_breaker_open
        {
            state.circuit_breaker_open = true;
            state.circuit_breaker_open_time = Some(Instant::now());
            info!(
                "Circuit breaker opened for guild {} after {} consecutive failures",
                guild_id, state.consecutive_failures
            );
        }
    }

    /// Update recovery state after a success
    async fn update_recovery_state_on_success(&self, guild_id: &str) {
        let mut recovery_states = self.recovery_states.write().await;
        let state = recovery_states.entry(guild_id.to_string()).or_default();

        let was_circuit_breaker_open = state.circuit_breaker_open;

        // Reset failure counters on success
        state.consecutive_failures = 0;
        state.last_failure = None;
        state.circuit_breaker_open = false;
        state.circuit_breaker_open_time = None;

        if was_circuit_breaker_open {
            info!(
                "Circuit breaker closed for guild {} after successful connection",
                guild_id
            );
        }
    }

    /// Handle voice connection events
    pub async fn handle_voice_event(&self, guild_id: &str, event: VoiceConnectionEvent) {
        // Create structured logger for this event
        let logger = VoiceEventLogger::new(guild_id.to_string());

        match &event {
            VoiceConnectionEvent::Connected => {
                info!("Voice connection established for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionEstablished,
                    std::collections::HashMap::new(),
                    None,
                );
                self.update_recovery_state_on_success(guild_id).await;

                // Register the voice connection with monitoring if enabled
                if let Some(monitor) = &self.monitor {
                    if let Some(call) = self.get_voice_connection(guild_id).await {
                        let monitor = monitor.read().await;
                        if let Err(e) = monitor
                            .register_voice_connection(guild_id.to_string(), call)
                            .await
                        {
                            warn!("Failed to register voice connection with monitoring for guild {}: {}", guild_id, e);
                            let mut context = std::collections::HashMap::new();
                            context
                                .insert("operation".to_string(), "register_monitoring".to_string());
                            logger.log_error_event(
                                &e,
                                "register_monitoring",
                                "monitoring_error",
                                context,
                            );
                        }
                    }
                }
            }
            VoiceConnectionEvent::Disconnected => {
                info!("Voice connection lost for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionClosed,
                    std::collections::HashMap::new(),
                    None,
                );

                // Unregister from monitoring if enabled
                if let Some(monitor) = &self.monitor {
                    let monitor = monitor.read().await;
                    if let Err(e) = monitor.unregister_voice_connection(guild_id).await {
                        warn!("Failed to unregister voice connection from monitoring for guild {}: {}", guild_id, e);
                        let mut context = std::collections::HashMap::new();
                        context
                            .insert("operation".to_string(), "unregister_monitoring".to_string());
                        logger.log_error_event(
                            &e,
                            "unregister_monitoring",
                            "monitoring_error",
                            context,
                        );
                    }
                }

                // Attempt to clean up the connection
                if let Err(e) = self.disconnect_voice_channel(guild_id).await {
                    warn!(
                        "Error cleaning up disconnected voice connection for guild {}: {}",
                        guild_id, e
                    );
                    let mut context = std::collections::HashMap::new();
                    context.insert("operation".to_string(), "cleanup_connection".to_string());
                    logger.log_error_event(&e, "cleanup_connection", "cleanup_error", context);
                }
            }
            VoiceConnectionEvent::Error(ref error) => {
                error!("Voice connection error for guild {}: {}", guild_id, error);
                let mut details = std::collections::HashMap::new();
                details.insert("error".to_string(), error.clone());
                logger.log_connection_event(VoiceEventType::ErrorOccurred, details, None);
                self.update_recovery_state_on_failure(guild_id).await;
            }
            VoiceConnectionEvent::RecoveryStarted { attempt, delay } => {
                info!(
                    "Starting recovery attempt {} for guild {} with delay {:?}",
                    attempt, guild_id, delay
                );
                logger.log_recovery_event(
                    VoiceEventType::RecoveryAttempt,
                    Some(*attempt),
                    None,
                    Some(*delay),
                    None,
                );
            }
            VoiceConnectionEvent::RecoverySucceeded { total_attempts } => {
                info!(
                    "Recovery succeeded for guild {} after {} attempts",
                    guild_id, total_attempts
                );
                logger.log_recovery_event(
                    VoiceEventType::RecoverySuccess,
                    None,
                    Some(*total_attempts),
                    None,
                    None,
                );
                self.update_recovery_state_on_success(guild_id).await;
            }
            VoiceConnectionEvent::RecoveryFailed {
                total_attempts,
                ref error,
            } => {
                error!(
                    "Recovery failed for guild {} after {} attempts: {}",
                    guild_id, total_attempts, error
                );
                logger.log_recovery_event(
                    VoiceEventType::RecoveryFailure,
                    None,
                    Some(*total_attempts),
                    None,
                    Some(error.clone()),
                );
            }
            VoiceConnectionEvent::CircuitBreakerOpened => {
                warn!("Circuit breaker opened for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::CircuitBreakerOpen,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::CircuitBreakerClosed => {
                info!("Circuit breaker closed for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::CircuitBreakerClosed,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::GatewayReady { ssrc, ref ip, port } => {
                info!(
                    "Voice gateway ready for guild {} - SSRC: {}, IP: {}, Port: {}",
                    guild_id, ssrc, ip, port
                );
                logger.log_gateway_event(
                    VoiceEventType::GatewayReady,
                    Some(*ssrc),
                    Some(ip.clone()),
                    Some(*port),
                    None,
                    None,
                );
                // This indicates the voice connection is fully established
                self.update_recovery_state_on_success(guild_id).await;
            }
            VoiceConnectionEvent::GatewayClosed {
                code,
                ref reason,
                by_remote,
            } => {
                warn!(
                    "Voice gateway closed for guild {} - Code: {}, Reason: {}, By Remote: {}",
                    guild_id, code, reason, by_remote
                );
                logger.log_gateway_event(
                    VoiceEventType::GatewayClosed,
                    None,
                    None,
                    None,
                    Some(*code),
                    Some(reason.clone()),
                );
                // Gateway closure may indicate connection issues
                if *by_remote {
                    self.update_recovery_state_on_failure(guild_id).await;
                }
            }
            VoiceConnectionEvent::GatewayError(ref error) => {
                error!("Voice gateway error for guild {}: {}", guild_id, error);
                let mut details = std::collections::HashMap::new();
                details.insert("error".to_string(), error.clone());
                logger.log_connection_event(VoiceEventType::GatewayError, details, None);
                self.update_recovery_state_on_failure(guild_id).await;
            }
            VoiceConnectionEvent::SpeakingStateChanged { speaking } => {
                debug!(
                    "Speaking state changed for guild {}: {}",
                    guild_id, speaking
                );
                let mut details = std::collections::HashMap::new();
                details.insert("speaking".to_string(), speaking.to_string());
                logger.log_connection_event(VoiceEventType::SpeakingStateChanged, details, None);
                // Speaking state changes are informational and don't affect recovery
            }

            // Connection State Transitions
            VoiceConnectionEvent::Connecting => {
                info!("Voice connection starting for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionStart,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::Reconnecting => {
                info!("Voice connection reconnecting for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::GatewayReconnect,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::ConnectionTimeout => {
                warn!("Voice connection timeout for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionTimeout,
                    std::collections::HashMap::new(),
                    None,
                );
                self.update_recovery_state_on_failure(guild_id).await;
            }
            VoiceConnectionEvent::ConnectionLost => {
                warn!("Voice connection lost for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionFailed,
                    std::collections::HashMap::new(),
                    None,
                );
                self.update_recovery_state_on_failure(guild_id).await;
            }
            VoiceConnectionEvent::ConnectionRestored => {
                info!("Voice connection restored for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::ConnectionEstablished,
                    std::collections::HashMap::new(),
                    None,
                );
                self.update_recovery_state_on_success(guild_id).await;
            }

            // Recovery Events
            VoiceConnectionEvent::RecoveryAborted { ref reason } => {
                warn!("Recovery aborted for guild {}: {}", guild_id, reason);
                let mut details = std::collections::HashMap::new();
                details.insert("reason".to_string(), reason.clone());
                logger.log_connection_event(VoiceEventType::RecoveryFailure, details, None);
            }

            // Circuit Breaker Events
            VoiceConnectionEvent::CircuitBreakerHalfOpen => {
                info!("Circuit breaker half-open for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::CircuitBreakerHalfOpen,
                    std::collections::HashMap::new(),
                    None,
                );
            }

            // Voice Gateway Events
            VoiceConnectionEvent::GatewayReconnecting => {
                info!("Voice gateway reconnecting for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::GatewayReconnect,
                    std::collections::HashMap::new(),
                    None,
                );
            }

            // Audio Events
            VoiceConnectionEvent::AudioStreamStarted => {
                info!("Audio stream started for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::AudioStreamStart,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::AudioStreamStopped => {
                info!("Audio stream stopped for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::AudioStreamStop,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::AudioStreamPaused => {
                info!("Audio stream paused for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::AudioStreamPause,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::AudioStreamResumed => {
                info!("Audio stream resumed for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::AudioStreamResume,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::AudioQualityChanged {
                old_bitrate,
                new_bitrate,
                ref reason,
            } => {
                info!(
                    "Audio quality changed for guild {} from {} to {} kbps: {}",
                    guild_id, old_bitrate, new_bitrate, reason
                );
                let mut details = std::collections::HashMap::new();
                details.insert("old_bitrate".to_string(), old_bitrate.to_string());
                details.insert("new_bitrate".to_string(), new_bitrate.to_string());
                details.insert("reason".to_string(), reason.clone());
                logger.log_connection_event(VoiceEventType::AudioQualityChange, details, None);
            }

            // Performance Events
            VoiceConnectionEvent::LatencyUpdate { latency_ms } => {
                debug!("Latency update for guild {}: {:.2}ms", guild_id, latency_ms);
                let mut details = std::collections::HashMap::new();
                details.insert("latency_ms".to_string(), latency_ms.to_string());
                logger.log_connection_event(VoiceEventType::PerformanceWarning, details, None);
            }
            VoiceConnectionEvent::PacketLoss { loss_percentage } => {
                if *loss_percentage > 5.0 {
                    warn!(
                        "High packet loss for guild {}: {:.2}%",
                        guild_id, loss_percentage
                    );
                } else {
                    debug!(
                        "Packet loss for guild {}: {:.2}%",
                        guild_id, loss_percentage
                    );
                }
                let mut details = std::collections::HashMap::new();
                details.insert("loss_percentage".to_string(), loss_percentage.to_string());
                logger.log_connection_event(VoiceEventType::PerformanceWarning, details, None);
            }
            VoiceConnectionEvent::JitterUpdate { jitter_ms } => {
                debug!("Jitter update for guild {}: {:.2}ms", guild_id, jitter_ms);
                let mut details = std::collections::HashMap::new();
                details.insert("jitter_ms".to_string(), jitter_ms.to_string());
                logger.log_connection_event(VoiceEventType::PerformanceWarning, details, None);
            }

            // State Events
            VoiceConnectionEvent::MuteStateChanged { muted } => {
                debug!("Mute state changed for guild {}: {}", guild_id, muted);
                let mut details = std::collections::HashMap::new();
                details.insert("muted".to_string(), muted.to_string());
                logger.log_connection_event(VoiceEventType::StateTransition, details, None);
            }
            VoiceConnectionEvent::DeafenStateChanged { deafened } => {
                debug!("Deafen state changed for guild {}: {}", guild_id, deafened);
                let mut details = std::collections::HashMap::new();
                details.insert("deafened".to_string(), deafened.to_string());
                logger.log_connection_event(VoiceEventType::StateTransition, details, None);
            }

            // Health Events
            VoiceConnectionEvent::HealthCheckPassed => {
                debug!("Health check passed for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::HealthCheck,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::HealthCheckFailed { ref reason } => {
                warn!("Health check failed for guild {}: {}", guild_id, reason);
                let mut details = std::collections::HashMap::new();
                details.insert("reason".to_string(), reason.clone());
                logger.log_connection_event(VoiceEventType::HealthCheckFailed, details, None);
            }
            VoiceConnectionEvent::ConnectionDegraded { ref severity } => {
                warn!(
                    "Connection degraded for guild {} (severity: {})",
                    guild_id, severity
                );
                let mut details = std::collections::HashMap::new();
                details.insert("severity".to_string(), severity.clone());
                logger.log_connection_event(VoiceEventType::PerformanceWarning, details, None);
            }
            VoiceConnectionEvent::ConnectionHealthy => {
                info!("Connection healthy for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::HealthCheck,
                    std::collections::HashMap::new(),
                    None,
                );
            }

            // Pool Events
            VoiceConnectionEvent::PoolConnectionCreated => {
                debug!("Pool connection created for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::PoolConnectionCreated,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::PoolConnectionDestroyed => {
                debug!("Pool connection destroyed for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::PoolConnectionDestroyed,
                    std::collections::HashMap::new(),
                    None,
                );
            }
            VoiceConnectionEvent::PoolConnectionReused => {
                debug!("Pool connection reused for guild {}", guild_id);
                logger.log_connection_event(
                    VoiceEventType::PoolConnectionReused,
                    std::collections::HashMap::new(),
                    None,
                );
            }

            // Error Events with Context
            VoiceConnectionEvent::CriticalError {
                ref error,
                ref context,
            } => {
                error!("Critical error for guild {}: {}", guild_id, error);
                let mut details = context.clone();
                details.insert("error".to_string(), error.clone());
                logger.log_connection_event(VoiceEventType::CriticalError, details, None);
                self.update_recovery_state_on_failure(guild_id).await;
            }
            VoiceConnectionEvent::ErrorRecovered {
                ref error,
                ref recovery_action,
            } => {
                info!(
                    "Error recovered for guild {} - Error: {}, Action: {}",
                    guild_id, error, recovery_action
                );
                let mut details = std::collections::HashMap::new();
                details.insert("error".to_string(), error.clone());
                details.insert("recovery_action".to_string(), recovery_action.clone());
                logger.log_connection_event(VoiceEventType::ErrorRecovered, details, None);
                self.update_recovery_state_on_success(guild_id).await;
            }
        }

        // Broadcast the event to external systems (player manager, WebSocket clients)
        self.broadcast_voice_event(guild_id, &event).await;
    }

    /// Broadcast voice connection events to external systems
    async fn broadcast_voice_event(&self, guild_id: &str, event: &VoiceConnectionEvent) {
        debug!(
            "Broadcasting voice event for guild {}: {:?}",
            guild_id, event
        );

        // Publish to subscription manager (handles event history and filtering)
        self.subscription_manager
            .publish_event(guild_id.to_string(), event.clone())
            .await;

        // Broadcast to player system if broadcaster is set
        if let Some(ref broadcaster) = self.event_broadcaster {
            broadcaster(guild_id.to_string(), event.clone());
        }

        // Send event to monitoring system if enabled
        if let Some(monitor) = &self.monitor {
            let monitor = monitor.read().await;
            monitor.handle_voice_event(guild_id, event).await;
        }
    }
}

/// Voice connection error classification for recovery strategies
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum VoiceErrorType {
    /// Temporary network issues that should be retried
    Temporary,
    /// Authentication or permission issues that need user intervention
    Authentication,
    /// Configuration issues (invalid endpoints, etc.)
    Configuration,
    /// Resource exhaustion (rate limits, etc.)
    ResourceExhaustion,
    /// Permanent failures that should not be retried
    Permanent,
}

/// Recovery strategy configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecoveryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay
    pub initial_backoff: Duration,
    /// Maximum backoff delay
    pub max_backoff: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor to prevent thundering herd (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker reset timeout
    pub circuit_breaker_reset_timeout: Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            circuit_breaker_threshold: 10,
            circuit_breaker_reset_timeout: Duration::from_secs(60),
        }
    }
}

/// Connection recovery state tracking
#[derive(Debug, Clone)]
#[allow(dead_code)]
#[derive(Default)]
pub struct RecoveryState {
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Last failure time
    pub last_failure: Option<Instant>,
    /// Circuit breaker state
    pub circuit_breaker_open: bool,
    /// Circuit breaker open time
    pub circuit_breaker_open_time: Option<Instant>,
    /// Total retry attempts
    pub total_retries: u32,
}

/// Voice connection events
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub enum VoiceConnectionEvent {
    // Basic Connection Events
    Connected,
    Disconnected,
    Error(String),

    // Connection State Transitions
    Connecting,
    Reconnecting,
    ConnectionTimeout,
    ConnectionLost,
    ConnectionRestored,

    // Recovery Events
    RecoveryStarted {
        attempt: u32,
        delay: Duration,
    },
    RecoverySucceeded {
        total_attempts: u32,
    },
    RecoveryFailed {
        total_attempts: u32,
        error: String,
    },
    RecoveryAborted {
        reason: String,
    },

    // Circuit Breaker Events
    CircuitBreakerOpened,
    CircuitBreakerClosed,
    CircuitBreakerHalfOpen,

    // Voice Gateway Events
    GatewayReady {
        ssrc: u32,
        ip: String,
        port: u16,
    },
    GatewayClosed {
        code: i32,
        reason: String,
        by_remote: bool,
    },
    GatewayError(String),
    GatewayReconnecting,

    // Audio Events
    AudioStreamStarted,
    AudioStreamStopped,
    AudioStreamPaused,
    AudioStreamResumed,
    AudioQualityChanged {
        old_bitrate: u32,
        new_bitrate: u32,
        reason: String,
    },

    // Performance Events
    LatencyUpdate {
        latency_ms: f64,
    },
    PacketLoss {
        loss_percentage: f64,
    },
    JitterUpdate {
        jitter_ms: f64,
    },

    // State Events
    SpeakingStateChanged {
        speaking: bool,
    },
    MuteStateChanged {
        muted: bool,
    },
    DeafenStateChanged {
        deafened: bool,
    },

    // Health Events
    HealthCheckPassed,
    HealthCheckFailed {
        reason: String,
    },
    ConnectionDegraded {
        severity: String,
    },
    ConnectionHealthy,

    // Pool Events
    PoolConnectionCreated,
    PoolConnectionDestroyed,
    PoolConnectionReused,

    // Error Events with Context
    CriticalError {
        error: String,
        context: std::collections::HashMap<String, String>,
    },
    ErrorRecovered {
        error: String,
        recovery_action: String,
    },
}

/// Recovery statistics for monitoring and debugging
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct RecoveryStatistics {
    /// Total number of guilds being tracked
    pub total_guilds: u32,
    /// Total number of retry attempts across all guilds
    pub total_retries: u64,
    /// Number of guilds with open circuit breakers
    pub open_circuit_breakers: u32,
    /// Number of guilds with current failures
    pub guilds_with_failures: u32,
    /// Maximum consecutive failures across all guilds
    pub max_consecutive_failures: u32,
}

impl Default for VoiceConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    #[tokio::test]
    async fn test_voice_connection_manager_creation() {
        let manager = VoiceConnectionManager::new();
        assert!(!manager.is_voice_connected("123456789").await);
    }

    #[tokio::test]
    async fn test_voice_state_update() {
        let manager = VoiceConnectionManager::new();

        // Test valid voice state
        let valid_state = VoiceState {
            token: "test_token".to_string(),
            endpoint: "test_endpoint".to_string(),
            session_id: "test_session".to_string(),
        };

        // This test will fail due to Songbird not being initialized in test environment
        // We'll use catch_unwind to handle the panic gracefully
        let result =
            AssertUnwindSafe(manager.update_voice_state("123456789".to_string(), valid_state))
                .catch_unwind()
                .await;

        // Either the function returns an error or panics (both are expected in test environment)
        match result {
            Ok(Err(_)) => {
                // Function returned an error - this is expected
            }
            Err(_) => {
                // Function panicked - this is also expected due to Songbird not being initialized
            }
            Ok(Ok(_)) => {
                // This would be unexpected in test environment
                panic!("Voice connection unexpectedly succeeded in test environment");
            }
        }
    }

    #[tokio::test]
    async fn test_partial_voice_state_rejection() {
        let manager = VoiceConnectionManager::new();

        // Test empty token
        let voice_state = VoiceState {
            token: "".to_string(),
            endpoint: "test_endpoint".to_string(),
            session_id: "test_session".to_string(),
        };

        let result = manager
            .update_voice_state("123456789".to_string(), voice_state)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Partial Lavalink voice state"));

        // Test empty endpoint
        let voice_state = VoiceState {
            token: "test_token".to_string(),
            endpoint: "".to_string(),
            session_id: "test_session".to_string(),
        };

        let result = manager
            .update_voice_state("123456789".to_string(), voice_state)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Partial Lavalink voice state"));

        // Test empty session_id
        let voice_state = VoiceState {
            token: "test_token".to_string(),
            endpoint: "test_endpoint".to_string(),
            session_id: "".to_string(),
        };

        let result = manager
            .update_voice_state("123456789".to_string(), voice_state)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Partial Lavalink voice state"));
    }

    #[tokio::test]
    async fn test_connection_tracking() {
        let manager = VoiceConnectionManager::new();

        // Initially no connections
        assert_eq!(manager.get_active_connections().await.len(), 0);
        assert!(!manager.is_voice_connected("123456789").await);
        assert!(manager.get_voice_connection("123456789").await.is_none());
    }

    #[tokio::test]
    async fn test_voice_event_handling() {
        let manager = VoiceConnectionManager::new();

        // Test event handling (should not panic)
        manager
            .handle_voice_event("123456789", VoiceConnectionEvent::Connected)
            .await;
        manager
            .handle_voice_event("123456789", VoiceConnectionEvent::Disconnected)
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::Error("Test error".to_string()),
            )
            .await;

        // Test recovery events
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::RecoveryStarted {
                    attempt: 1,
                    delay: Duration::from_millis(500),
                },
            )
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::RecoverySucceeded { total_attempts: 2 },
            )
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::RecoveryFailed {
                    total_attempts: 3,
                    error: "Test recovery error".to_string(),
                },
            )
            .await;
        manager
            .handle_voice_event("123456789", VoiceConnectionEvent::CircuitBreakerOpened)
            .await;
        manager
            .handle_voice_event("123456789", VoiceConnectionEvent::CircuitBreakerClosed)
            .await;

        // Test new voice gateway events
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::GatewayReady {
                    ssrc: 12345,
                    ip: "127.0.0.1".to_string(),
                    port: 50000,
                },
            )
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::GatewayClosed {
                    code: 1000,
                    reason: "Normal closure".to_string(),
                    by_remote: false,
                },
            )
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::GatewayError("Test gateway error".to_string()),
            )
            .await;
        manager
            .handle_voice_event(
                "123456789",
                VoiceConnectionEvent::SpeakingStateChanged { speaking: true },
            )
            .await;
    }

    #[tokio::test]
    async fn test_recovery_config() {
        let custom_config = RecoveryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 1.5,
            jitter_factor: 0.2,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_timeout: Duration::from_secs(30),
        };

        let manager = VoiceConnectionManager::with_recovery_config(custom_config.clone());
        assert_eq!(manager.get_recovery_config().max_retries, 3);
        assert_eq!(manager.get_recovery_config().circuit_breaker_threshold, 5);
    }

    #[tokio::test]
    async fn test_recovery_state_management() {
        let manager = VoiceConnectionManager::new();
        let guild_id = "123456789";

        // Initially no recovery state
        assert!(manager.get_recovery_state(guild_id).await.is_none());

        // Simulate failure to create recovery state
        manager.update_recovery_state_on_failure(guild_id).await;
        let state = manager.get_recovery_state(guild_id).await.unwrap();
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.total_retries, 1);

        // Simulate success to reset state
        manager.update_recovery_state_on_success(guild_id).await;
        let state = manager.get_recovery_state(guild_id).await.unwrap();
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.circuit_breaker_open);

        // Reset recovery state
        manager.reset_recovery_state(guild_id).await;
        assert!(manager.get_recovery_state(guild_id).await.is_none());
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 2,
            ..Default::default()
        };
        let manager = VoiceConnectionManager::with_recovery_config(config);
        let guild_id = "123456789";

        // Simulate failures to trigger circuit breaker
        manager.update_recovery_state_on_failure(guild_id).await;
        assert!(!manager.should_open_circuit_breaker(guild_id).await);

        manager.update_recovery_state_on_failure(guild_id).await;
        assert!(manager.should_open_circuit_breaker(guild_id).await);

        // Force close circuit breaker
        manager.force_close_circuit_breaker(guild_id).await;
        let state = manager.get_recovery_state(guild_id).await.unwrap();
        assert!(!state.circuit_breaker_open);
    }

    #[tokio::test]
    async fn test_recovery_statistics() {
        let manager = VoiceConnectionManager::new();

        // Initially empty statistics
        let stats = manager.get_recovery_statistics().await;
        assert_eq!(stats.total_guilds, 0);
        assert_eq!(stats.total_retries, 0);

        // Add some failures
        manager.update_recovery_state_on_failure("guild1").await;
        manager.update_recovery_state_on_failure("guild1").await;
        manager.update_recovery_state_on_failure("guild2").await;

        let stats = manager.get_recovery_statistics().await;
        assert_eq!(stats.total_guilds, 2);
        assert_eq!(stats.total_retries, 3);
        assert_eq!(stats.guilds_with_failures, 2);
        assert_eq!(stats.max_consecutive_failures, 2);
    }
}

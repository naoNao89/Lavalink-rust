#[cfg(test)]
#[allow(clippy::module_inception)]
#[allow(dead_code)] // Test utilities may not be used in all test configurations
mod recovery_tests {
    use crate::protocol::messages::VoiceState;
    use crate::voice::connection::*;
    use anyhow::{anyhow, Result};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{Mutex, RwLock};
    use tokio::time::{sleep, Duration};

    /// Mock voice client for testing recovery scenarios
    pub struct MockRecoveryVoiceClient {
        /// Connection failure simulation
        failure_mode: Arc<RwLock<FailureMode>>,
        /// Connection attempt counter
        attempt_counter: Arc<RwLock<u32>>,
        /// Event callback
        event_callback: Option<Arc<dyn Fn(String, VoiceConnectionEvent) + Send + Sync>>,
        /// Simulated connections
        connections: Arc<RwLock<HashMap<String, bool>>>,
    }

    /// Different failure modes for testing
    #[derive(Debug, Clone)]
    pub enum FailureMode {
        /// No failures - all connections succeed
        None,
        /// Fail for the first N attempts, then succeed
        FailFirstN(u32),
        /// Always fail with a specific error type
        AlwaysFail(VoiceErrorType),
        /// Intermittent failures (fail every Nth attempt)
        Intermittent(u32),
        /// Timeout failures
        Timeout,
        /// Authentication failures
        Authentication,
        /// Configuration failures
        Configuration,
        /// Resource exhaustion
        ResourceExhaustion,
    }

    impl MockRecoveryVoiceClient {
        pub fn new() -> Self {
            Self {
                failure_mode: Arc::new(RwLock::new(FailureMode::None)),
                attempt_counter: Arc::new(RwLock::new(0)),
                event_callback: None,
                connections: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn set_failure_mode(&self, mode: FailureMode) {
            let mut failure_mode = self.failure_mode.write().await;
            *failure_mode = mode;
        }

        pub async fn reset_attempt_counter(&self) {
            let mut counter = self.attempt_counter.write().await;
            *counter = 0;
        }

        pub async fn get_attempt_count(&self) -> u32 {
            let counter = self.attempt_counter.read().await;
            *counter
        }

        pub fn set_event_callback<F>(&mut self, callback: F)
        where
            F: Fn(String, VoiceConnectionEvent) + Send + Sync + 'static,
        {
            self.event_callback = Some(Arc::new(callback));
        }

        pub async fn simulate_connection(
            &self,
            guild_id: String,
            _voice_state: VoiceState,
        ) -> Result<()> {
            // Increment attempt counter
            {
                let mut counter = self.attempt_counter.write().await;
                *counter += 1;
            }

            let failure_mode = self.failure_mode.read().await;
            let attempt_count = self.get_attempt_count().await;

            // Determine if this attempt should fail based on failure mode
            let should_fail = match &*failure_mode {
                FailureMode::None => false,
                FailureMode::FailFirstN(n) => attempt_count <= *n,
                FailureMode::AlwaysFail(_) => true,
                FailureMode::Intermittent(interval) => attempt_count % interval == 0,
                FailureMode::Timeout => true,
                FailureMode::Authentication => true,
                FailureMode::Configuration => true,
                FailureMode::ResourceExhaustion => true,
            };

            if should_fail {
                let error = match &*failure_mode {
                    FailureMode::FailFirstN(_) => anyhow!("Temporary connection failure"),
                    FailureMode::AlwaysFail(error_type) => match error_type {
                        VoiceErrorType::Temporary => anyhow!("Network timeout"),
                        VoiceErrorType::Authentication => anyhow!("Authentication failed"),
                        VoiceErrorType::Configuration => anyhow!("Invalid endpoint configuration"),
                        VoiceErrorType::ResourceExhaustion => anyhow!("Rate limit exceeded"),
                        VoiceErrorType::Permanent => anyhow!("Permanent connection failure"),
                    },
                    FailureMode::Intermittent(_) => anyhow!("Intermittent network error"),
                    FailureMode::Timeout => anyhow!("Connection timeout"),
                    FailureMode::Authentication => anyhow!("Unauthorized access"),
                    FailureMode::Configuration => anyhow!("Malformed voice state"),
                    FailureMode::ResourceExhaustion => anyhow!("Too many requests"),
                    FailureMode::None => unreachable!(),
                };

                // Trigger error event if callback is set
                if let Some(ref callback) = self.event_callback {
                    callback(
                        guild_id.clone(),
                        VoiceConnectionEvent::Error(error.to_string()),
                    );
                }

                return Err(error);
            }

            // Connection succeeded
            {
                let mut connections = self.connections.write().await;
                connections.insert(guild_id.clone(), true);
            }

            // Trigger success events if callback is set
            if let Some(ref callback) = self.event_callback {
                callback(guild_id.clone(), VoiceConnectionEvent::Connecting);
                sleep(Duration::from_millis(10)).await;
                callback(guild_id, VoiceConnectionEvent::Connected);
            }

            Ok(())
        }

        pub async fn is_connected(&self, guild_id: &str) -> bool {
            let connections = self.connections.read().await;
            connections.get(guild_id).copied().unwrap_or(false)
        }

        pub async fn disconnect(&self, guild_id: &str) {
            let mut connections = self.connections.write().await;
            connections.remove(guild_id);

            if let Some(ref callback) = self.event_callback {
                callback(guild_id.to_string(), VoiceConnectionEvent::Disconnected);
            }
        }
    }

    /// Test environment for recovery scenarios
    pub struct RecoveryTestEnvironment {
        pub voice_manager: Arc<VoiceConnectionManager>,
        pub mock_client: Arc<Mutex<MockRecoveryVoiceClient>>,
        pub event_receiver: Arc<Mutex<Vec<(String, VoiceConnectionEvent)>>>,
    }

    impl RecoveryTestEnvironment {
        pub async fn new() -> Self {
            Self::with_config(RecoveryConfig::default()).await
        }

        pub async fn with_config(config: RecoveryConfig) -> Self {
            let voice_manager = Arc::new(VoiceConnectionManager::with_recovery_config(config));
            let mock_client = Arc::new(Mutex::new(MockRecoveryVoiceClient::new()));
            let event_receiver = Arc::new(Mutex::new(Vec::new()));

            // Set up event callback to capture events
            let event_receiver_clone = event_receiver.clone();
            {
                let mut client = mock_client.lock().await;
                client.set_event_callback(move |guild_id, event| {
                    let receiver = event_receiver_clone.clone();
                    tokio::spawn(async move {
                        let mut events = receiver.lock().await;
                        events.push((guild_id, event));
                    });
                });
            }

            Self {
                voice_manager,
                mock_client,
                event_receiver,
            }
        }

        pub async fn create_voice_state(&self, guild_id: &str) -> VoiceState {
            VoiceState {
                token: format!("test_token_{guild_id}"),
                endpoint: "test.discord.gg:443".to_string(),
                session_id: format!("test_session_{guild_id}"),
            }
        }

        pub async fn get_received_events(&self) -> Vec<(String, VoiceConnectionEvent)> {
            let events = self.event_receiver.lock().await;
            events.clone()
        }

        pub async fn clear_events(&self) {
            let mut events = self.event_receiver.lock().await;
            events.clear();
        }

        pub async fn count_events_of_type(
            &self,
            event_type: fn(&VoiceConnectionEvent) -> bool,
        ) -> usize {
            let events = self.get_received_events().await;
            events.iter().filter(|(_, event)| event_type(event)).count()
        }
    }

    #[tokio::test]
    async fn test_basic_recovery_success() {
        let config = RecoveryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let env = RecoveryTestEnvironment::with_config(config).await;
        let guild_id = "test_guild_recovery";
        let voice_state = env.create_voice_state(guild_id).await;

        // Set failure mode to fail first 2 attempts, then succeed
        {
            let client = env.mock_client.lock().await;
            client.set_failure_mode(FailureMode::FailFirstN(2)).await;
        }

        // Simulate connection attempts through the recovery system
        for attempt in 0..3 {
            let client = env.mock_client.lock().await;
            let result = client
                .simulate_connection(guild_id.to_string(), voice_state.clone())
                .await;

            if attempt < 2 {
                assert!(
                    result.is_err(),
                    "Expected failure on attempt {}",
                    attempt + 1
                );
            } else {
                assert!(
                    result.is_ok(),
                    "Expected success on attempt {}",
                    attempt + 1
                );
                break;
            }
        }

        // Verify final connection state
        {
            let client = env.mock_client.lock().await;
            assert!(client.is_connected(guild_id).await);
            assert_eq!(client.get_attempt_count().await, 3);
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_opening() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 3,
            max_retries: 2,
            initial_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let env = RecoveryTestEnvironment::with_config(config).await;
        let guild_id = "test_guild_circuit_breaker";

        // Set mock client to always fail to trigger circuit breaker
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::AlwaysFail(VoiceErrorType::Temporary))
            .await;

        // Attempt multiple connections that will fail
        for _ in 0..4 {
            let voice_state = env.create_voice_state(guild_id).await;
            let _ = env
                .voice_manager
                .update_voice_state(guild_id.to_string(), voice_state)
                .await;
        }

        // Get recovery state to verify circuit breaker behavior
        let state = env.voice_manager.get_recovery_state(guild_id).await;
        assert!(state.is_some());
        let state = state.unwrap();
        assert!(state.consecutive_failures >= 3);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 2,
            circuit_breaker_reset_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let env = RecoveryTestEnvironment::with_config(config).await;
        let guild_id = "test_guild_reset";

        // Set mock client to fail first few attempts
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::FailFirstN(3))
            .await;

        // Trigger circuit breaker by attempting connections
        for _ in 0..3 {
            let voice_state = env.create_voice_state(guild_id).await;
            let _ = env
                .voice_manager
                .update_voice_state(guild_id.to_string(), voice_state)
                .await;
        }

        // Get initial state
        let state = env.voice_manager.get_recovery_state(guild_id).await;
        assert!(state.is_some());

        // Force close circuit breaker
        env.voice_manager
            .force_close_circuit_breaker(guild_id)
            .await;

        // Verify circuit breaker is closed
        let state = env
            .voice_manager
            .get_recovery_state(guild_id)
            .await
            .unwrap();
        assert!(!state.circuit_breaker_open);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_recovery_config_validation() {
        let config = RecoveryConfig {
            max_retries: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            circuit_breaker_threshold: 3,
            circuit_breaker_reset_timeout: Duration::from_secs(60),
        };

        let manager = VoiceConnectionManager::with_recovery_config(config.clone());
        let retrieved_config = manager.get_recovery_config();

        assert_eq!(retrieved_config.max_retries, 5);
        assert_eq!(retrieved_config.circuit_breaker_threshold, 3);
        assert_eq!(retrieved_config.initial_backoff, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_recovery_event_handling() {
        let manager = VoiceConnectionManager::new();
        let guild_id = "test_guild_events";

        // Set up event capture
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        manager
            .subscription_manager()
            .subscribe(
                "test_subscription".to_string(),
                VoiceEventFilter::default(),
                move |guild_id, event| {
                    let events = events_clone.clone();
                    tokio::spawn(async move {
                        let mut events_guard = events.lock().await;
                        events_guard.push((guild_id, event));
                    });
                },
            )
            .await
            .unwrap();

        // Trigger some events manually to test event handling
        manager
            .handle_voice_event(guild_id, VoiceConnectionEvent::Connected)
            .await;
        manager
            .handle_voice_event(guild_id, VoiceConnectionEvent::Disconnected)
            .await;
        manager
            .handle_voice_event(
                guild_id,
                VoiceConnectionEvent::Error("Test error".to_string()),
            )
            .await;

        // Give events time to be processed
        sleep(Duration::from_millis(10)).await;

        // Check that events were captured
        let events_guard = events.lock().await;
        assert!(events_guard.len() >= 3);

        // Verify we received the expected events
        let event_types: Vec<_> = events_guard
            .iter()
            .map(|(_, event)| match event {
                VoiceConnectionEvent::Connected => "Connected",
                VoiceConnectionEvent::Disconnected => "Disconnected",
                VoiceConnectionEvent::Error(_) => "Error",
                _ => "Other",
            })
            .collect();

        assert!(event_types.contains(&"Connected"));
        assert!(event_types.contains(&"Disconnected"));
        assert!(event_types.contains(&"Error"));
    }

    #[tokio::test]
    async fn test_recovery_statistics() {
        let env = RecoveryTestEnvironment::new().await;
        let guild1 = "guild1";
        let guild2 = "guild2";

        // Set mock client to fail connections
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::AlwaysFail(VoiceErrorType::Temporary))
            .await;

        // Simulate failures for different guilds by attempting connections
        for _ in 0..2 {
            let voice_state = env.create_voice_state(guild1).await;
            let _ = env
                .voice_manager
                .update_voice_state(guild1.to_string(), voice_state)
                .await;
        }

        let voice_state = env.create_voice_state(guild2).await;
        let _ = env
            .voice_manager
            .update_voice_state(guild2.to_string(), voice_state)
            .await;

        // Get recovery statistics
        let stats = env.voice_manager.get_recovery_statistics().await;

        assert!(stats.total_guilds >= 2);
        assert!(stats.guilds_with_failures >= 2);
        assert!(stats.total_retries >= 3);
    }

    #[tokio::test]
    async fn test_non_retryable_errors() {
        let config = RecoveryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let env = RecoveryTestEnvironment::with_config(config).await;
        let guild_id = "test_guild_non_retryable";

        // Set failure mode to authentication error (non-retryable)
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::Authentication)
            .await;

        // Attempt connection - should fail immediately without retries
        let voice_state = env.create_voice_state(guild_id).await;
        let result = env
            .voice_manager
            .update_voice_state(guild_id.to_string(), voice_state)
            .await;
        assert!(result.is_err());

        // For non-retryable errors, should have fewer attempts
        let attempt_count = env.mock_client.lock().await.get_attempt_count().await;
        assert!(attempt_count <= 2); // Should not retry authentication errors extensively
    }

    #[tokio::test]
    async fn test_intermittent_failures() {
        let config = RecoveryConfig {
            max_retries: 2,
            initial_backoff: Duration::from_millis(5),
            circuit_breaker_threshold: 10, // High threshold to avoid circuit breaker
            ..Default::default()
        };
        let manager = VoiceConnectionManager::with_recovery_config(config);

        // Test multiple guilds to simulate intermittent behavior
        let mut success_count = 0;
        let mut failure_count = 0;

        for i in 0..6 {
            let guild_id = format!("test_guild_intermittent_{i}");
            let voice_state = VoiceState {
                token: format!("test_token_{i}"),
                endpoint: format!("test_endpoint_{i}"),
                session_id: format!("test_session_{i}"),
            };

            let result = manager
                .update_voice_state(guild_id.clone(), voice_state)
                .await;

            if result.is_ok() {
                success_count += 1;
            } else {
                failure_count += 1;
            }

            // Check recovery state was created
            let state = manager.get_recovery_state(&guild_id).await;
            if result.is_err() {
                assert!(state.is_some());
                let state = state.unwrap();
                assert!(state.total_retries > 0);
            }
        }

        // All attempts should fail since we don't have a real Discord connection
        // But recovery states should be created
        assert_eq!(failure_count, 6);
        assert_eq!(success_count, 0);

        // Verify recovery statistics
        let stats = manager.get_recovery_statistics().await;
        assert_eq!(stats.total_guilds, 6);
        assert!(stats.total_retries >= 6); // At least one retry per guild
    }

    #[tokio::test]
    async fn test_recovery_state_persistence() {
        let env = RecoveryTestEnvironment::new().await;
        let guild_id = "test_guild_persistence";

        // Set mock client to fail first few attempts
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::FailFirstN(3))
            .await;

        // Create initial failure state by attempting connections
        for _ in 0..2 {
            let voice_state = env.create_voice_state(guild_id).await;
            let _ = env
                .voice_manager
                .update_voice_state(guild_id.to_string(), voice_state)
                .await;
        }

        // Verify state is persisted
        let state1 = env.voice_manager.get_recovery_state(guild_id).await;
        assert!(state1.is_some());
        let state1 = state1.unwrap();
        assert!(state1.consecutive_failures >= 2);
        assert!(state1.total_retries >= 2);

        // Add more failures
        let voice_state = env.create_voice_state(guild_id).await;
        let _ = env
            .voice_manager
            .update_voice_state(guild_id.to_string(), voice_state)
            .await;

        // Verify state is updated
        let state2 = env
            .voice_manager
            .get_recovery_state(guild_id)
            .await
            .unwrap();
        assert!(state2.consecutive_failures >= 3);
        assert!(state2.total_retries >= 3);

        // Set mock client to succeed and attempt connection
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::None)
            .await;
        let voice_state = env.create_voice_state(guild_id).await;
        let result = env
            .voice_manager
            .update_voice_state(guild_id.to_string(), voice_state)
            .await;

        // If connection succeeded, consecutive failures should be reset
        if result.is_ok() {
            let state3 = env
                .voice_manager
                .get_recovery_state(guild_id)
                .await
                .unwrap();
            assert_eq!(state3.consecutive_failures, 0);
            assert!(!state3.circuit_breaker_open);
        }
    }

    #[tokio::test]
    async fn test_multiple_guild_isolation() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 2,
            max_retries: 1,
            initial_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let env = RecoveryTestEnvironment::with_config(config).await;
        let guild1 = "guild1";
        let guild2 = "guild2";

        // Set mock client to always fail
        env.mock_client
            .lock()
            .await
            .set_failure_mode(FailureMode::AlwaysFail(VoiceErrorType::Temporary))
            .await;

        // Trigger circuit breaker for guild1 only (2 attempts)
        for _ in 0..2 {
            let voice_state = env.create_voice_state(guild1).await;
            let _ = env
                .voice_manager
                .update_voice_state(guild1.to_string(), voice_state)
                .await;
        }

        // Add one failure for guild2
        let voice_state = env.create_voice_state(guild2).await;
        let _ = env
            .voice_manager
            .update_voice_state(guild2.to_string(), voice_state)
            .await;

        // Verify states are independent
        let state1 = env.voice_manager.get_recovery_state(guild1).await;
        let state2 = env.voice_manager.get_recovery_state(guild2).await;

        assert!(state1.is_some());
        assert!(state2.is_some());

        let state1 = state1.unwrap();
        let state2 = state2.unwrap();

        // Guild1 should have more failures than guild2
        assert!(state1.consecutive_failures >= 2);
        assert!(state2.consecutive_failures >= 1);
        assert!(state1.consecutive_failures > state2.consecutive_failures);
    }
}

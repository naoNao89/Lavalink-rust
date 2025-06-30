// Voice connection monitoring and health checks
// Provides comprehensive monitoring for Discord voice connections

use anyhow::Result;
use serde::{Deserialize, Serialize};
use songbird::Call;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::voice::connection::VoiceConnectionEvent;
use crate::voice::logging::{VoiceEventLogger, VoiceEventType};

/// Voice connection health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum HealthStatus {
    /// Connection is healthy and functioning normally
    Healthy,
    /// Connection has minor issues but is still functional
    Degraded,
    /// Connection has significant issues affecting performance
    Unhealthy,
    /// Connection is completely non-functional
    Critical,
    /// Connection status is unknown or being checked
    #[default]
    Unknown,
}


/// Connection performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionPerformanceMetrics {
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Packet loss percentage (0.0-100.0)
    pub packet_loss_percent: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Connection uptime in seconds
    pub uptime_seconds: u64,
    /// Number of reconnections
    pub reconnection_count: u32,
    /// Last successful ping timestamp (seconds since UNIX epoch)
    pub last_ping: Option<u64>,
    /// Audio quality score (0-100)
    pub audio_quality_score: u8,
}

/// Voice connection health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Guild ID being monitored
    pub guild_id: String,
    /// Overall health status
    pub status: HealthStatus,
    /// Performance metrics
    pub metrics: ConnectionPerformanceMetrics,
    /// Last check timestamp (seconds since UNIX epoch)
    pub last_check: u64,
    /// Issues detected during health check
    pub issues: Vec<String>,
    /// Recommendations for improving connection health
    pub recommendations: Vec<String>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Connection timeout for health checks in seconds
    pub health_check_timeout: u64,
    /// Latency threshold for degraded status (ms)
    pub latency_degraded_threshold: f64,
    /// Latency threshold for unhealthy status (ms)
    pub latency_unhealthy_threshold: f64,
    /// Packet loss threshold for degraded status (%)
    pub packet_loss_degraded_threshold: f64,
    /// Packet loss threshold for unhealthy status (%)
    pub packet_loss_unhealthy_threshold: f64,
    /// Maximum allowed reconnections per hour
    pub max_reconnections_per_hour: u32,
    /// Enable detailed performance tracking
    pub enable_performance_tracking: bool,
    /// Enable automatic remediation
    pub enable_auto_remediation: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval: 30,
            health_check_timeout: 10,
            latency_degraded_threshold: 100.0,
            latency_unhealthy_threshold: 250.0,
            packet_loss_degraded_threshold: 1.0,
            packet_loss_unhealthy_threshold: 5.0,
            max_reconnections_per_hour: 10,
            enable_performance_tracking: true,
            enable_auto_remediation: false,
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Monitoring alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringAlert {
    /// Alert ID
    pub id: String,
    /// Guild ID affected
    pub guild_id: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert title
    pub title: String,
    /// Alert description
    pub description: String,
    /// Timestamp when alert was created (seconds since UNIX epoch)
    pub created_at: u64,
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
    /// Suggested actions to resolve the issue
    pub suggested_actions: Vec<String>,
}

/// Real-time connection metrics collector
#[derive(Debug)]
struct ConnectionMetricsCollector {
    /// Connection start time for uptime calculation
    connection_start: Option<Instant>,
    /// Last ping time for latency measurement
    last_ping_time: Option<Instant>,
    /// Recent latency measurements (for averaging)
    latency_samples: Vec<f64>,
    /// Connection event timestamps for stability analysis
    connection_events: Vec<(Instant, VoiceConnectionEvent)>,
    /// Reconnection count
    reconnection_count: u32,
    /// Last successful ping timestamp
    last_successful_ping: Option<u64>,
}

impl ConnectionMetricsCollector {
    fn new(_guild_id: String) -> Self {
        Self {
            connection_start: None,
            last_ping_time: None,
            latency_samples: Vec::with_capacity(10), // Keep last 10 samples
            connection_events: Vec::new(),
            reconnection_count: 0,
            last_successful_ping: None,
        }
    }

    /// Record a connection event
    fn record_event(&mut self, event: VoiceConnectionEvent) {
        let now = Instant::now();

        match &event {
            VoiceConnectionEvent::Connected => {
                self.connection_start = Some(now);
                self.reconnection_count += 1;
            }
            VoiceConnectionEvent::Disconnected => {
                self.connection_start = None;
            }
            VoiceConnectionEvent::GatewayReady { .. } => {
                self.last_successful_ping = Some(current_timestamp());
            }
            _ => {}
        }

        // Keep only recent events (last 100)
        self.connection_events.push((now, event));
        if self.connection_events.len() > 100 {
            self.connection_events.remove(0);
        }
    }

    /// Start a ping measurement
    fn start_ping(&mut self) {
        self.last_ping_time = Some(Instant::now());
    }

    /// Complete a ping measurement and record latency
    fn complete_ping(&mut self) -> Option<f64> {
        if let Some(ping_start) = self.last_ping_time.take() {
            let latency_ms = ping_start.elapsed().as_millis() as f64;

            // Add to samples
            self.latency_samples.push(latency_ms);
            if self.latency_samples.len() > 10 {
                self.latency_samples.remove(0);
            }

            self.last_successful_ping = Some(current_timestamp());
            Some(latency_ms)
        } else {
            None
        }
    }

    /// Generate current performance metrics
    fn generate_metrics(&self) -> ConnectionPerformanceMetrics {
        let mut metrics = ConnectionPerformanceMetrics::default();

        // Calculate average latency
        if !self.latency_samples.is_empty() {
            metrics.avg_latency_ms =
                self.latency_samples.iter().sum::<f64>() / self.latency_samples.len() as f64;
        }

        // Calculate uptime
        if let Some(start_time) = self.connection_start {
            metrics.uptime_seconds = start_time.elapsed().as_secs();
        }

        // Set reconnection count
        metrics.reconnection_count = self.reconnection_count;

        // Set last ping
        metrics.last_ping = self.last_successful_ping;

        // Calculate jitter (standard deviation of latency samples)
        if self.latency_samples.len() > 1 {
            let mean = metrics.avg_latency_ms;
            let variance = self
                .latency_samples
                .iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>()
                / (self.latency_samples.len() - 1) as f64;
            metrics.jitter_ms = variance.sqrt();
        }

        // Estimate packet loss based on connection stability
        metrics.packet_loss_percent = self.estimate_packet_loss();

        // Calculate audio quality score
        metrics.audio_quality_score = self.calculate_quality_score(&metrics);

        metrics
    }

    /// Estimate packet loss based on connection events and stability
    fn estimate_packet_loss(&self) -> f64 {
        if self.connection_events.is_empty() {
            return 0.0;
        }

        // Count error events in the last minute
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        let recent_errors = self
            .connection_events
            .iter()
            .filter(|(timestamp, event)| {
                *timestamp > one_minute_ago && matches!(event, VoiceConnectionEvent::Error(_))
            })
            .count();

        // Estimate packet loss based on error frequency
        // This is a heuristic - in a real implementation, you'd want actual packet statistics
        let error_rate = recent_errors as f64 / 60.0; // errors per second
        (error_rate * 10.0).min(100.0) // Convert to percentage, cap at 100%
    }

    /// Calculate audio quality score based on metrics
    fn calculate_quality_score(&self, metrics: &ConnectionPerformanceMetrics) -> u8 {
        let mut score = 100u8;

        // Reduce score based on latency
        if metrics.avg_latency_ms > 50.0 {
            score = score.saturating_sub(((metrics.avg_latency_ms - 50.0) / 10.0) as u8);
        }

        // Reduce score based on packet loss
        if metrics.packet_loss_percent > 0.0 {
            score = score.saturating_sub((metrics.packet_loss_percent * 10.0) as u8);
        }

        // Reduce score based on jitter
        if metrics.jitter_ms > 10.0 {
            score = score.saturating_sub(((metrics.jitter_ms - 10.0) / 2.0) as u8);
        }

        // Reduce score based on connection instability
        let recent_disconnects = self
            .connection_events
            .iter()
            .filter(|(timestamp, event)| {
                *timestamp > Instant::now() - Duration::from_secs(300) && // Last 5 minutes
                matches!(event, VoiceConnectionEvent::Disconnected)
            })
            .count();

        if recent_disconnects > 0 {
            score = score.saturating_sub((recent_disconnects * 20) as u8);
        }

        score
    }
}

/// Voice connection monitor
pub struct VoiceConnectionMonitor {
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Health check results per guild
    health_results: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    /// Active monitoring alerts
    alerts: Arc<RwLock<HashMap<String, MonitoringAlert>>>,
    /// Performance metrics history
    metrics_history: Arc<RwLock<HashMap<String, Vec<ConnectionPerformanceMetrics>>>>,
    /// Alert callback for external notification systems
    alert_callback: Option<Arc<dyn Fn(MonitoringAlert) + Send + Sync>>,
    /// Health check task handle
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
    /// Real-time metrics collectors per guild
    metrics_collectors: Arc<RwLock<HashMap<String, ConnectionMetricsCollector>>>,
    /// Voice connections for real metrics collection
    voice_connections: Arc<RwLock<HashMap<String, Arc<Mutex<Call>>>>>,
}

/// Helper function to get current timestamp as seconds since UNIX epoch
#[allow(dead_code)]
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[allow(dead_code)]
impl VoiceConnectionMonitor {
    /// Create a new voice connection monitor
    pub fn new() -> Self {
        Self {
            config: MonitoringConfig::default(),
            health_results: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            alert_callback: None,
            health_check_handle: None,
            metrics_collectors: Arc::new(RwLock::new(HashMap::new())),
            voice_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create monitor with custom configuration
    pub fn with_config(config: MonitoringConfig) -> Self {
        Self {
            config,
            health_results: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            alert_callback: None,
            health_check_handle: None,
            metrics_collectors: Arc::new(RwLock::new(HashMap::new())),
            voice_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set alert callback for external notifications
    pub fn set_alert_callback<F>(&mut self, callback: F)
    where
        F: Fn(MonitoringAlert) + Send + Sync + 'static,
    {
        self.alert_callback = Some(Arc::new(callback));
    }

    /// Register a voice connection for monitoring
    pub async fn register_voice_connection(
        &self,
        guild_id: String,
        call: Arc<Mutex<Call>>,
    ) -> Result<()> {
        info!(
            "Registering voice connection for monitoring: guild {}",
            guild_id
        );

        // Store the voice connection
        {
            let mut connections = self.voice_connections.write().await;
            connections.insert(guild_id.clone(), call);
        }

        // Initialize metrics collector
        {
            let mut collectors = self.metrics_collectors.write().await;
            collectors.insert(
                guild_id.clone(),
                ConnectionMetricsCollector::new(guild_id.clone()),
            );
        }

        Ok(())
    }

    /// Unregister a voice connection from monitoring
    pub async fn unregister_voice_connection(&self, guild_id: &str) -> Result<()> {
        info!(
            "Unregistering voice connection from monitoring: guild {}",
            guild_id
        );

        // Remove voice connection
        {
            let mut connections = self.voice_connections.write().await;
            connections.remove(guild_id);
        }

        // Remove metrics collector
        {
            let mut collectors = self.metrics_collectors.write().await;
            collectors.remove(guild_id);
        }

        Ok(())
    }

    /// Perform a ping test for latency measurement
    pub async fn ping_connection(&self, guild_id: &str) -> Result<Option<f64>> {
        // Start ping measurement
        {
            let mut collectors = self.metrics_collectors.write().await;
            if let Some(collector) = collectors.get_mut(guild_id) {
                collector.start_ping();
            }
        }

        // Simulate ping by checking connection status
        // In a real implementation, you might send a ping packet or check connection timing
        let latency = if let Some(call) = self.get_voice_connection(guild_id).await {
            let call_guard = call.lock().await;

            // Check if connection is active
            if call_guard.current_connection().is_some() {
                // Simulate network round-trip time
                let start = Instant::now();

                // Use a small timeout to simulate ping
                let _ = timeout(Duration::from_millis(100), sleep(Duration::from_millis(1))).await;

                let elapsed = start.elapsed().as_millis() as f64;
                Some(elapsed)
            } else {
                None
            }
        } else {
            None
        };

        // Complete ping measurement
        if latency.is_some() {
            let mut collectors = self.metrics_collectors.write().await;
            if let Some(collector) = collectors.get_mut(guild_id) {
                return Ok(collector.complete_ping());
            }
        }

        Ok(latency)
    }

    /// Get voice connection for a guild
    async fn get_voice_connection(&self, guild_id: &str) -> Option<Arc<Mutex<Call>>> {
        let connections = self.voice_connections.read().await;
        connections.get(guild_id).cloned()
    }

    /// Start monitoring for a guild
    pub async fn start_monitoring(&mut self, guild_id: String) -> Result<()> {
        info!(
            "Starting voice connection monitoring for guild {}",
            guild_id
        );

        // Initialize health check result
        let initial_result = HealthCheckResult {
            guild_id: guild_id.clone(),
            status: HealthStatus::Unknown,
            metrics: ConnectionPerformanceMetrics::default(),
            last_check: current_timestamp(),
            issues: Vec::new(),
            recommendations: Vec::new(),
        };

        {
            let mut health_results = self.health_results.write().await;
            health_results.insert(guild_id.clone(), initial_result);
        }

        // Initialize metrics history
        {
            let mut metrics_history = self.metrics_history.write().await;
            metrics_history.insert(guild_id.clone(), Vec::new());
        }

        // Initialize metrics collector if not already present
        {
            let mut collectors = self.metrics_collectors.write().await;
            if !collectors.contains_key(&guild_id) {
                collectors.insert(guild_id.clone(), ConnectionMetricsCollector::new(guild_id));
            }
        }

        Ok(())
    }

    /// Stop monitoring for a guild
    pub async fn stop_monitoring(&mut self, guild_id: &str) -> Result<()> {
        info!(
            "Stopping voice connection monitoring for guild {}",
            guild_id
        );

        // Remove health results
        {
            let mut health_results = self.health_results.write().await;
            health_results.remove(guild_id);
        }

        // Remove metrics history
        {
            let mut metrics_history = self.metrics_history.write().await;
            metrics_history.remove(guild_id);
        }

        // Remove related alerts
        {
            let mut alerts = self.alerts.write().await;
            alerts.retain(|_, alert| alert.guild_id != guild_id);
        }

        Ok(())
    }

    /// Start the health check background task
    pub async fn start_health_checks(&mut self) {
        if self.health_check_handle.is_some() {
            warn!("Health check task is already running");
            return;
        }

        let config = self.config.clone();
        let health_results = Arc::clone(&self.health_results);
        let alerts = Arc::clone(&self.alerts);
        let metrics_history = Arc::clone(&self.metrics_history);
        let metrics_collectors = Arc::clone(&self.metrics_collectors);
        let alert_callback = self.alert_callback.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.health_check_interval));

            loop {
                interval.tick().await;

                // Get list of guilds to monitor
                let guild_ids: Vec<String> = {
                    let health_results = health_results.read().await;
                    health_results.keys().cloned().collect()
                };

                // Perform health checks for each guild
                for guild_id in guild_ids {
                    if let Err(e) = Self::perform_health_check(
                        &guild_id,
                        &config,
                        &health_results,
                        &alerts,
                        &metrics_history,
                        &alert_callback,
                        &metrics_collectors,
                    )
                    .await
                    {
                        error!("Health check failed for guild {}: {}", guild_id, e);
                    }
                }
            }
        });

        self.health_check_handle = Some(handle);
        info!("Started voice connection health check task");
    }

    /// Stop the health check background task
    pub async fn stop_health_checks(&mut self) {
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
            info!("Stopped voice connection health check task");
        }
    }

    /// Perform health check for a specific guild
    async fn perform_health_check(
        guild_id: &str,
        config: &MonitoringConfig,
        health_results: &Arc<RwLock<HashMap<String, HealthCheckResult>>>,
        alerts: &Arc<RwLock<HashMap<String, MonitoringAlert>>>,
        metrics_history: &Arc<RwLock<HashMap<String, Vec<ConnectionPerformanceMetrics>>>>,
        alert_callback: &Option<Arc<dyn Fn(MonitoringAlert) + Send + Sync>>,
        metrics_collectors: &Arc<RwLock<HashMap<String, ConnectionMetricsCollector>>>,
    ) -> Result<()> {
        debug!("Performing health check for guild {}", guild_id);

        // Create structured logger for health check events
        let logger = VoiceEventLogger::new(guild_id.to_string());

        // Get real metrics from the collector
        let metrics = {
            let collectors = metrics_collectors.read().await;
            if let Some(collector) = collectors.get(guild_id) {
                collector.generate_metrics()
            } else {
                // If no collector exists, create default metrics
                debug!(
                    "No metrics collector found for guild {}, using defaults",
                    guild_id
                );
                ConnectionPerformanceMetrics::default()
            }
        };

        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Determine health status
        let status = Self::determine_health_status(&metrics, config);

        // Check for issues and generate recommendations
        if metrics.avg_latency_ms > config.latency_degraded_threshold {
            issues.push(format!("High latency: {:.1}ms", metrics.avg_latency_ms));
            recommendations.push("Consider checking network connectivity".to_string());
        }

        if metrics.packet_loss_percent > config.packet_loss_degraded_threshold {
            issues.push(format!(
                "Packet loss detected: {:.1}%",
                metrics.packet_loss_percent
            ));
            recommendations.push("Check network stability and bandwidth".to_string());
        }

        // Create health check result
        let result = HealthCheckResult {
            guild_id: guild_id.to_string(),
            status,
            metrics: metrics.clone(),
            last_check: current_timestamp(),
            issues,
            recommendations,
        };

        // Log health check event
        let health_status_str = match status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Critical => "critical",
            HealthStatus::Unknown => "unknown",
        };

        logger.log_monitoring_event(
            VoiceEventType::HealthCheck,
            Some(health_status_str.to_string()),
            Some(metrics.avg_latency_ms),
            Some(metrics.packet_loss_percent),
            Some(Self::calculate_audio_quality_score(&metrics)),
        );

        // Update health results
        {
            let mut health_results = health_results.write().await;
            health_results.insert(guild_id.to_string(), result.clone());
        }

        // Store metrics history
        if config.enable_performance_tracking {
            let mut metrics_history = metrics_history.write().await;
            if let Some(history) = metrics_history.get_mut(guild_id) {
                history.push(metrics.clone());
                // Keep only last 100 entries
                if history.len() > 100 {
                    history.remove(0);
                }
            }
        }

        // Generate alerts if needed
        if status == HealthStatus::Unhealthy || status == HealthStatus::Critical {
            logger.log_monitoring_event(
                VoiceEventType::AlertGenerated,
                Some(health_status_str.to_string()),
                Some(metrics.avg_latency_ms),
                Some(metrics.packet_loss_percent),
                Some(Self::calculate_audio_quality_score(&metrics)),
            );

            Self::generate_alert(guild_id, &result, alerts, alert_callback).await?;
        }

        Ok(())
    }

    /// Calculate audio quality score based on performance metrics
    fn calculate_audio_quality_score(metrics: &ConnectionPerformanceMetrics) -> u8 {
        let mut score = 100u8;

        // Reduce score based on latency
        if metrics.avg_latency_ms > 50.0 {
            score = score.saturating_sub(((metrics.avg_latency_ms - 50.0) / 10.0) as u8);
        }

        // Reduce score based on packet loss
        if metrics.packet_loss_percent > 0.0 {
            score = score.saturating_sub((metrics.packet_loss_percent * 10.0) as u8);
        }

        // Reduce score based on jitter
        if metrics.jitter_ms > 10.0 {
            score = score.saturating_sub(((metrics.jitter_ms - 10.0) / 2.0) as u8);
        }

        score
    }

    /// Determine health status based on metrics and configuration
    fn determine_health_status(
        metrics: &ConnectionPerformanceMetrics,
        config: &MonitoringConfig,
    ) -> HealthStatus {
        // Check for critical conditions
        if metrics.avg_latency_ms > config.latency_unhealthy_threshold * 2.0
            || metrics.packet_loss_percent > config.packet_loss_unhealthy_threshold * 2.0
        {
            return HealthStatus::Critical;
        }

        // Check for unhealthy conditions
        if metrics.avg_latency_ms > config.latency_unhealthy_threshold
            || metrics.packet_loss_percent > config.packet_loss_unhealthy_threshold
        {
            return HealthStatus::Unhealthy;
        }

        // Check for degraded conditions
        if metrics.avg_latency_ms > config.latency_degraded_threshold
            || metrics.packet_loss_percent > config.packet_loss_degraded_threshold
        {
            return HealthStatus::Degraded;
        }

        HealthStatus::Healthy
    }

    /// Generate monitoring alert
    async fn generate_alert(
        guild_id: &str,
        result: &HealthCheckResult,
        alerts: &Arc<RwLock<HashMap<String, MonitoringAlert>>>,
        alert_callback: &Option<Arc<dyn Fn(MonitoringAlert) + Send + Sync>>,
    ) -> Result<()> {
        let alert_id = format!("{}_{}", guild_id, current_timestamp());

        let (severity, title) = match result.status {
            HealthStatus::Critical => (AlertSeverity::Critical, "Voice Connection Critical"),
            HealthStatus::Unhealthy => (AlertSeverity::Error, "Voice Connection Unhealthy"),
            HealthStatus::Degraded => (AlertSeverity::Warning, "Voice Connection Degraded"),
            _ => return Ok(()), // No alert needed for healthy connections
        };

        let description = format!(
            "Voice connection for guild {} is {}. Latency: {:.1}ms, Packet Loss: {:.1}%",
            guild_id,
            format!("{:?}", result.status).to_lowercase(),
            result.metrics.avg_latency_ms,
            result.metrics.packet_loss_percent
        );

        let alert = MonitoringAlert {
            id: alert_id.clone(),
            guild_id: guild_id.to_string(),
            severity,
            title: title.to_string(),
            description,
            created_at: current_timestamp(),
            acknowledged: false,
            suggested_actions: result.recommendations.clone(),
        };

        // Store alert
        {
            let mut alerts = alerts.write().await;
            alerts.insert(alert_id, alert.clone());
        }

        // Trigger callback if available
        if let Some(callback) = alert_callback {
            callback(alert);
        }

        Ok(())
    }

    /// Get health status for a specific guild
    pub async fn get_health_status(&self, guild_id: &str) -> Option<HealthCheckResult> {
        let health_results = self.health_results.read().await;
        health_results.get(guild_id).cloned()
    }

    /// Get health status for all monitored guilds
    pub async fn get_all_health_status(&self) -> HashMap<String, HealthCheckResult> {
        let health_results = self.health_results.read().await;
        health_results.clone()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<MonitoringAlert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|alert| !alert.acknowledged)
            .cloned()
            .collect()
    }

    /// Get all alerts (including acknowledged ones)
    pub async fn get_all_alerts(&self) -> Vec<MonitoringAlert> {
        let alerts = self.alerts.read().await;
        alerts.values().cloned().collect()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            info!("Alert {} acknowledged", alert_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Alert {} not found", alert_id))
        }
    }

    /// Clear acknowledged alerts older than specified duration
    pub async fn clear_old_alerts(&self, max_age: Duration) -> usize {
        let mut alerts = self.alerts.write().await;
        let cutoff_time = current_timestamp() - max_age.as_secs();
        let initial_count = alerts.len();

        alerts.retain(|_, alert| !alert.acknowledged || alert.created_at > cutoff_time);

        let cleared_count = initial_count - alerts.len();
        if cleared_count > 0 {
            info!("Cleared {} old acknowledged alerts", cleared_count);
        }
        cleared_count
    }

    /// Get performance metrics history for a guild
    pub async fn get_metrics_history(
        &self,
        guild_id: &str,
    ) -> Option<Vec<ConnectionPerformanceMetrics>> {
        let metrics_history = self.metrics_history.read().await;
        metrics_history.get(guild_id).cloned()
    }

    /// Get monitoring summary statistics
    pub async fn get_monitoring_summary(&self) -> MonitoringSummary {
        let health_results = self.health_results.read().await;
        let alerts = self.alerts.read().await;

        let mut summary = MonitoringSummary {
            total_monitored_guilds: health_results.len() as u32,
            ..Default::default()
        };

        // Count by health status
        for result in health_results.values() {
            match result.status {
                HealthStatus::Healthy => summary.healthy_connections += 1,
                HealthStatus::Degraded => summary.degraded_connections += 1,
                HealthStatus::Unhealthy => summary.unhealthy_connections += 1,
                HealthStatus::Critical => summary.critical_connections += 1,
                HealthStatus::Unknown => summary.unknown_connections += 1,
            }

            // Calculate average metrics
            summary.avg_latency_ms += result.metrics.avg_latency_ms;
            summary.avg_packet_loss_percent += result.metrics.packet_loss_percent;
        }

        if !health_results.is_empty() {
            summary.avg_latency_ms /= health_results.len() as f64;
            summary.avg_packet_loss_percent /= health_results.len() as f64;
        }

        // Count alerts
        summary.total_alerts = alerts.len() as u32;
        summary.unacknowledged_alerts =
            alerts.values().filter(|alert| !alert.acknowledged).count() as u32;

        summary
    }

    /// Update monitoring configuration
    pub fn update_config(&mut self, config: MonitoringConfig) {
        self.config = config;
        info!("Updated monitoring configuration");
    }

    /// Handle voice connection event for monitoring
    pub async fn handle_voice_event(&self, guild_id: &str, event: &VoiceConnectionEvent) {
        debug!("Handling voice event for guild {}: {:?}", guild_id, event);

        // Create structured logger for monitoring events
        let logger = VoiceEventLogger::new(guild_id.to_string());

        // Update metrics collector with the event
        {
            let mut collectors = self.metrics_collectors.write().await;
            if let Some(collector) = collectors.get_mut(guild_id) {
                collector.record_event(event.clone());
            }
        }

        // Update health results based on event
        let mut health_results = self.health_results.write().await;
        if let Some(result) = health_results.get_mut(guild_id) {
            match event {
                VoiceConnectionEvent::Connected => {
                    result.status = HealthStatus::Healthy;
                    result.issues.clear(); // Clear previous issues on successful connection
                    logger.log_monitoring_event(
                        VoiceEventType::ConnectionEstablished,
                        Some("healthy".to_string()),
                        None,
                        None,
                        None,
                    );
                }
                VoiceConnectionEvent::Disconnected => {
                    result.status = HealthStatus::Critical;
                    result.issues.push("Connection lost".to_string());
                    logger.log_monitoring_event(
                        VoiceEventType::ConnectionClosed,
                        Some("critical".to_string()),
                        None,
                        None,
                        None,
                    );
                }
                VoiceConnectionEvent::Error(error) => {
                    result.status = HealthStatus::Unhealthy;
                    result.issues.push(format!("Connection error: {error}"));
                    let mut context = std::collections::HashMap::new();
                    context.insert("error".to_string(), error.clone());
                    logger.log_error_event(
                        &anyhow::anyhow!("{}", error),
                        "voice_connection",
                        "connection_error",
                        context,
                    );
                }
                VoiceConnectionEvent::GatewayReady { .. } => {
                    // Trigger a ping measurement for latency
                    let guild_id_clone = guild_id.to_string();
                    let voice_connections = Arc::clone(&self.voice_connections);
                    let metrics_collectors = Arc::clone(&self.metrics_collectors);

                    tokio::spawn(async move {
                        // Perform ping measurement inline to avoid lifetime issues
                        if let Some(call) = {
                            let connections = voice_connections.read().await;
                            connections.get(&guild_id_clone).cloned()
                        } {
                            let call_guard = call.lock().await;

                            // Check if connection is active and measure latency
                            if call_guard.current_connection().is_some() {
                                let start = std::time::Instant::now();

                                // Use a small timeout to simulate ping
                                let _ = timeout(
                                    Duration::from_millis(100),
                                    sleep(Duration::from_millis(1)),
                                )
                                .await;

                                let latency_ms = start.elapsed().as_millis() as f64;

                                // Update metrics collector
                                let mut collectors = metrics_collectors.write().await;
                                if let Some(collector) = collectors.get_mut(&guild_id_clone) {
                                    collector.start_ping();
                                    let _ = collector.complete_ping();
                                }

                                debug!(
                                    "Ping measurement completed for guild {}: {}ms",
                                    guild_id_clone, latency_ms
                                );
                            }
                        }
                    });
                }
                _ => {} // Handle other events as needed
            }
            result.last_check = current_timestamp();
        }
    }
}

/// Monitoring summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitoringSummary {
    /// Total number of monitored guilds
    pub total_monitored_guilds: u32,
    /// Number of healthy connections
    pub healthy_connections: u32,
    /// Number of degraded connections
    pub degraded_connections: u32,
    /// Number of unhealthy connections
    pub unhealthy_connections: u32,
    /// Number of critical connections
    pub critical_connections: u32,
    /// Number of unknown status connections
    pub unknown_connections: u32,
    /// Average latency across all connections
    pub avg_latency_ms: f64,
    /// Average packet loss across all connections
    pub avg_packet_loss_percent: f64,
    /// Total number of alerts
    pub total_alerts: u32,
    /// Number of unacknowledged alerts
    pub unacknowledged_alerts: u32,
}

impl Default for VoiceConnectionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

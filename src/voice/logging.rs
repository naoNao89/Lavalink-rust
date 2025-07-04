//! Comprehensive logging utilities for voice connection events
//!
//! This module provides structured logging, correlation tracking, performance metrics,
//! and enhanced error context for the voice connection system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{error, info, trace, warn};
use uuid::Uuid;

/// Global correlation ID counter for generating unique IDs
static CORRELATION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Voice event correlation ID for tracking related events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

#[allow(dead_code)]
impl CorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> Self {
        let counter = CORRELATION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let uuid = Uuid::new_v4();
        Self(format!("voice-{}-{}", counter, uuid.simple()))
    }

    /// Create correlation ID from string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the correlation ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Performance timing utility for measuring operation durations
#[derive(Debug)]
pub struct PerformanceTimer {
    operation: String,
    start_time: Instant,
    correlation_id: CorrelationId,
    guild_id: Option<String>,
}

#[allow(dead_code)]
impl PerformanceTimer {
    /// Start timing an operation
    pub fn start(operation: &str, correlation_id: CorrelationId, guild_id: Option<String>) -> Self {
        trace!(
            correlation_id = %correlation_id,
            guild_id = guild_id.as_deref().unwrap_or("unknown"),
            operation = operation,
            "Starting performance timer"
        );

        Self {
            operation: operation.to_string(),
            start_time: Instant::now(),
            correlation_id,
            guild_id,
        }
    }

    /// Complete timing and log the duration
    pub fn complete(self) -> Duration {
        let duration = self.start_time.elapsed();

        info!(
            correlation_id = %self.correlation_id,
            guild_id = self.guild_id.as_deref().unwrap_or("unknown"),
            operation = %self.operation,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        duration
    }

    /// Complete timing with custom log level and additional context
    pub fn complete_with_context(
        self,
        success: bool,
        context: HashMap<String, String>,
    ) -> Duration {
        let duration = self.start_time.elapsed();

        let log_level = if success { "info" } else { "warn" };

        match log_level {
            "info" => info!(
                correlation_id = %self.correlation_id,
                guild_id = self.guild_id.as_deref().unwrap_or("unknown"),
                operation = %self.operation,
                duration_ms = duration.as_millis(),
                success = success,
                context = ?context,
                "Operation completed with context"
            ),
            "warn" => warn!(
                correlation_id = %self.correlation_id,
                guild_id = self.guild_id.as_deref().unwrap_or("unknown"),
                operation = %self.operation,
                duration_ms = duration.as_millis(),
                success = success,
                context = ?context,
                "Operation completed with warnings"
            ),
            _ => {}
        }

        duration
    }
}

/// Voice event types for structured logging
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoiceEventType {
    // Connection Events
    ConnectionStart,
    ConnectionEstablished,
    ConnectionFailed,
    ConnectionClosed,
    ConnectionRetry,
    ConnectionTimeout,

    // Gateway Events
    GatewayReady,
    GatewayClosed,
    GatewayError,
    GatewayReconnect,

    // Audio Events
    AudioStreamStart,
    AudioStreamStop,
    AudioStreamPause,
    AudioStreamResume,
    StreamStart,
    StreamEnd,
    AudioQualityChange,
    BitrateAdjustment,
    AudioBufferUnderrun,
    AudioBufferOverrun,

    // Recovery Events
    RecoveryAttempt,
    RecoverySuccess,
    RecoveryFailure,
    RecoveryAborted,

    // Circuit Breaker Events
    CircuitBreakerOpen,
    CircuitBreakerClosed,
    CircuitBreakerHalfOpen,

    // Monitoring Events
    HealthCheck,
    HealthCheckFailed,
    AlertGenerated,
    AlertResolved,
    MetricsCollection,
    PerformanceWarning,

    // Pool Events
    PoolOperation,
    PoolConnectionCreated,
    PoolConnectionDestroyed,
    PoolConnectionReused,
    PoolCleanup,

    // State Events
    StateTransition,
    SpeakingStateChanged,

    // Error Events
    ErrorOccurred,
    ErrorRecovered,
    CriticalError,
}

/// Structured voice event for consistent logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceEvent {
    pub correlation_id: CorrelationId,
    pub event_type: VoiceEventType,
    pub guild_id: String,
    pub timestamp: u64,
    pub details: HashMap<String, String>,
    pub metrics: Option<HashMap<String, f64>>,
}

#[allow(dead_code)]
impl VoiceEvent {
    /// Create a new voice event
    pub fn new(
        correlation_id: CorrelationId,
        event_type: VoiceEventType,
        guild_id: String,
    ) -> Self {
        Self {
            correlation_id,
            event_type,
            guild_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            details: HashMap::new(),
            metrics: None,
        }
    }

    /// Add detail to the event
    pub fn with_detail(mut self, key: &str, value: &str) -> Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }

    /// Add multiple details to the event
    pub fn with_details(mut self, details: HashMap<String, String>) -> Self {
        self.details.extend(details);
        self
    }

    /// Add metrics to the event
    pub fn with_metrics(mut self, metrics: HashMap<String, f64>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Log the event at the appropriate level
    pub fn log(self) {
        match self.event_type {
            // Error level events
            VoiceEventType::ConnectionFailed
            | VoiceEventType::ConnectionTimeout
            | VoiceEventType::GatewayError
            | VoiceEventType::CircuitBreakerOpen
            | VoiceEventType::RecoveryFailure
            | VoiceEventType::RecoveryAborted
            | VoiceEventType::HealthCheckFailed
            | VoiceEventType::AudioBufferUnderrun
            | VoiceEventType::AudioBufferOverrun
            | VoiceEventType::ErrorOccurred
            | VoiceEventType::CriticalError => {
                error!(
                    correlation_id = %self.correlation_id,
                    guild_id = %self.guild_id,
                    event_type = ?self.event_type,
                    timestamp = self.timestamp,
                    details = ?self.details,
                    metrics = ?self.metrics,
                    "Voice event occurred"
                );
            }
            // Warning level events
            VoiceEventType::RecoveryAttempt
            | VoiceEventType::AudioQualityChange
            | VoiceEventType::BitrateAdjustment
            | VoiceEventType::GatewayClosed
            | VoiceEventType::GatewayReconnect
            | VoiceEventType::ConnectionRetry
            | VoiceEventType::CircuitBreakerHalfOpen
            | VoiceEventType::AlertGenerated
            | VoiceEventType::PerformanceWarning
            | VoiceEventType::PoolCleanup => {
                warn!(
                    correlation_id = %self.correlation_id,
                    guild_id = %self.guild_id,
                    event_type = ?self.event_type,
                    timestamp = self.timestamp,
                    details = ?self.details,
                    metrics = ?self.metrics,
                    "Voice event occurred"
                );
            }
            // Info level events (default)
            _ => {
                info!(
                    correlation_id = %self.correlation_id,
                    guild_id = %self.guild_id,
                    event_type = ?self.event_type,
                    timestamp = self.timestamp,
                    details = ?self.details,
                    metrics = ?self.metrics,
                    "Voice event occurred"
                );
            }
        }
    }
}

/// Error context builder for enhanced error logging
#[derive(Debug, Clone)]
pub struct VoiceErrorContext {
    pub correlation_id: CorrelationId,
    pub guild_id: String,
    pub operation: String,
    pub error_type: String,
    pub context: HashMap<String, String>,
    pub troubleshooting_hints: Vec<String>,
}

#[allow(dead_code)]
impl VoiceErrorContext {
    /// Create a new error context
    pub fn new(
        correlation_id: CorrelationId,
        guild_id: String,
        operation: String,
        error_type: String,
    ) -> Self {
        Self {
            correlation_id,
            guild_id,
            operation,
            error_type,
            context: HashMap::new(),
            troubleshooting_hints: Vec::new(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }

    /// Add troubleshooting hint
    pub fn with_hint(mut self, hint: &str) -> Self {
        self.troubleshooting_hints.push(hint.to_string());
        self
    }

    /// Log the error with full context
    pub fn log_error(self, error: &anyhow::Error) {
        error!(
            correlation_id = %self.correlation_id,
            guild_id = %self.guild_id,
            operation = %self.operation,
            error_type = %self.error_type,
            error = %error,
            context = ?self.context,
            troubleshooting_hints = ?self.troubleshooting_hints,
            "Voice operation failed with context"
        );
    }
}

/// Macro for logging voice connection events
#[macro_export]
macro_rules! log_voice_connection {
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_connection",
            $message
        );
    };
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr, $($key:ident = $value:expr),+) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_connection",
            $($key = $value),+,
            $message
        );
    };
}

/// Macro for logging voice audio events
#[macro_export]
macro_rules! log_voice_audio {
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_audio",
            $message
        );
    };
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr, $($key:ident = $value:expr),+) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_audio",
            $($key = $value),+,
            $message
        );
    };
}

/// Macro for logging voice pool events
#[macro_export]
macro_rules! log_voice_pool {
    ($level:ident, $correlation_id:expr, $message:expr) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            component = "voice_pool",
            $message
        );
    };
    ($level:ident, $correlation_id:expr, $message:expr, $($key:ident = $value:expr),+) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            component = "voice_pool",
            $($key = $value),+,
            $message
        );
    };
}

/// Macro for logging voice monitoring events
#[macro_export]
macro_rules! log_voice_monitoring {
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_monitoring",
            $message
        );
    };
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr, $($key:ident = $value:expr),+) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_monitoring",
            $($key = $value),+,
            $message
        );
    };
}

/// Macro for logging voice quality events
#[macro_export]
macro_rules! log_voice_quality {
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_quality",
            $message
        );
    };
    ($level:ident, $correlation_id:expr, $guild_id:expr, $message:expr, $($key:ident = $value:expr),+) => {
        tracing::$level!(
            correlation_id = %$correlation_id,
            guild_id = %$guild_id,
            component = "voice_quality",
            $($key = $value),+,
            $message
        );
    };
}

/// Comprehensive voice event logger
pub struct VoiceEventLogger {
    correlation_id: CorrelationId,
    guild_id: String,
}

impl VoiceEventLogger {
    /// Create a new voice event logger
    pub fn new(guild_id: String) -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            guild_id,
        }
    }

    /// Log a connection event
    pub fn log_connection_event(
        &self,
        event_type: VoiceEventType,
        details: HashMap<String, String>,
        metrics: Option<HashMap<String, f64>>,
    ) {
        let event = VoiceEvent::new(
            self.correlation_id.clone(),
            event_type,
            self.guild_id.clone(),
        )
        .with_details(details)
        .with_metrics(metrics.unwrap_or_default());
        event.log();
    }

    /// Log a gateway event with specific details
    pub fn log_gateway_event(
        &self,
        event_type: VoiceEventType,
        ssrc: Option<u32>,
        ip: Option<String>,
        port: Option<u16>,
        code: Option<i32>,
        reason: Option<String>,
    ) {
        let mut details = HashMap::new();

        if let Some(ssrc) = ssrc {
            details.insert("ssrc".to_string(), ssrc.to_string());
        }
        if let Some(ip) = ip {
            details.insert("ip".to_string(), ip);
        }
        if let Some(port) = port {
            details.insert("port".to_string(), port.to_string());
        }
        if let Some(code) = code {
            details.insert("code".to_string(), code.to_string());
        }
        if let Some(reason) = reason {
            details.insert("reason".to_string(), reason);
        }

        self.log_connection_event(event_type, details, None);
    }

    /// Log a recovery event
    pub fn log_recovery_event(
        &self,
        event_type: VoiceEventType,
        attempt: Option<u32>,
        total_attempts: Option<u32>,
        delay: Option<Duration>,
        error: Option<String>,
    ) {
        let mut details = HashMap::new();
        let mut metrics = HashMap::new();

        if let Some(attempt) = attempt {
            details.insert("attempt".to_string(), attempt.to_string());
            metrics.insert("attempt".to_string(), attempt as f64);
        }
        if let Some(total_attempts) = total_attempts {
            details.insert("total_attempts".to_string(), total_attempts.to_string());
            metrics.insert("total_attempts".to_string(), total_attempts as f64);
        }
        if let Some(delay) = delay {
            details.insert("delay_ms".to_string(), delay.as_millis().to_string());
            metrics.insert("delay_ms".to_string(), delay.as_millis() as f64);
        }
        if let Some(error) = error {
            details.insert("error".to_string(), error);
        }

        self.log_connection_event(event_type, details, Some(metrics));
    }

    /// Log a monitoring event with health metrics
    pub fn log_monitoring_event(
        &self,
        event_type: VoiceEventType,
        health_status: Option<String>,
        latency_ms: Option<f64>,
        packet_loss: Option<f64>,
        quality_score: Option<u8>,
    ) {
        let mut details = HashMap::new();
        let mut metrics = HashMap::new();

        if let Some(status) = health_status {
            details.insert("health_status".to_string(), status);
        }
        if let Some(latency) = latency_ms {
            metrics.insert("latency_ms".to_string(), latency);
            details.insert("latency_ms".to_string(), format!("{latency:.2}"));
        }
        if let Some(loss) = packet_loss {
            metrics.insert("packet_loss_percent".to_string(), loss);
            details.insert("packet_loss_percent".to_string(), format!("{loss:.2}"));
        }
        if let Some(quality) = quality_score {
            metrics.insert("quality_score".to_string(), quality as f64);
            details.insert("quality_score".to_string(), quality.to_string());
        }

        self.log_connection_event(event_type, details, Some(metrics));
    }

    /// Log an error event with context
    pub fn log_error_event(
        &self,
        error: &anyhow::Error,
        operation: &str,
        error_type: &str,
        context: HashMap<String, String>,
    ) {
        let mut error_context = VoiceErrorContext::new(
            self.correlation_id.clone(),
            self.guild_id.clone(),
            operation.to_string(),
            error_type.to_string(),
        );

        // Add all context key-value pairs
        for (key, value) in context {
            error_context = error_context.with_context(&key, &value);
        }

        error_context.log_error(error);
    }
}

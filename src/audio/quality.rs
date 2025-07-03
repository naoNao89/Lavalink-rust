// Audio quality and bitrate management for Discord voice streaming
// Provides configuration and control over audio quality parameters

use anyhow::Result;
use serde::{Deserialize, Serialize};
use songbird::{
    driver::{Channels, MixMode, SampleRate},
    Config as SongbirdConfig,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use tracing::{debug, error, info, warn};

/// Audio quality configuration for Discord voice streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioQualityConfig {
    /// Audio bitrate in kbps (64-320)
    pub bitrate: u32,
    /// Sample rate for audio processing
    pub sample_rate: AudioSampleRate,
    /// Channel configuration (mono/stereo)
    pub channels: AudioChannels,
    /// Audio quality preset
    pub quality_preset: QualityPreset,
    /// Enable soft clipping to prevent distortion
    pub soft_clipping: bool,
    /// Buffer configuration for jitter smoothing
    pub buffer_config: BufferConfig,
    /// Opus encoding quality (0-10, 10 = highest)
    pub opus_quality: u8,
    /// Enable automatic quality adjustment based on network conditions
    pub adaptive_quality: bool,
}

/// Audio sample rate options
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioSampleRate {
    /// 8 kHz - Very low quality, minimal bandwidth
    Hz8000,
    /// 16 kHz - Low quality, voice optimized
    Hz16000,
    /// 24 kHz - Medium quality
    Hz24000,
    /// 48 kHz - High quality, Discord standard
    Hz48000,
    /// 96 kHz - Very high quality (not recommended for Discord)
    Hz96000,
}

/// Audio channel configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioChannels {
    /// Mono audio (1 channel)
    Mono,
    /// Stereo audio (2 channels)
    Stereo,
}

/// Audio quality presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QualityPreset {
    /// Minimal quality for voice chat (64 kbps, mono, 16 kHz)
    Voice,
    /// Low quality for music (96 kbps, stereo, 48 kHz)
    Low,
    /// Medium quality for general use (128 kbps, stereo, 48 kHz)
    Medium,
    /// High quality for music (192 kbps, stereo, 48 kHz)
    High,
    /// Maximum quality (320 kbps, stereo, 48 kHz)
    Maximum,
    /// Custom configuration
    Custom,
}

/// Buffer configuration for audio streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// Playout buffer length in packets (affects latency vs stability)
    pub playout_buffer_length: usize,
    /// Additional buffer space for packet bursts
    pub playout_spike_length: usize,
    /// Timeout for decoder state cleanup
    pub decode_state_timeout_ms: u64,
}

/// Audio quality manager for runtime quality control
#[allow(dead_code)] // Fields used in complex async quality management
pub struct AudioQualityManager {
    /// Current quality configuration
    config: AudioQualityConfig,
    /// Guild ID this manager belongs to
    guild_id: String,
    /// Network quality metrics
    network_metrics: NetworkMetrics,
    /// Real-time quality monitoring metrics
    quality_metrics: Arc<RwLock<QualityMetrics>>,
    /// Quality monitoring configuration
    monitoring_config: QualityMonitoringConfig,
    /// Bitrate adjustment configuration
    adjustment_config: BitrateAdjustmentConfig,
    /// Historical quality data for trend analysis
    quality_history: Arc<RwLock<VecDeque<QualityDataPoint>>>,
    /// Quality adjustment state tracking
    adjustment_state: Arc<RwLock<QualityAdjustmentState>>,
    /// Last quality adjustment timestamp
    last_adjustment: Arc<RwLock<Option<Instant>>>,
    /// Quality alert callback
    alert_callback: Option<Box<dyn Fn(QualityAlert, String) + Send + Sync>>,
}

/// Network quality metrics for adaptive quality adjustment
#[derive(Debug, Clone, Default)]
pub struct NetworkMetrics {
    /// Average packet loss percentage (0.0-100.0)
    pub packet_loss: f32,
    /// Average round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Jitter in milliseconds
    pub jitter_ms: u32,
    /// Bandwidth estimate in kbps
    pub bandwidth_kbps: u32,
}

/// Real-time quality monitoring metrics
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in complex async patterns and external APIs
pub struct QualityMetrics {
    /// Current effective bitrate (kbps)
    pub effective_bitrate: u32,
    /// Buffer health percentage (0-100)
    pub buffer_health: u8,
    /// Encoding performance score (0-100)
    pub encoding_performance: u8,
    /// Stream stability score (0-100)
    pub stream_stability: u8,
    /// Quality degradation events in last minute
    pub degradation_events: u32,
    /// Average quality score over time window
    pub average_quality_score: u8,
    /// Timestamp of last update
    pub last_update: Instant,
    /// Quality trend (Improving, Stable, Degrading)
    pub quality_trend: QualityTrend,
}

/// Quality trend indicators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityTrend {
    /// Quality is improving over time
    Improving,
    /// Quality is stable
    Stable,
    /// Quality is degrading over time
    Degrading,
}

/// Quality alert levels
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // Variants used in quality degradation handling
pub enum QualityAlert {
    Warning,
    Critical,
}

/// Quality monitoring configuration
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in quality monitoring logic
pub struct QualityMonitoringConfig {
    /// Monitoring interval in seconds
    pub monitoring_interval: Duration,
    /// History window size for trend analysis
    pub history_window_size: usize,
    /// Threshold for quality degradation alert
    pub degradation_threshold: u8,
    /// Threshold for critical quality alert
    pub critical_threshold: u8,
    /// Enable automatic quality adjustment
    pub auto_adjustment_enabled: bool,
    /// Minimum time between quality adjustments
    pub adjustment_cooldown: Duration,
}

/// Dynamic bitrate adjustment configuration
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in quality adjustment algorithms
pub struct BitrateAdjustmentConfig {
    /// Enable gradual quality transitions
    pub gradual_transitions: bool,
    /// Hysteresis margin to prevent oscillation (percentage)
    pub hysteresis_margin: f32,
    /// Maximum bitrate change per adjustment (kbps)
    pub max_bitrate_change: u32,
    /// Adjustment sensitivity (0.0-1.0, higher = more sensitive)
    pub adjustment_sensitivity: f32,
    /// Custom adjustment policy
    pub adjustment_policy: AdjustmentPolicy,
    /// Minimum stable period before allowing upgrades
    pub upgrade_stability_period: Duration,
    /// Emergency downgrade threshold (immediate action)
    pub emergency_threshold: u8,
}

/// Bitrate adjustment policies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdjustmentPolicy {
    /// Balanced: Moderate adjustments, balance quality and stability
    Balanced,
}

/// Quality adjustment state tracking
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in quality adjustment state management
struct QualityAdjustmentState {
    /// Current adjustment phase
    current_phase: AdjustmentPhase,
    /// Time when current quality became stable
    stable_since: Option<Instant>,
    /// Number of consecutive adjustments in same direction
    adjustment_streak: u32,
    /// Recent adjustment history for hysteresis
    recent_adjustments: VecDeque<QualityAdjustment>,
}

/// Adjustment phase tracking
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // Variants used in adjustment phase management
enum AdjustmentPhase {
    Stable,
    Degrading,
    Recovering,
    Emergency,
}

/// Reasons for quality adjustments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Variants used throughout quality adjustment logic
pub enum AdjustmentReason {
    NetworkDegradation,
    NetworkImprovement,
    UserRequest,
    Emergency,
}

/// Quality adjustment record
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in adjustment history tracking
struct QualityAdjustment {
    timestamp: Instant,
    from_preset: QualityPreset,
    to_preset: QualityPreset,
}

/// Historical quality data point
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in trend analysis calculations
struct QualityDataPoint {
    quality_score: u8,
}

impl Default for AudioQualityConfig {
    fn default() -> Self {
        Self {
            bitrate: 128,
            sample_rate: AudioSampleRate::Hz48000,
            channels: AudioChannels::Stereo,
            quality_preset: QualityPreset::Medium,
            soft_clipping: true,
            buffer_config: BufferConfig::default(),
            opus_quality: 8, // High quality Opus encoding
            adaptive_quality: true,
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            playout_buffer_length: 5,       // 100ms at 20ms per packet
            playout_spike_length: 3,        // Additional 60ms for bursts
            decode_state_timeout_ms: 60000, // 1 minute
        }
    }
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            effective_bitrate: 128,
            buffer_health: 100,
            encoding_performance: 100,
            stream_stability: 100,
            degradation_events: 0,
            average_quality_score: 100,
            last_update: Instant::now(),
            quality_trend: QualityTrend::Stable,
        }
    }
}

impl Default for QualityMonitoringConfig {
    fn default() -> Self {
        Self {
            monitoring_interval: Duration::from_secs(5),
            history_window_size: 60, // 5 minutes at 5-second intervals
            degradation_threshold: 70,
            critical_threshold: 50,
            auto_adjustment_enabled: true,
            adjustment_cooldown: Duration::from_secs(30),
        }
    }
}

impl Default for BitrateAdjustmentConfig {
    fn default() -> Self {
        Self {
            gradual_transitions: true,
            hysteresis_margin: 10.0,     // 10% margin
            max_bitrate_change: 64,      // Max 64 kbps change per adjustment
            adjustment_sensitivity: 0.7, // Moderate sensitivity
            adjustment_policy: AdjustmentPolicy::Balanced,
            upgrade_stability_period: Duration::from_secs(60),
            emergency_threshold: 30,
        }
    }
}

impl Default for QualityAdjustmentState {
    fn default() -> Self {
        Self {
            current_phase: AdjustmentPhase::Stable,
            stable_since: Some(Instant::now()),
            adjustment_streak: 0,
            recent_adjustments: VecDeque::new(),
        }
    }
}

impl AdjustmentPolicy {}

impl AudioSampleRate {
    /// Convert to Songbird SampleRate enum
    pub fn to_songbird(self) -> SampleRate {
        match self {
            AudioSampleRate::Hz8000 => SampleRate::Hz8000,
            AudioSampleRate::Hz16000 => SampleRate::Hz16000,
            AudioSampleRate::Hz24000 => SampleRate::Hz24000,
            AudioSampleRate::Hz48000 => SampleRate::Hz48000,
            // Map 96kHz to 48kHz as Songbird doesn't support 96kHz
            AudioSampleRate::Hz96000 => SampleRate::Hz48000,
        }
    }

    /// Get sample rate as u32 for other uses
    #[allow(dead_code)]
    pub fn as_u32(&self) -> u32 {
        match self {
            AudioSampleRate::Hz8000 => 8000,
            AudioSampleRate::Hz16000 => 16000,
            AudioSampleRate::Hz24000 => 24000,
            AudioSampleRate::Hz48000 => 48000,
            AudioSampleRate::Hz96000 => 96000,
        }
    }
}

impl AudioChannels {
    /// Convert to Songbird MixMode enum
    pub fn to_mix_mode(self) -> MixMode {
        match self {
            AudioChannels::Mono => MixMode::Mono,
            AudioChannels::Stereo => MixMode::Stereo,
        }
    }

    /// Get channel count as u8
    #[allow(dead_code)]
    pub fn count(&self) -> u8 {
        match self {
            AudioChannels::Mono => 1,
            AudioChannels::Stereo => 2,
        }
    }
}

impl QualityPreset {
    /// Get the audio quality configuration for this preset
    pub fn to_config(self) -> AudioQualityConfig {
        match self {
            QualityPreset::Voice => AudioQualityConfig {
                bitrate: 64,
                sample_rate: AudioSampleRate::Hz16000,
                channels: AudioChannels::Mono,
                quality_preset: self,
                soft_clipping: true,
                buffer_config: BufferConfig {
                    playout_buffer_length: 3, // Lower latency for voice
                    playout_spike_length: 2,
                    decode_state_timeout_ms: 30000,
                },
                opus_quality: 6, // Medium quality for voice
                adaptive_quality: true,
            },
            QualityPreset::Low => AudioQualityConfig {
                bitrate: 96,
                sample_rate: AudioSampleRate::Hz48000,
                channels: AudioChannels::Stereo,
                quality_preset: self,
                soft_clipping: true,
                buffer_config: BufferConfig::default(),
                opus_quality: 7,
                adaptive_quality: true,
            },
            QualityPreset::Medium => AudioQualityConfig::default(),
            QualityPreset::High => AudioQualityConfig {
                bitrate: 192,
                sample_rate: AudioSampleRate::Hz48000,
                channels: AudioChannels::Stereo,
                quality_preset: self,
                soft_clipping: true,
                buffer_config: BufferConfig {
                    playout_buffer_length: 7, // Higher stability
                    playout_spike_length: 4,
                    decode_state_timeout_ms: 90000,
                },
                opus_quality: 9,
                adaptive_quality: true,
            },
            QualityPreset::Maximum => AudioQualityConfig {
                bitrate: 320,
                sample_rate: AudioSampleRate::Hz48000,
                channels: AudioChannels::Stereo,
                quality_preset: self,
                soft_clipping: true,
                buffer_config: BufferConfig {
                    playout_buffer_length: 10, // Maximum stability
                    playout_spike_length: 5,
                    decode_state_timeout_ms: 120000,
                },
                opus_quality: 10,        // Maximum Opus quality
                adaptive_quality: false, // Don't downgrade from maximum
            },
            QualityPreset::Custom => AudioQualityConfig::default(),
        }
    }
}

#[allow(dead_code)] // Methods used in complex async quality management patterns
impl AudioQualityManager {
    /// Create a new audio quality manager
    pub fn new(guild_id: String, config: AudioQualityConfig) -> Self {
        info!(
            "Creating audio quality manager for guild {} with preset {:?}",
            guild_id, config.quality_preset
        );

        let adjustment_state = QualityAdjustmentState::default();

        Self {
            config,
            guild_id,
            network_metrics: NetworkMetrics::default(),
            quality_metrics: Arc::new(RwLock::new(QualityMetrics::default())),
            monitoring_config: QualityMonitoringConfig::default(),
            adjustment_config: BitrateAdjustmentConfig::default(),
            quality_history: Arc::new(RwLock::new(VecDeque::new())),
            adjustment_state: Arc::new(RwLock::new(adjustment_state)),
            last_adjustment: Arc::new(RwLock::new(None)),
            alert_callback: None,
        }
    }

    /// Create with a quality preset
    pub fn with_preset(guild_id: String, preset: QualityPreset) -> Self {
        Self::new(guild_id, preset.to_config())
    }

    /// Get current quality configuration
    pub fn get_config(&self) -> &AudioQualityConfig {
        &self.config
    }

    /// Update quality configuration
    pub fn update_config(&mut self, config: AudioQualityConfig) -> Result<()> {
        info!(
            "Updating audio quality config for guild {}: {:?} -> {:?}",
            self.guild_id, self.config.quality_preset, config.quality_preset
        );

        // Validate configuration
        self.validate_config(&config)?;

        self.config = config;
        Ok(())
    }

    /// Apply quality preset
    pub fn apply_preset(&mut self, preset: QualityPreset) -> Result<()> {
        let config = preset.to_config();
        self.update_config(config)
    }

    /// Update network metrics for adaptive quality
    pub fn update_network_metrics(&mut self, metrics: NetworkMetrics) {
        debug!(
            "Updating network metrics for guild {}: loss={:.1}%, rtt={}ms, jitter={}ms, bandwidth={}kbps",
            self.guild_id, metrics.packet_loss, metrics.rtt_ms, metrics.jitter_ms, metrics.bandwidth_kbps
        );

        self.network_metrics = metrics;

        // Apply adaptive quality adjustment if enabled
        if self.config.adaptive_quality {
            // Note: This would need to be called from an async context in practice
            // For now, we'll log that adjustment is needed
            debug!(
                "Network conditions changed for guild {}, adjustment may be needed",
                self.guild_id
            );
        }
    }

    /// Enhanced quality adjustment with dynamic bitrate management
    async fn adjust_quality_for_network(&mut self) -> Result<()> {
        let metrics = &self.network_metrics;

        // Don't adjust if using custom preset
        if matches!(self.config.quality_preset, QualityPreset::Custom) {
            return Ok(());
        }

        // Calculate quality scores for decision making
        let network_score = self.network_quality_score();
        let quality_metrics = self.quality_metrics.read().await;
        let overall_score =
            (network_score as f32 * 0.6 + quality_metrics.average_quality_score as f32 * 0.4) as u8;
        drop(quality_metrics);

        // Determine adjustment need using enhanced logic
        let adjustment_decision = self
            .determine_quality_adjustment(overall_score, metrics)
            .await?;

        match adjustment_decision {
            Some((target_preset, reason)) => {
                self.execute_quality_adjustment(target_preset, reason)
                    .await?;
            }
            None => {
                // Update stable state if no adjustment needed
                self.update_stable_state().await;
            }
        }

        Ok(())
    }

    /// Determine if quality adjustment is needed using enhanced algorithms
    async fn determine_quality_adjustment(
        &self,
        overall_score: u8,
        metrics: &NetworkMetrics,
    ) -> Result<Option<(QualityPreset, AdjustmentReason)>> {
        let mut state = self.adjustment_state.write().await;
        let current_preset = self.config.quality_preset;

        // Check for emergency conditions first
        if overall_score < self.adjustment_config.emergency_threshold {
            state.current_phase = AdjustmentPhase::Emergency;
            return Ok(Some((QualityPreset::Voice, AdjustmentReason::Emergency)));
        }

        // Apply hysteresis to prevent oscillation
        if self.should_apply_hysteresis(&state, overall_score).await {
            debug!(
                "Hysteresis preventing quality adjustment for guild {}",
                self.guild_id
            );
            return Ok(None);
        }

        // Determine adjustment based on current phase and conditions
        let adjustment = match state.current_phase {
            AdjustmentPhase::Stable => {
                self.evaluate_stable_phase_adjustment(overall_score, metrics, &state)
                    .await
            }
            AdjustmentPhase::Degrading => {
                self.evaluate_degrading_phase_adjustment(overall_score, metrics, &state)
                    .await
            }
            AdjustmentPhase::Recovering => {
                self.evaluate_recovering_phase_adjustment(overall_score, metrics, &state)
                    .await
            }
            AdjustmentPhase::Emergency => {
                self.evaluate_emergency_phase_adjustment(overall_score, metrics, &state)
                    .await
            }
        };

        // Update phase based on decision
        if let Some((target_preset, reason)) = &adjustment {
            state.current_phase = match reason {
                AdjustmentReason::Emergency => AdjustmentPhase::Emergency,
                AdjustmentReason::NetworkDegradation => {
                    if self.is_downgrade(current_preset, *target_preset) {
                        AdjustmentPhase::Degrading
                    } else {
                        AdjustmentPhase::Recovering
                    }
                }
                AdjustmentReason::NetworkImprovement => AdjustmentPhase::Recovering,
                AdjustmentReason::UserRequest => state.current_phase, // Keep current phase
            };
        }

        Ok(adjustment)
    }

    /// Execute quality adjustment with gradual transitions if enabled
    async fn execute_quality_adjustment(
        &mut self,
        target_preset: QualityPreset,
        reason: AdjustmentReason,
    ) -> Result<()> {
        let current_preset = self.config.quality_preset;

        if current_preset == target_preset {
            return Ok(());
        }

        info!(
            "Executing quality adjustment for guild {}: {:?} -> {:?} (reason: {:?})",
            self.guild_id, current_preset, target_preset, reason
        );

        // Record adjustment attempt
        let adjustment_record = QualityAdjustment {
            timestamp: Instant::now(),
            from_preset: current_preset,
            to_preset: target_preset,
        };

        let success = if self.adjustment_config.gradual_transitions
            && !matches!(reason, AdjustmentReason::Emergency)
        {
            self.execute_gradual_transition(current_preset, target_preset)
                .await
        } else {
            self.apply_preset(target_preset).is_ok()
        };

        // Update adjustment state
        let mut state = self.adjustment_state.write().await;
        state.recent_adjustments.push_back(adjustment_record);

        // Maintain adjustment history size
        while state.recent_adjustments.len() > 10 {
            state.recent_adjustments.pop_front();
        }

        if success {
            state.adjustment_streak =
                if self.is_same_direction(&state, current_preset, target_preset) {
                    state.adjustment_streak + 1
                } else {
                    1
                };

            state.stable_since = None; // Reset stability timer

            info!("Quality adjustment successful for guild {}", self.guild_id);
        } else {
            error!("Quality adjustment failed for guild {}", self.guild_id);
        }

        Ok(())
    }

    /// Execute gradual quality transition
    async fn execute_gradual_transition(
        &mut self,
        from_preset: QualityPreset,
        to_preset: QualityPreset,
    ) -> bool {
        let steps = self.calculate_transition_steps(from_preset, to_preset);

        for step_preset in steps {
            if self.apply_preset(step_preset).is_err() {
                warn!(
                    "Gradual transition step failed for guild {}: {:?}",
                    self.guild_id, step_preset
                );
                return false;
            }

            // Small delay between steps for smooth transition
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        true
    }

    /// Calculate intermediate steps for gradual transition
    fn calculate_transition_steps(
        &self,
        from: QualityPreset,
        to: QualityPreset,
    ) -> Vec<QualityPreset> {
        let presets = [
            QualityPreset::Voice,
            QualityPreset::Low,
            QualityPreset::Medium,
            QualityPreset::High,
            QualityPreset::Maximum,
        ];

        let from_idx = presets.iter().position(|&p| p == from).unwrap_or(2);
        let to_idx = presets.iter().position(|&p| p == to).unwrap_or(2);

        if from_idx == to_idx {
            return vec![];
        }

        let mut steps = Vec::new();
        if from_idx < to_idx {
            // Upgrading
            for preset in presets.iter().take(to_idx + 1).skip(from_idx + 1) {
                steps.push(*preset);
            }
        } else {
            // Downgrading
            for i in ((to_idx)..from_idx).rev() {
                steps.push(presets[i]);
            }
        }

        steps
    }

    /// Validate audio quality configuration
    fn validate_config(&self, config: &AudioQualityConfig) -> Result<()> {
        // Validate bitrate range
        if config.bitrate < 32 || config.bitrate > 320 {
            return Err(anyhow::anyhow!(
                "Invalid bitrate: {}. Must be between 32 and 320 kbps",
                config.bitrate
            ));
        }

        // Validate Opus quality range
        if config.opus_quality > 10 {
            return Err(anyhow::anyhow!(
                "Invalid Opus quality: {}. Must be between 0 and 10",
                config.opus_quality
            ));
        }

        // Validate buffer configuration
        if config.buffer_config.playout_buffer_length == 0 {
            return Err(anyhow::anyhow!(
                "Playout buffer length must be greater than 0"
            ));
        }

        if config.buffer_config.playout_buffer_length > 64 {
            return Err(anyhow::anyhow!(
                "Playout buffer length too large: {}. Maximum is 64 packets",
                config.buffer_config.playout_buffer_length
            ));
        }

        Ok(())
    }

    /// Create Songbird configuration from audio quality settings
    pub fn create_songbird_config(&self) -> SongbirdConfig {
        let config = &self.config;

        debug!(
            "Creating Songbird config for guild {}: {}kbps, {:?}, {:?}",
            self.guild_id, config.bitrate, config.sample_rate, config.channels
        );

        // Create config using builder pattern since Config is non-exhaustive
        SongbirdConfig::default()
            .mix_mode(config.channels.to_mix_mode())
            .decode_channels(match config.channels {
                AudioChannels::Mono => Channels::Mono,
                AudioChannels::Stereo => Channels::Stereo,
            })
            .decode_sample_rate(config.sample_rate.to_songbird())
            .use_softclip(config.soft_clipping)
            .playout_buffer_length(
                std::num::NonZeroUsize::new(config.buffer_config.playout_buffer_length)
                    .unwrap_or(std::num::NonZeroUsize::new(5).unwrap()),
            )
            .playout_spike_length(config.buffer_config.playout_spike_length)
    }

    /// Get estimated bandwidth usage in kbps
    pub fn estimated_bandwidth(&self) -> u32 {
        // Base bitrate plus overhead for RTP/UDP headers and Discord protocol
        let overhead_factor = 1.2; // 20% overhead estimate
        (self.config.bitrate as f32 * overhead_factor) as u32
    }

    /// Get current network quality score (0-100)
    pub fn network_quality_score(&self) -> u8 {
        let metrics = &self.network_metrics;

        // Calculate score based on network metrics
        let loss_score = ((100.0 - metrics.packet_loss.min(100.0)) / 100.0 * 40.0) as u8;
        let rtt_score = ((300.0 - metrics.rtt_ms.min(300) as f32) / 300.0 * 30.0) as u8;
        let jitter_score = ((100.0 - metrics.jitter_ms.min(100) as f32) / 100.0 * 20.0) as u8;
        let bandwidth_score = if metrics.bandwidth_kbps >= self.estimated_bandwidth() * 2 {
            10
        } else if metrics.bandwidth_kbps >= self.estimated_bandwidth() {
            5
        } else {
            0
        };

        (loss_score + rtt_score + jitter_score + bandwidth_score).min(100)
    }

    /// Check if current quality is appropriate for network conditions
    pub fn is_quality_appropriate(&self) -> bool {
        let score = self.network_quality_score();

        match self.config.quality_preset {
            QualityPreset::Voice => score >= 30,
            QualityPreset::Low => score >= 50,
            QualityPreset::Medium => score >= 70,
            QualityPreset::High => score >= 85,
            QualityPreset::Maximum => score >= 95,
            QualityPreset::Custom => true, // User knows what they're doing
        }
    }

    /// Set quality alert callback for notifications
    pub fn set_alert_callback<F>(&mut self, callback: F)
    where
        F: Fn(QualityAlert, String) + Send + Sync + 'static,
    {
        self.alert_callback = Some(Box::new(callback));
    }

    /// Update quality monitoring configuration
    pub fn update_monitoring_config(&mut self, config: QualityMonitoringConfig) {
        info!(
            "Updating quality monitoring config for guild {}: interval={}s, auto_adjustment={}",
            self.guild_id,
            config.monitoring_interval.as_secs(),
            config.auto_adjustment_enabled
        );
        self.monitoring_config = config;
    }

    /// Update bitrate adjustment configuration
    pub fn update_adjustment_config(&mut self, config: BitrateAdjustmentConfig) {
        info!(
            "Updating bitrate adjustment config for guild {}: policy={:?}, gradual={}, hysteresis={}%",
            self.guild_id, config.adjustment_policy, config.gradual_transitions, config.hysteresis_margin
        );
        self.adjustment_config = config;
    }

    /// Trigger quality adjustment based on current conditions (async version)
    pub async fn trigger_quality_adjustment(&mut self) -> Result<()> {
        if self.config.adaptive_quality {
            self.adjust_quality_for_network().await
        } else {
            Ok(())
        }
    }

    /// Get current quality preset
    pub fn get_current_preset(&self) -> QualityPreset {
        self.config.quality_preset
    }

    /// Get current quality metrics
    pub async fn get_quality_metrics(&self) -> QualityMetrics {
        self.quality_metrics.read().await.clone()
    }

    /// Update real-time quality metrics
    pub async fn update_quality_metrics(
        &self,
        effective_bitrate: u32,
        buffer_health: u8,
        encoding_performance: u8,
        stream_stability: u8,
    ) -> Result<()> {
        let mut metrics = self.quality_metrics.write().await;

        // Update metrics
        metrics.effective_bitrate = effective_bitrate;
        metrics.buffer_health = buffer_health;
        metrics.encoding_performance = encoding_performance;
        metrics.stream_stability = stream_stability;
        metrics.last_update = Instant::now();

        // Calculate overall quality score
        let quality_score = self.calculate_overall_quality_score(&metrics).await;
        metrics.average_quality_score = quality_score;

        // Add to history for trend analysis BEFORE calculating trend
        self.add_quality_data_point(quality_score, effective_bitrate)
            .await;

        // Update quality trend (now includes current data point)
        metrics.quality_trend = self.calculate_quality_trend(quality_score).await;

        // Check for quality degradation
        if quality_score < self.monitoring_config.degradation_threshold {
            metrics.degradation_events += 1;
            self.handle_quality_degradation(quality_score).await;
        }

        debug!(
            "Updated quality metrics for guild {}: score={}, trend={:?}, bitrate={}kbps",
            self.guild_id, quality_score, metrics.quality_trend, effective_bitrate
        );

        Ok(())
    }

    /// Calculate overall quality score from individual metrics
    async fn calculate_overall_quality_score(&self, metrics: &QualityMetrics) -> u8 {
        let network_score = self.network_quality_score();

        // Weighted average of different quality factors
        let weights = [0.3, 0.25, 0.25, 0.2]; // network, buffer, encoding, stability
        let scores = [
            network_score as f32,
            metrics.buffer_health as f32,
            metrics.encoding_performance as f32,
            metrics.stream_stability as f32,
        ];

        let weighted_sum: f32 = weights.iter().zip(scores.iter()).map(|(w, s)| w * s).sum();
        weighted_sum.round() as u8
    }

    /// Calculate quality trend based on historical data
    async fn calculate_quality_trend(&self, current_score: u8) -> QualityTrend {
        let history = self.quality_history.read().await;

        if history.len() < 2 {
            return QualityTrend::Stable;
        }

        // Get recent scores for trend analysis (including current score)
        let mut recent_scores: Vec<u8> = history
            .iter()
            .rev()
            .take(10) // Take last 10 historical scores for better trend detection
            .map(|point| point.quality_score)
            .collect();

        // Add current score to the front (most recent)
        recent_scores.insert(0, current_score);

        if recent_scores.len() < 3 {
            return QualityTrend::Stable;
        }

        // Calculate trend using slope between oldest and newest scores
        let oldest_score = recent_scores[recent_scores.len() - 1] as f32;
        let newest_score = recent_scores[0] as f32;
        let score_change = newest_score - oldest_score;

        // Also check for consistent direction in recent changes
        let mut declining_count = 0;
        let mut improving_count = 0;

        for i in 1..recent_scores.len() {
            if recent_scores[i - 1] < recent_scores[i] {
                declining_count += 1;
            } else if recent_scores[i - 1] > recent_scores[i] {
                improving_count += 1;
            }
        }

        // Use more sensitive thresholds for gradual changes
        let trend_threshold = 3.0; // More sensitive threshold
        let consistency_threshold = recent_scores.len() / 3; // At least 1/3 of changes in same direction

        if score_change > trend_threshold || improving_count >= consistency_threshold {
            QualityTrend::Improving
        } else if score_change < -trend_threshold || declining_count >= consistency_threshold {
            QualityTrend::Degrading
        } else {
            QualityTrend::Stable
        }
    }

    /// Add quality data point to history
    async fn add_quality_data_point(&self, quality_score: u8, _bitrate: u32) {
        let mut history = self.quality_history.write().await;

        let data_point = QualityDataPoint { quality_score };

        history.push_back(data_point);

        // Maintain history window size
        while history.len() > self.monitoring_config.history_window_size {
            history.pop_front();
        }
    }

    /// Handle quality degradation events
    async fn handle_quality_degradation(&self, quality_score: u8) {
        let alert_level = if quality_score < self.monitoring_config.critical_threshold {
            QualityAlert::Critical
        } else {
            QualityAlert::Warning
        };

        let message = format!(
            "Quality degradation detected for guild {}: score={}, threshold={}",
            self.guild_id, quality_score, self.monitoring_config.degradation_threshold
        );

        match alert_level {
            QualityAlert::Critical => error!("{}", message),
            QualityAlert::Warning => warn!("{}", message),
        }

        // Trigger alert callback if set
        if let Some(ref callback) = self.alert_callback {
            callback(alert_level, message);
        }

        // Attempt automatic quality adjustment if enabled
        if self.monitoring_config.auto_adjustment_enabled {
            if let Err(e) = self.attempt_quality_recovery().await {
                error!(
                    "Failed to recover quality for guild {}: {}",
                    self.guild_id, e
                );
            }
        }
    }

    /// Attempt automatic quality recovery
    async fn attempt_quality_recovery(&self) -> Result<()> {
        let last_adjustment = self.last_adjustment.read().await;

        // Check cooldown period
        if let Some(last_time) = *last_adjustment {
            if last_time.elapsed() < self.monitoring_config.adjustment_cooldown {
                debug!("Quality adjustment on cooldown for guild {}", self.guild_id);
                return Ok(());
            }
        }
        drop(last_adjustment);

        // Update last adjustment time
        *self.last_adjustment.write().await = Some(Instant::now());

        info!("Attempting quality recovery for guild {}", self.guild_id);

        // This would typically trigger a quality downgrade
        // Implementation depends on integration with streaming manager
        Ok(())
    }

    /// Get quality monitoring statistics
    pub async fn get_monitoring_stats(&self) -> Result<serde_json::Value> {
        let metrics = self.quality_metrics.read().await;
        let history = self.quality_history.read().await;

        let stats = serde_json::json!({
            "guild_id": self.guild_id,
            "current_quality": {
                "score": metrics.average_quality_score,
                "trend": format!("{:?}", metrics.quality_trend),
                "effective_bitrate": metrics.effective_bitrate,
                "buffer_health": metrics.buffer_health,
                "encoding_performance": metrics.encoding_performance,
                "stream_stability": metrics.stream_stability,
                "degradation_events": metrics.degradation_events
            },
            "network_quality": {
                "score": self.network_quality_score(),
                "packet_loss": self.network_metrics.packet_loss,
                "rtt_ms": self.network_metrics.rtt_ms,
                "jitter_ms": self.network_metrics.jitter_ms,
                "bandwidth_kbps": self.network_metrics.bandwidth_kbps
            },
            "history": {
                "data_points": history.len(),
                "window_size": self.monitoring_config.history_window_size,
                "monitoring_interval_secs": self.monitoring_config.monitoring_interval.as_secs()
            },
            "configuration": {
                "preset": format!("{:?}", self.config.quality_preset),
                "target_bitrate": self.config.bitrate,
                "adaptive_quality": self.config.adaptive_quality,
                "auto_adjustment": self.monitoring_config.auto_adjustment_enabled
            }
        });

        Ok(stats)
    }

    /// Helper methods for enhanced quality adjustment system
    /// Check if hysteresis should prevent adjustment
    async fn should_apply_hysteresis(
        &self,
        state: &QualityAdjustmentState,
        current_score: u8,
    ) -> bool {
        if state.recent_adjustments.is_empty() {
            return false;
        }

        // Check if we recently made an opposite adjustment
        if let Some(last_adjustment) = state.recent_adjustments.back() {
            let time_since_last = last_adjustment.timestamp.elapsed();
            if time_since_last < Duration::from_secs(30) {
                let hysteresis_threshold = self.adjustment_config.hysteresis_margin;
                let score_diff = (current_score as f32 - 50.0).abs(); // Distance from neutral

                return score_diff < hysteresis_threshold;
            }
        }

        false
    }

    /// Evaluate adjustment in stable phase
    async fn evaluate_stable_phase_adjustment(
        &self,
        overall_score: u8,
        metrics: &NetworkMetrics,
        state: &QualityAdjustmentState,
    ) -> Option<(QualityPreset, AdjustmentReason)> {
        let current_preset = self.config.quality_preset;

        // Check if we should downgrade due to poor conditions
        if overall_score < self.monitoring_config.degradation_threshold && metrics.packet_loss > 5.0
        {
            return Some((
                self.get_downgrade_preset(current_preset),
                AdjustmentReason::NetworkDegradation,
            ));
        }

        // Check if we can upgrade (only if stable for required period)
        if let Some(stable_since) = state.stable_since {
            if stable_since.elapsed() >= self.adjustment_config.upgrade_stability_period
                && overall_score > 85
                && metrics.packet_loss < 1.0
                && metrics.bandwidth_kbps > self.config.bitrate * 3
            {
                return Some((
                    self.get_upgrade_preset(current_preset),
                    AdjustmentReason::NetworkImprovement,
                ));
            }
        }

        None
    }

    /// Evaluate adjustment in degrading phase
    async fn evaluate_degrading_phase_adjustment(
        &self,
        overall_score: u8,
        _metrics: &NetworkMetrics,
        _state: &QualityAdjustmentState,
    ) -> Option<(QualityPreset, AdjustmentReason)> {
        let current_preset = self.config.quality_preset;

        // Continue degrading if score is still poor
        if overall_score < self.monitoring_config.degradation_threshold {
            return Some((
                self.get_downgrade_preset(current_preset),
                AdjustmentReason::NetworkDegradation,
            ));
        }

        None
    }

    /// Evaluate adjustment in recovering phase
    async fn evaluate_recovering_phase_adjustment(
        &self,
        overall_score: u8,
        metrics: &NetworkMetrics,
        _state: &QualityAdjustmentState,
    ) -> Option<(QualityPreset, AdjustmentReason)> {
        let current_preset = self.config.quality_preset;

        // Continue upgrading if conditions are good
        if overall_score > 80
            && metrics.packet_loss < 2.0
            && metrics.bandwidth_kbps > self.config.bitrate * 2
        {
            return Some((
                self.get_upgrade_preset(current_preset),
                AdjustmentReason::NetworkImprovement,
            ));
        }

        // Fall back to degrading if conditions worsen
        if overall_score < self.monitoring_config.degradation_threshold {
            return Some((
                self.get_downgrade_preset(current_preset),
                AdjustmentReason::NetworkDegradation,
            ));
        }

        None
    }

    /// Evaluate adjustment in emergency phase
    async fn evaluate_emergency_phase_adjustment(
        &self,
        overall_score: u8,
        _metrics: &NetworkMetrics,
        _state: &QualityAdjustmentState,
    ) -> Option<(QualityPreset, AdjustmentReason)> {
        // In emergency, always try to get to voice quality
        if self.config.quality_preset != QualityPreset::Voice {
            return Some((QualityPreset::Voice, AdjustmentReason::Emergency));
        }

        // If already at voice quality and score improves, start recovering
        if overall_score > self.adjustment_config.emergency_threshold + 20 {
            return Some((QualityPreset::Low, AdjustmentReason::NetworkImprovement));
        }

        None
    }

    /// Update stable state tracking
    async fn update_stable_state(&self) {
        let mut state = self.adjustment_state.write().await;

        if state.stable_since.is_none() {
            state.stable_since = Some(Instant::now());
            state.current_phase = AdjustmentPhase::Stable;
            state.adjustment_streak = 0;
        }
    }

    /// Check if adjustment is in same direction as previous
    fn is_same_direction(
        &self,
        state: &QualityAdjustmentState,
        from: QualityPreset,
        to: QualityPreset,
    ) -> bool {
        if let Some(last_adjustment) = state.recent_adjustments.back() {
            let last_was_upgrade =
                self.is_upgrade(last_adjustment.from_preset, last_adjustment.to_preset);
            let current_is_upgrade = self.is_upgrade(from, to);
            return last_was_upgrade == current_is_upgrade;
        }
        false
    }

    /// Check if preset change is an upgrade
    fn is_upgrade(&self, from: QualityPreset, to: QualityPreset) -> bool {
        self.get_preset_rank(to) > self.get_preset_rank(from)
    }

    /// Check if preset change is a downgrade
    fn is_downgrade(&self, from: QualityPreset, to: QualityPreset) -> bool {
        self.get_preset_rank(to) < self.get_preset_rank(from)
    }

    /// Get numeric rank for preset comparison
    fn get_preset_rank(&self, preset: QualityPreset) -> u8 {
        match preset {
            QualityPreset::Voice => 1,
            QualityPreset::Low => 2,
            QualityPreset::Medium => 3,
            QualityPreset::High => 4,
            QualityPreset::Maximum => 5,
            QualityPreset::Custom => 3, // Treat as medium
        }
    }

    /// Get next lower quality preset
    fn get_downgrade_preset(&self, current: QualityPreset) -> QualityPreset {
        match current {
            QualityPreset::Maximum => QualityPreset::High,
            QualityPreset::High => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::Low,
            QualityPreset::Low => QualityPreset::Voice,
            QualityPreset::Voice => QualityPreset::Voice, // Can't go lower
            QualityPreset::Custom => QualityPreset::Medium, // Safe fallback
        }
    }

    /// Get next higher quality preset
    fn get_upgrade_preset(&self, current: QualityPreset) -> QualityPreset {
        match current {
            QualityPreset::Voice => QualityPreset::Low,
            QualityPreset::Low => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::High,
            QualityPreset::High => QualityPreset::Maximum,
            QualityPreset::Maximum => QualityPreset::Maximum, // Can't go higher
            QualityPreset::Custom => QualityPreset::High,     // Safe upgrade
        }
    }

    /// Generate a comprehensive quality report
    pub async fn generate_quality_report(&self) -> Result<serde_json::Value> {
        let metrics = self.quality_metrics.read().await;
        let history = self.quality_history.read().await;

        let report = serde_json::json!({
            "guild_id": self.guild_id,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "current_metrics": {
                "effective_bitrate": metrics.effective_bitrate,
                "buffer_health": metrics.buffer_health,
                "encoding_performance": metrics.encoding_performance,
                "stream_stability": metrics.stream_stability,
                "average_quality_score": metrics.average_quality_score,
                "quality_trend": format!("{:?}", metrics.quality_trend),
                "degradation_events": metrics.degradation_events
            },
            "network_quality": {
                "score": self.network_quality_score(),
                "packet_loss": self.network_metrics.packet_loss,
                "rtt_ms": self.network_metrics.rtt_ms,
                "jitter_ms": self.network_metrics.jitter_ms,
                "bandwidth_kbps": self.network_metrics.bandwidth_kbps
            },
            "configuration": {
                "quality_preset": format!("{:?}", self.config.quality_preset),
                "target_bitrate": self.config.bitrate,
                "adaptive_quality": self.config.adaptive_quality,
                "monitoring_enabled": self.monitoring_config.auto_adjustment_enabled
            },
            "history": {
                "data_points": history.len(),
                "window_size": self.monitoring_config.history_window_size
            }
        });

        Ok(report)
    }
}

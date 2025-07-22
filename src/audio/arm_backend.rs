// ARM-specific audio backend for IoT devices
// This module provides ALSA-based audio output optimized for ARM devices

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

#[cfg(feature = "alsa-backend")]
use alsa::{PCM, Direction, ValueOr};

/// ARM-optimized audio configuration
#[derive(Debug, Clone)]
pub struct ArmAudioConfig {
    /// Sample rate (typically 44100 or 48000)
    pub sample_rate: u32,
    /// Number of channels (1 for mono, 2 for stereo)
    pub channels: u16,
    /// Buffer size in frames
    pub buffer_size: u32,
    /// Period size in frames
    pub period_size: u32,
    /// Use NEON SIMD optimizations if available
    pub use_neon: bool,
    /// Low-latency mode for real-time audio
    pub low_latency: bool,
}

impl Default for ArmAudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            buffer_size: 1024,
            period_size: 256,
            use_neon: cfg!(target_feature = "neon"),
            low_latency: false,
        }
    }
}

/// ARM-specific audio backend
pub struct ArmAudioBackend {
    config: ArmAudioConfig,
    #[cfg(feature = "alsa-backend")]
    pcm: Option<Arc<RwLock<PCM>>>,
    is_initialized: bool,
}

impl ArmAudioBackend {
    /// Create a new ARM audio backend
    pub fn new(config: ArmAudioConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "alsa-backend")]
            pcm: None,
            is_initialized: false,
        }
    }

    /// Initialize the audio backend
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing ARM audio backend");
        debug!("Audio config: {:?}", self.config);

        #[cfg(feature = "alsa-backend")]
        {
            self.init_alsa().await?;
        }

        #[cfg(not(feature = "alsa-backend"))]
        {
            warn!("ALSA backend not enabled, using dummy backend");
            self.init_dummy().await?;
        }

        self.is_initialized = true;
        info!("ARM audio backend initialized successfully");
        Ok(())
    }

    #[cfg(feature = "alsa-backend")]
    async fn init_alsa(&mut self) -> Result<()> {
        use alsa::{PCM, Direction, ValueOr};
        use alsa::pcm::{HwParams, Format, Access, State};

        // Open PCM device
        let pcm = PCM::new("default", Direction::Playback, false)?;
        
        // Set hardware parameters
        {
            let hwp = HwParams::any(&pcm)?;
            hwp.set_channels(self.config.channels as u32)?;
            hwp.set_rate(self.config.sample_rate, ValueOr::Nearest)?;
            hwp.set_format(Format::s16())?;
            hwp.set_access(Access::RWInterleaved)?;
            
            // Set buffer and period sizes
            hwp.set_buffer_size_near(self.config.buffer_size as i64)?;
            hwp.set_period_size_near(self.config.period_size as i64, ValueOr::Nearest)?;
            
            pcm.hw_params(&hwp)?;
        }

        // Set software parameters
        {
            let swp = pcm.sw_params_current()?;
            swp.set_start_threshold(self.config.period_size as i64)?;
            pcm.sw_params(&swp)?;
        }

        // Prepare the PCM for use
        pcm.prepare()?;

        self.pcm = Some(Arc::new(RwLock::new(pcm)));
        info!("ALSA PCM device initialized");
        Ok(())
    }

    #[cfg(not(feature = "alsa-backend"))]
    async fn init_dummy(&mut self) -> Result<()> {
        info!("Dummy audio backend initialized (no actual audio output)");
        Ok(())
    }

    /// Play audio data
    pub async fn play(&self, data: &[i16]) -> Result<()> {
        if !self.is_initialized {
            return Err(anyhow::anyhow!("Audio backend not initialized"));
        }

        #[cfg(feature = "alsa-backend")]
        {
            self.play_alsa(data).await
        }

        #[cfg(not(feature = "alsa-backend"))]
        {
            self.play_dummy(data).await
        }
    }

    #[cfg(feature = "alsa-backend")]
    async fn play_alsa(&self, data: &[i16]) -> Result<()> {
        if let Some(pcm_arc) = &self.pcm {
            let pcm = pcm_arc.read().await;
            
            // Convert i16 samples to the format expected by ALSA
            let frames = data.len() / self.config.channels as usize;
            
            match pcm.io_i16()?.writei(data) {
                Ok(written) => {
                    if written != frames {
                        warn!("Partial write: {} frames written out of {}", written, frames);
                    }
                }
                Err(e) => {
                    error!("ALSA write error: {}", e);
                    return Err(anyhow::anyhow!("ALSA write failed: {}", e));
                }
            }
        }
        Ok(())
    }

    #[cfg(not(feature = "alsa-backend"))]
    async fn play_dummy(&self, data: &[i16]) -> Result<()> {
        // Simulate audio playback timing
        let duration_ms = (data.len() as f64 / self.config.channels as f64 / self.config.sample_rate as f64) * 1000.0;
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms as u64)).await;
        debug!("Dummy audio playback: {} samples", data.len());
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &ArmAudioConfig {
        &self.config
    }

    /// Check if the backend is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Get optimal buffer size for ARM device
    pub fn optimal_buffer_size(&self) -> u32 {
        // ARM devices typically benefit from smaller buffer sizes
        // to reduce latency, but not too small to avoid underruns
        match self.config.low_latency {
            true => 256,   // Low latency mode
            false => 1024, // Standard mode
        }
    }

    /// Apply ARM-specific audio optimizations
    pub fn apply_arm_optimizations(&mut self) {
        // Enable NEON SIMD if available
        if cfg!(target_feature = "neon") {
            self.config.use_neon = true;
            info!("NEON SIMD optimizations enabled");
        }

        // Adjust buffer sizes for ARM performance characteristics
        if self.config.buffer_size > 2048 {
            self.config.buffer_size = 2048;
            warn!("Buffer size reduced to 2048 for ARM optimization");
        }

        // Set period size to 1/4 of buffer size for good performance
        self.config.period_size = self.config.buffer_size / 4;
    }
}

/// ARM-specific audio processing utilities
pub mod arm_utils {
    use super::*;

    /// Process audio samples with ARM optimizations
    #[cfg(target_feature = "neon")]
    pub fn process_samples_neon(input: &[f32], output: &mut [f32], gain: f32) {
        // NEON SIMD implementation would go here
        // For now, fall back to scalar implementation
        process_samples_scalar(input, output, gain);
    }

    /// Scalar audio processing fallback
    pub fn process_samples_scalar(input: &[f32], output: &mut [f32], gain: f32) {
        for (i, o) in input.iter().zip(output.iter_mut()) {
            *o = *i * gain;
        }
    }

    /// Convert f32 samples to i16 with ARM optimizations
    pub fn f32_to_i16_arm(input: &[f32], output: &mut [i16]) {
        const SCALE: f32 = 32767.0;
        
        for (i, o) in input.iter().zip(output.iter_mut()) {
            let scaled = *i * SCALE;
            *o = scaled.clamp(-32768.0, 32767.0) as i16;
        }
    }

    /// Check if NEON SIMD is available
    pub fn has_neon() -> bool {
        cfg!(target_feature = "neon")
    }

    /// Get ARM CPU information
    pub fn get_arm_cpu_info() -> String {
        #[cfg(target_arch = "arm")]
        {
            "ARM 32-bit".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        {
            "ARM 64-bit".to_string()
        }
        #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
        {
            "Non-ARM".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_arm_audio_backend_creation() {
        let config = ArmAudioConfig::default();
        let backend = ArmAudioBackend::new(config);
        assert!(!backend.is_initialized());
    }

    #[test]
    fn test_arm_utils() {
        use arm_utils::*;
        
        let input = vec![0.5, -0.5, 1.0, -1.0];
        let mut output = vec![0i16; 4];
        
        f32_to_i16_arm(&input, &mut output);
        
        assert_eq!(output[0], 16383);  // 0.5 * 32767
        assert_eq!(output[1], -16384); // -0.5 * 32767
        assert_eq!(output[2], 32767);  // 1.0 * 32767 (clamped)
        assert_eq!(output[3], -32768); // -1.0 * 32767 (clamped)
    }

    #[test]
    fn test_optimal_buffer_size() {
        let mut config = ArmAudioConfig::default();
        config.low_latency = true;
        let backend = ArmAudioBackend::new(config);
        assert_eq!(backend.optimal_buffer_size(), 256);

        let mut config = ArmAudioConfig::default();
        config.low_latency = false;
        let backend = ArmAudioBackend::new(config);
        assert_eq!(backend.optimal_buffer_size(), 1024);
    }
}

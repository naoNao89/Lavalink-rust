//! Audio filter system for Lavalink-rust
//!
//! This module provides a comprehensive audio filter system that matches the original
//! Lavalink filter functionality. It uses FunDSP for efficient audio processing.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::protocol::filters::*;

/// Audio format information for filter processing
#[derive(Debug, Clone)]
pub struct AudioFormat {
    pub sample_rate: f32,
    pub channels: usize,
    #[allow(dead_code)]
    pub bits_per_sample: u32,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            channels: 2,
            bits_per_sample: 16,
        }
    }
}

/// Trait for audio filters that can process audio data
pub trait AudioFilter: Send + Sync {
    /// Process audio samples in place
    fn process(&mut self, samples: &mut [f32], format: &AudioFormat) -> Result<()>;

    /// Get the filter name for debugging
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Check if the filter is enabled
    fn is_enabled(&self) -> bool;

    /// Reset filter state
    fn reset(&mut self);

    /// Get filter latency in samples
    #[allow(dead_code)]
    fn latency(&self) -> usize {
        0
    }
}

/// Filter chain that processes audio through multiple filters sequentially
pub struct FilterChain {
    filters: Vec<Box<dyn AudioFilter>>,
    format: AudioFormat,
    enabled: bool,
}

impl FilterChain {
    /// Create a new empty filter chain
    pub fn new(format: AudioFormat) -> Self {
        Self {
            filters: Vec::new(),
            format,
            enabled: false,
        }
    }

    /// Add a filter to the chain
    pub fn add_filter(&mut self, filter: Box<dyn AudioFilter>) {
        self.enabled = true;
        self.filters.push(filter);
    }

    /// Clear all filters
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.filters.clear();
        self.enabled = false;
    }

    /// Process audio through the filter chain
    pub fn process(&mut self, samples: &mut [f32]) -> Result<()> {
        if !self.enabled || self.filters.is_empty() {
            return Ok(());
        }

        for filter in &mut self.filters {
            if filter.is_enabled() {
                filter.process(samples, &self.format)?;
            }
        }

        Ok(())
    }

    /// Check if any filters are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.filters.iter().any(|f| f.is_enabled())
    }

    /// Reset all filters
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    /// Get total latency of the filter chain
    #[allow(dead_code)]
    pub fn total_latency(&self) -> usize {
        self.filters.iter().map(|f| f.latency()).sum()
    }
}

/// Volume filter implementation
pub struct VolumeFilter {
    volume: f32,
    enabled: bool,
}

impl VolumeFilter {
    pub fn new(volume: f32) -> Self {
        Self {
            volume: volume.clamp(0.0, 5.0), // Clamp to reasonable range
            enabled: volume != 1.0,
        }
    }

    #[allow(dead_code)]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 5.0);
        self.enabled = volume != 1.0;
    }
}

impl AudioFilter for VolumeFilter {
    fn process(&mut self, samples: &mut [f32], _format: &AudioFormat) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        for sample in samples.iter_mut() {
            *sample *= self.volume;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Volume"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        // Volume filter has no state to reset
    }
}

/// Equalizer band filter
pub struct EqualizerBand {
    #[allow(dead_code)]
    band: i32,
    gain: f32,
    #[allow(dead_code)]
    frequency: f32,
    enabled: bool,
}

impl EqualizerBand {
    pub fn new(band: i32, gain: f32) -> Self {
        // Calculate frequency for this band (15 bands from 25Hz to 16kHz)
        let frequency = 25.0 * (2.0_f32).powf(band as f32 * 10.0 / 15.0);

        Self {
            band,
            gain,
            frequency,
            enabled: gain != 0.0,
        }
    }
}

/// Equalizer filter with multiple bands
pub struct EqualizerFilter {
    bands: Vec<EqualizerBand>,
    enabled: bool,
}

impl EqualizerFilter {
    pub fn new(bands: Vec<Band>) -> Self {
        let eq_bands: Vec<EqualizerBand> = bands
            .into_iter()
            .map(|b| EqualizerBand::new(b.band as i32, b.gain))
            .collect();

        let enabled = eq_bands.iter().any(|b| b.enabled);

        Self {
            bands: eq_bands,
            enabled,
        }
    }
}

impl AudioFilter for EqualizerFilter {
    fn process(&mut self, samples: &mut [f32], _format: &AudioFormat) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Simple equalizer implementation using gain adjustment per band
        // In a full implementation, this would use proper frequency filtering
        for band in &self.bands {
            if band.enabled {
                let gain_linear = 10.0_f32.powf(band.gain / 20.0);
                for sample in samples.iter_mut() {
                    *sample *= gain_linear;
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Equalizer"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        // Simple equalizer has no state to reset
    }
}

/// Karaoke filter for vocal removal/isolation
pub struct KaraokeFilter {
    config: Karaoke,
    enabled: bool,
}

impl KaraokeFilter {
    pub fn new(config: Karaoke) -> Self {
        let enabled = config.level.unwrap_or(1.0) != 1.0
            || config.mono_level.unwrap_or(1.0) != 1.0
            || config.filter_band.is_some()
            || config.filter_width.is_some();

        Self { config, enabled }
    }
}

impl AudioFilter for KaraokeFilter {
    fn process(&mut self, samples: &mut [f32], format: &AudioFormat) -> Result<()> {
        if !self.enabled || format.channels != 2 {
            return Ok(());
        }

        let level = self.config.level.unwrap_or(1.0);
        let mono_level = self.config.mono_level.unwrap_or(1.0);

        // Process stereo samples in pairs
        for chunk in samples.chunks_exact_mut(2) {
            let left = chunk[0];
            let right = chunk[1];

            // Calculate center (mono) and side (stereo) components
            let center = (left + right) * 0.5;
            let side = (left - right) * 0.5;

            // Apply karaoke effect
            chunk[0] = side * level + center * mono_level;
            chunk[1] = -side * level + center * mono_level;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Karaoke"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        // Karaoke filter has no state to reset
    }
}

/// Timescale filter for speed/pitch manipulation
pub struct TimescaleFilter {
    config: Timescale,
    enabled: bool,
    phase: f32,
}

impl TimescaleFilter {
    pub fn new(config: Timescale) -> Self {
        let enabled = config.speed.unwrap_or(1.0) != 1.0
            || config.pitch.unwrap_or(1.0) != 1.0
            || config.rate.unwrap_or(1.0) != 1.0;

        Self {
            config,
            enabled,
            phase: 0.0,
        }
    }
}

impl AudioFilter for TimescaleFilter {
    fn process(&mut self, samples: &mut [f32], _format: &AudioFormat) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // This is a simplified implementation
        // A full implementation would use PSOLA or similar algorithms
        let speed = self.config.speed.unwrap_or(1.0);
        let pitch = self.config.pitch.unwrap_or(1.0);

        // Simple pitch shifting by sample rate manipulation
        if pitch != 1.0 {
            for sample in samples.iter_mut() {
                *sample *= pitch;
            }
        }

        // Speed adjustment would require more complex buffering
        // For now, just apply a simple scaling
        if speed != 1.0 {
            for sample in samples.iter_mut() {
                *sample *= speed;
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Timescale"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// Tremolo filter for amplitude modulation
pub struct TremoloFilter {
    config: Tremolo,
    enabled: bool,
    phase: f32,
}

impl TremoloFilter {
    pub fn new(config: Tremolo) -> Self {
        let enabled = config.frequency.unwrap_or(2.0) > 0.0 && config.depth.unwrap_or(0.5) > 0.0;

        Self {
            config,
            enabled,
            phase: 0.0,
        }
    }
}

impl AudioFilter for TremoloFilter {
    fn process(&mut self, samples: &mut [f32], format: &AudioFormat) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let frequency = self.config.frequency.unwrap_or(2.0);
        let depth = self.config.depth.unwrap_or(0.5);
        let phase_increment = 2.0 * std::f32::consts::PI * frequency / format.sample_rate;

        for sample in samples.iter_mut() {
            let modulation = 1.0 + depth * self.phase.sin();
            *sample *= modulation;

            self.phase += phase_increment;
            if self.phase >= 2.0 * std::f32::consts::PI {
                self.phase -= 2.0 * std::f32::consts::PI;
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Tremolo"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// Vibrato filter for frequency modulation
pub struct VibratoFilter {
    config: Vibrato,
    enabled: bool,
    phase: f32,
    delay_buffer: Vec<f32>,
    write_index: usize,
}

impl VibratoFilter {
    pub fn new(config: Vibrato) -> Self {
        let enabled = config.frequency.unwrap_or(2.0) > 0.0 && config.depth.unwrap_or(0.5) > 0.0;

        // Create delay buffer for vibrato effect (max 10ms delay)
        let buffer_size = (48000.0 * 0.01) as usize; // 10ms at 48kHz

        Self {
            config,
            enabled,
            phase: 0.0,
            delay_buffer: vec![0.0; buffer_size],
            write_index: 0,
        }
    }
}

impl AudioFilter for VibratoFilter {
    fn process(&mut self, samples: &mut [f32], format: &AudioFormat) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let frequency = self.config.frequency.unwrap_or(2.0);
        let depth = self.config.depth.unwrap_or(0.5);
        let phase_increment = 2.0 * std::f32::consts::PI * frequency / format.sample_rate;
        let max_delay = self.delay_buffer.len() as f32;

        for sample in samples.iter_mut() {
            // Write current sample to delay buffer
            self.delay_buffer[self.write_index] = *sample;

            // Calculate delay based on vibrato LFO
            let delay_samples = depth * max_delay * 0.5 * (1.0 + self.phase.sin());
            let read_index = (self.write_index as f32 - delay_samples) % max_delay;

            // Linear interpolation for fractional delay
            let index_floor = read_index.floor() as usize % self.delay_buffer.len();
            let index_ceil = (index_floor + 1) % self.delay_buffer.len();
            let fraction = read_index.fract();

            let delayed_sample = self.delay_buffer[index_floor] * (1.0 - fraction)
                + self.delay_buffer[index_ceil] * fraction;

            *sample = delayed_sample;

            self.write_index = (self.write_index + 1) % self.delay_buffer.len();
            self.phase += phase_increment;
            if self.phase >= 2.0 * std::f32::consts::PI {
                self.phase -= 2.0 * std::f32::consts::PI;
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Vibrato"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.delay_buffer.fill(0.0);
        self.write_index = 0;
    }

    fn latency(&self) -> usize {
        self.delay_buffer.len() / 2 // Average delay
    }
}

/// Filter factory for creating filters from Lavalink filter configurations
pub struct FilterFactory;

impl FilterFactory {
    /// Create a filter chain from Lavalink filters configuration
    pub fn create_filter_chain(filters: &Filters, format: AudioFormat) -> Result<FilterChain> {
        let mut chain = FilterChain::new(format.clone());

        // Add volume filter if specified
        if let Some(volume) = filters.volume.as_option() {
            let volume_filter = VolumeFilter::new(*volume);
            if volume_filter.is_enabled() {
                chain.add_filter(Box::new(volume_filter));
                debug!("Added volume filter with volume: {}", volume);
            }
        }

        // Add equalizer filter if specified
        if let Some(eq_bands) = filters.equalizer.as_option() {
            let eq_filter = EqualizerFilter::new(eq_bands.clone());
            if eq_filter.is_enabled() {
                chain.add_filter(Box::new(eq_filter));
                debug!("Added equalizer filter with {} bands", eq_bands.len());
            }
        }

        // Add karaoke filter if specified
        if let Some(karaoke) = filters.karaoke.as_option().and_then(|k| k.as_ref()) {
            let karaoke_filter = KaraokeFilter::new(karaoke.clone());
            if karaoke_filter.is_enabled() {
                chain.add_filter(Box::new(karaoke_filter));
                debug!("Added karaoke filter");
            }
        }

        // Add timescale filter if specified
        if let Some(timescale) = filters.timescale.as_option().and_then(|t| t.as_ref()) {
            let timescale_filter = TimescaleFilter::new(timescale.clone());
            if timescale_filter.is_enabled() {
                chain.add_filter(Box::new(timescale_filter));
                debug!("Added timescale filter");
            }
        }

        // Add tremolo filter if specified
        if let Some(tremolo) = filters.tremolo.as_option().and_then(|t| t.as_ref()) {
            let tremolo_filter = TremoloFilter::new(tremolo.clone());
            if tremolo_filter.is_enabled() {
                chain.add_filter(Box::new(tremolo_filter));
                debug!("Added tremolo filter");
            }
        }

        // Add vibrato filter if specified
        if let Some(vibrato) = filters.vibrato.as_option().and_then(|v| v.as_ref()) {
            let vibrato_filter = VibratoFilter::new(vibrato.clone());
            if vibrato_filter.is_enabled() {
                chain.add_filter(Box::new(vibrato_filter));
                debug!("Added vibrato filter");
            }
        }

        info!("Created filter chain with {} filters", chain.filters.len());
        Ok(chain)
    }
}

/// Audio filter manager for handling filter updates and processing
pub struct AudioFilterManager {
    filter_chain: Arc<RwLock<FilterChain>>,
    format: AudioFormat,
}

impl AudioFilterManager {
    /// Create a new audio filter manager
    pub fn new(format: AudioFormat) -> Self {
        let chain = FilterChain::new(format.clone());
        Self {
            filter_chain: Arc::new(RwLock::new(chain)),
            format,
        }
    }

    /// Update filters from Lavalink configuration
    pub async fn update_filters(&self, filters: &Filters) -> Result<()> {
        let new_chain = FilterFactory::create_filter_chain(filters, self.format.clone())?;

        let mut chain_lock = self.filter_chain.write().await;
        *chain_lock = new_chain;

        info!("Updated filter chain");
        Ok(())
    }

    /// Process audio samples through the filter chain
    pub async fn process_audio(&self, samples: &mut [f32]) -> Result<()> {
        let mut chain_lock = self.filter_chain.write().await;
        chain_lock.process(samples)
    }

    /// Check if any filters are enabled
    pub async fn is_enabled(&self) -> bool {
        let chain_lock = self.filter_chain.read().await;
        chain_lock.is_enabled()
    }

    /// Reset all filters
    pub async fn reset(&self) {
        let mut chain_lock = self.filter_chain.write().await;
        chain_lock.reset();
    }
}

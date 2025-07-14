use super::Omissible;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Audio filters that can be applied to a player
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Filters {
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub volume: Omissible<f32>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub equalizer: Omissible<Vec<Band>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub karaoke: Omissible<Option<Karaoke>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub timescale: Omissible<Option<Timescale>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub tremolo: Omissible<Option<Tremolo>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub vibrato: Omissible<Option<Vibrato>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub distortion: Omissible<Option<Distortion>>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub rotation: Omissible<Option<Rotation>>,
    #[serde(
        rename = "channelMix",
        skip_serializing_if = "Omissible::is_omitted",
        default
    )]
    pub channel_mix: Omissible<Option<ChannelMix>>,
    #[serde(
        rename = "lowPass",
        skip_serializing_if = "Omissible::is_omitted",
        default
    )]
    pub low_pass: Omissible<Option<LowPass>>,
    #[serde(rename = "pluginFilters", flatten)]
    #[cfg(feature = "plugins")]
    pub plugin_filters: HashMap<String, serde_json::Value>,
}

/// Equalizer band configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub band: u8,
    pub gain: f32,
}

/// Karaoke filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Karaoke {
    pub level: Option<f32>,
    #[serde(rename = "monoLevel")]
    pub mono_level: Option<f32>,
    #[serde(rename = "filterBand")]
    pub filter_band: Option<f32>,
    #[serde(rename = "filterWidth")]
    pub filter_width: Option<f32>,
}

/// Timescale filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timescale {
    pub speed: Option<f32>,
    pub pitch: Option<f32>,
    pub rate: Option<f32>,
}

/// Tremolo filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tremolo {
    pub frequency: Option<f32>,
    pub depth: Option<f32>,
}

/// Vibrato filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vibrato {
    pub frequency: Option<f32>,
    pub depth: Option<f32>,
}

/// Distortion filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distortion {
    #[serde(rename = "sinOffset")]
    pub sin_offset: Option<f32>,
    #[serde(rename = "sinScale")]
    pub sin_scale: Option<f32>,
    #[serde(rename = "cosOffset")]
    pub cos_offset: Option<f32>,
    #[serde(rename = "cosScale")]
    pub cos_scale: Option<f32>,
    #[serde(rename = "tanOffset")]
    pub tan_offset: Option<f32>,
    #[serde(rename = "tanScale")]
    pub tan_scale: Option<f32>,
    pub offset: Option<f32>,
    pub scale: Option<f32>,
}

/// Rotation filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rotation {
    #[serde(rename = "rotationHz")]
    pub rotation_hz: Option<f32>,
}

/// Channel mix filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMix {
    #[serde(rename = "leftToLeft")]
    pub left_to_left: Option<f32>,
    #[serde(rename = "leftToRight")]
    pub left_to_right: Option<f32>,
    #[serde(rename = "rightToLeft")]
    pub right_to_left: Option<f32>,
    #[serde(rename = "rightToRight")]
    pub right_to_right: Option<f32>,
}

/// Low pass filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowPass {
    pub smoothing: Option<f32>,
}

impl Filters {
    /// Create a new empty filters configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any filters are enabled
    pub fn is_enabled(&self) -> bool {
        self.volume.is_present()
            || self.equalizer.is_present()
            || self.karaoke.is_present()
            || self.timescale.is_present()
            || self.tremolo.is_present()
            || self.vibrato.is_present()
            || self.distortion.is_present()
            || self.rotation.is_present()
            || self.channel_mix.is_present()
            || self.low_pass.is_present()
            || {
                #[cfg(feature = "plugins")]
                {
                    !self.plugin_filters.is_empty()
                }
                #[cfg(not(feature = "plugins"))]
                {
                    false
                }
            }
    }

    /// Create a bass boost preset
    pub fn bass_boost() -> Self {
        let mut filters = Self::new();
        filters.equalizer = crate::protocol::Omissible::Present(vec![
            Band { band: 0, gain: 0.2 }, // 25 Hz
            Band {
                band: 1,
                gain: 0.15,
            }, // 40 Hz
            Band { band: 2, gain: 0.1 }, // 63 Hz
            Band {
                band: 3,
                gain: 0.05,
            }, // 100 Hz
        ]);
        filters
    }

    /// Create a nightcore preset (higher pitch and speed)
    pub fn nightcore() -> Self {
        let mut filters = Self::new();
        filters.timescale = crate::protocol::Omissible::Present(Some(Timescale {
            speed: Some(1.2),
            pitch: Some(1.2),
            rate: Some(1.0),
        }));
        filters
    }

    /// Create a vaporwave preset (slower speed, lower pitch)
    pub fn vaporwave() -> Self {
        let mut filters = Self::new();
        filters.timescale = crate::protocol::Omissible::Present(Some(Timescale {
            speed: Some(0.8),
            pitch: Some(0.8),
            rate: Some(1.0),
        }));
        filters
    }

    /// Create a karaoke preset
    pub fn karaoke() -> Self {
        let mut filters = Self::new();
        filters.karaoke = crate::protocol::Omissible::Present(Some(Karaoke::default()));
        filters
    }

    /// Create a soft distortion preset
    pub fn soft_distortion() -> Self {
        let mut filters = Self::new();
        filters.distortion = crate::protocol::Omissible::Present(Some(Distortion {
            sin_offset: Some(0.0),
            sin_scale: Some(1.0),
            cos_offset: Some(0.0),
            cos_scale: Some(1.0),
            tan_offset: Some(0.0),
            tan_scale: Some(1.0),
            offset: Some(0.0),
            scale: Some(1.2),
        }));
        filters
    }

    /// Create a tremolo preset
    pub fn tremolo() -> Self {
        let mut filters = Self::new();
        filters.tremolo = crate::protocol::Omissible::Present(Some(Tremolo {
            frequency: Some(2.0),
            depth: Some(0.5),
        }));
        filters
    }

    /// Create a vibrato preset
    pub fn vibrato() -> Self {
        let mut filters = Self::new();
        filters.vibrato = crate::protocol::Omissible::Present(Some(Vibrato {
            frequency: Some(2.0),
            depth: Some(0.5),
        }));
        filters
    }

    /// Validate filters against disabled filter list
    pub fn validate(&self, disabled_filters: &[String]) -> Vec<String> {
        let mut errors = Vec::new();

        if disabled_filters.contains(&"volume".to_string()) && self.volume.is_present() {
            errors.push("Volume filter is disabled".to_string());
        }

        if disabled_filters.contains(&"equalizer".to_string()) && self.equalizer.is_present() {
            errors.push("Equalizer filter is disabled".to_string());
        }

        if disabled_filters.contains(&"karaoke".to_string()) && self.karaoke.is_present() {
            errors.push("Karaoke filter is disabled".to_string());
        }

        if disabled_filters.contains(&"timescale".to_string()) && self.timescale.is_present() {
            errors.push("Timescale filter is disabled".to_string());
        }

        if disabled_filters.contains(&"tremolo".to_string()) && self.tremolo.is_present() {
            errors.push("Tremolo filter is disabled".to_string());
        }

        if disabled_filters.contains(&"vibrato".to_string()) && self.vibrato.is_present() {
            errors.push("Vibrato filter is disabled".to_string());
        }

        if disabled_filters.contains(&"distortion".to_string()) && self.distortion.is_present() {
            errors.push("Distortion filter is disabled".to_string());
        }

        if disabled_filters.contains(&"rotation".to_string()) && self.rotation.is_present() {
            errors.push("Rotation filter is disabled".to_string());
        }

        if disabled_filters.contains(&"channelMix".to_string()) && self.channel_mix.is_present() {
            errors.push("Channel mix filter is disabled".to_string());
        }

        if disabled_filters.contains(&"lowPass".to_string()) && self.low_pass.is_present() {
            errors.push("Low pass filter is disabled".to_string());
        }

        // Check plugin filters
        #[cfg(feature = "plugins")]
        {
            for filter_name in self.plugin_filters.keys() {
                if disabled_filters.contains(filter_name) {
                    errors.push(format!("Plugin filter '{filter_name}' is disabled"));
                }
            }
        }

        errors
    }
}

impl Default for Karaoke {
    fn default() -> Self {
        Self {
            level: Some(1.0),
            mono_level: Some(1.0),
            filter_band: Some(220.0),
            filter_width: Some(100.0),
        }
    }
}

impl Default for Timescale {
    fn default() -> Self {
        Self {
            speed: Some(1.0),
            pitch: Some(1.0),
            rate: Some(1.0),
        }
    }
}

impl Default for Tremolo {
    fn default() -> Self {
        Self {
            frequency: Some(2.0),
            depth: Some(0.5),
        }
    }
}

impl Default for Vibrato {
    fn default() -> Self {
        Self {
            frequency: Some(2.0),
            depth: Some(0.5),
        }
    }
}

impl Default for Distortion {
    fn default() -> Self {
        Self {
            sin_offset: Some(0.0),
            sin_scale: Some(1.0),
            cos_offset: Some(0.0),
            cos_scale: Some(1.0),
            tan_offset: Some(0.0),
            tan_scale: Some(1.0),
            offset: Some(0.0),
            scale: Some(1.0),
        }
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self {
            rotation_hz: Some(0.0),
        }
    }
}

impl Default for ChannelMix {
    fn default() -> Self {
        Self {
            left_to_left: Some(1.0),
            left_to_right: Some(0.0),
            right_to_left: Some(0.0),
            right_to_right: Some(1.0),
        }
    }
}

impl Default for LowPass {
    fn default() -> Self {
        Self {
            smoothing: Some(20.0),
        }
    }
}

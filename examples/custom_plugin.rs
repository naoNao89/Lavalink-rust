// Example custom plugin implementation
// This demonstrates how to create a custom plugin for Lavalink-rust

use anyhow::Result;
use async_trait::async_trait;
use lavalink_rust::plugin::{LavalinkPlugin, PluginManager};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Custom audio enhancement plugin
pub struct AudioEnhancementPlugin {
    name: String,
    version: String,
    description: String,
    config: Arc<RwLock<AudioEnhancementConfig>>,
    statistics: Arc<RwLock<PluginStatistics>>,
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct AudioEnhancementConfig {
    pub enabled: bool,
    pub auto_normalize: bool,
    pub bass_boost: f32,
    pub treble_boost: f32,
    pub noise_reduction: bool,
    pub max_volume: f32,
}

impl Default for AudioEnhancementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_normalize: false,
            bass_boost: 0.0,
            treble_boost: 0.0,
            noise_reduction: false,
            max_volume: 1.0,
        }
    }
}

/// Plugin statistics
#[derive(Debug, Default, Clone)]
pub struct PluginStatistics {
    pub tracks_processed: u64,
    pub enhancements_applied: u64,
    pub errors_encountered: u64,
    pub average_processing_time_ms: f64,
}

impl AudioEnhancementPlugin {
    pub fn new() -> Self {
        Self {
            name: "audio-enhancement".to_string(),
            version: "1.2.0".to_string(),
            description:
                "Advanced audio enhancement plugin with normalization, EQ, and noise reduction"
                    .to_string(),
            config: Arc::new(RwLock::new(AudioEnhancementConfig::default())),
            statistics: Arc::new(RwLock::new(PluginStatistics::default())),
        }
    }

    pub async fn get_statistics(&self) -> PluginStatistics {
        self.statistics.read().await.clone()
    }

    async fn apply_enhancements(&self, track_id: &str) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(format!("Track {} - enhancements disabled", track_id));
        }

        let mut enhancements = Vec::new();

        if config.auto_normalize {
            enhancements.push("auto-normalize".to_string());
        }

        if config.bass_boost > 0.0 {
            enhancements.push(format!("bass-boost-{}", config.bass_boost));
        }

        if config.treble_boost > 0.0 {
            enhancements.push(format!("treble-boost-{}", config.treble_boost));
        }

        if config.noise_reduction {
            enhancements.push("noise-reduction".to_string());
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.tracks_processed += 1;
            if !enhancements.is_empty() {
                stats.enhancements_applied += 1;
            }
            // Simulate processing time
            stats.average_processing_time_ms = (stats.average_processing_time_ms + 15.5) / 2.0;
        }

        if enhancements.is_empty() {
            Ok(format!("Track {} - no enhancements applied", track_id))
        } else {
            Ok(format!(
                "Track {} enhanced with: {}",
                track_id,
                enhancements.join(", ")
            ))
        }
    }
}

#[async_trait]
impl LavalinkPlugin for AudioEnhancementPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn initialize(&mut self) -> Result<()> {
        println!("🎵 Initializing Audio Enhancement Plugin v{}", self.version);
        println!("   Features: Auto-normalize, Bass/Treble boost, Noise reduction");

        // Reset statistics on initialization
        {
            let mut stats = self.statistics.write().await;
            *stats = PluginStatistics::default();
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        let stats = self.statistics.read().await;
        println!("🔌 Shutting down Audio Enhancement Plugin");
        println!("   Final Statistics:");
        println!("   - Tracks processed: {}", stats.tracks_processed);
        println!("   - Enhancements applied: {}", stats.enhancements_applied);
        println!(
            "   - Average processing time: {:.2}ms",
            stats.average_processing_time_ms
        );
        Ok(())
    }

    async fn on_track_load(&self, identifier: &str) -> Result<Option<String>> {
        match self.apply_enhancements(identifier).await {
            Ok(result) => {
                println!("🎶 Enhanced track: {}", result);
                Ok(Some(result))
            }
            Err(e) => {
                // Update error statistics
                {
                    let mut stats = self.statistics.write().await;
                    stats.errors_encountered += 1;
                }
                println!("❌ Enhancement error for {}: {}", identifier, e);
                Err(e)
            }
        }
    }

    async fn on_player_event(&self, event: &str) -> Result<()> {
        println!(
            "🎧 Audio Enhancement Plugin received player event: {}",
            event
        );

        // React to specific events
        match event {
            "track_start" => {
                println!("   🎵 Track started - enhancements active");
            }
            "track_end" => {
                println!("   ⏹️ Track ended - resetting enhancement state");
            }
            "volume_change" => {
                let config = self.config.read().await;
                if config.max_volume < 1.0 {
                    println!("   🔊 Volume change detected - enforcing max volume limit");
                }
            }
            _ => {
                println!("   ℹ️ Other event: {}", event);
            }
        }

        Ok(())
    }

    fn get_config_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "title": "Audio Enhancement Configuration",
            "description": "Configuration for the audio enhancement plugin",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable or disable the audio enhancement plugin"
                },
                "auto_normalize": {
                    "type": "boolean",
                    "default": false,
                    "description": "Automatically normalize audio levels"
                },
                "bass_boost": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 10.0,
                    "default": 0.0,
                    "description": "Bass boost level (0.0 to 10.0)"
                },
                "treble_boost": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 10.0,
                    "default": 0.0,
                    "description": "Treble boost level (0.0 to 10.0)"
                },
                "noise_reduction": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enable noise reduction"
                },
                "max_volume": {
                    "type": "number",
                    "minimum": 0.1,
                    "maximum": 2.0,
                    "default": 1.0,
                    "description": "Maximum volume limit (0.1 to 2.0)"
                }
            },
            "required": ["enabled"]
        }))
    }

    async fn update_config(&mut self, config: Value) -> Result<()> {
        println!("🔧 Updating Audio Enhancement Plugin configuration");

        let mut current_config = self.config.write().await;

        if let Some(enabled) = config.get("enabled").and_then(|v| v.as_bool()) {
            current_config.enabled = enabled;
            println!("   - Enabled: {}", enabled);
        }

        if let Some(auto_normalize) = config.get("auto_normalize").and_then(|v| v.as_bool()) {
            current_config.auto_normalize = auto_normalize;
            println!("   - Auto normalize: {}", auto_normalize);
        }

        if let Some(bass_boost) = config.get("bass_boost").and_then(|v| v.as_f64()) {
            current_config.bass_boost = bass_boost as f32;
            println!("   - Bass boost: {}", bass_boost);
        }

        if let Some(treble_boost) = config.get("treble_boost").and_then(|v| v.as_f64()) {
            current_config.treble_boost = treble_boost as f32;
            println!("   - Treble boost: {}", treble_boost);
        }

        if let Some(noise_reduction) = config.get("noise_reduction").and_then(|v| v.as_bool()) {
            current_config.noise_reduction = noise_reduction;
            println!("   - Noise reduction: {}", noise_reduction);
        }

        if let Some(max_volume) = config.get("max_volume").and_then(|v| v.as_f64()) {
            current_config.max_volume = max_volume as f32;
            println!("   - Max volume: {}", max_volume);
        }

        println!("✅ Configuration updated successfully");
        Ok(())
    }
}

/// Example usage of the custom plugin
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Lavalink-rust Custom Plugin Example");
    println!("=====================================");

    // Create plugin manager
    let mut plugin_manager = PluginManager::new();

    // Create and register our custom plugin
    let audio_plugin = Box::new(AudioEnhancementPlugin::new());
    plugin_manager.register_plugin(audio_plugin).await?;

    println!("\n📋 Registered Plugins:");
    for name in plugin_manager.get_plugin_names() {
        if let Some(plugin) = plugin_manager.get_plugin(&name) {
            println!(
                "  - {} v{}: {}",
                plugin.name(),
                plugin.version(),
                plugin.description()
            );
        }
    }

    // Test plugin functionality
    if let Some(plugin) = plugin_manager.get_plugin("audio-enhancement") {
        println!("\n🧪 Testing Plugin Functionality:");

        // Test track loading
        let result = plugin.on_track_load("test-track-123").await?;
        if let Some(enhancement_result) = result {
            println!("Enhancement result: {}", enhancement_result);
        }

        // Test player events
        plugin.on_player_event("track_start").await?;
        plugin.on_player_event("volume_change").await?;
        plugin.on_player_event("track_end").await?;

        // Test configuration update
        let _new_config = serde_json::json!({
            "enabled": true,
            "auto_normalize": true,
            "bass_boost": 2.5,
            "noise_reduction": true,
            "max_volume": 0.8
        });

        // Note: We can't call update_config on an immutable reference
        // This would require a mutable plugin manager design
        println!("\n📝 Configuration schema available:");
        if let Some(schema) = plugin.get_config_schema() {
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    plugin_manager.unload_all_plugins().await;

    println!("✅ Example completed successfully!");
    Ok(())
}

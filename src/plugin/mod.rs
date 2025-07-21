// Plugin system module
// This will handle loading and managing plugins

use anyhow::Result;
use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;

use crate::config::PluginsConfig;

pub mod interface;
pub mod loader;

pub use interface::*;
pub use loader::*;

/// Enhanced plugin manager for handling Lavalink plugins
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn LavalinkPlugin + Send + Sync>>,
    pub dynamic_loader: DynamicPluginLoader,
}

/// Trait for Lavalink plugins
#[async_trait]
pub trait LavalinkPlugin {
    /// Get the plugin name
    fn name(&self) -> &str;

    /// Get the plugin version
    fn version(&self) -> &str;

    /// Get the plugin description
    fn description(&self) -> &str {
        "No description provided"
    }

    /// Initialize the plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handle track loading
    async fn on_track_load(&self, _identifier: &str) -> Result<Option<String>> {
        Ok(None)
    }

    /// Handle player events
    async fn on_player_event(&self, _event: &str) -> Result<()> {
        Ok(())
    }

    /// Get plugin configuration schema
    fn get_config_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        let config = PluginsConfig {
            plugins: None,
            plugins_dir: Some("./plugins".to_string()),
            default_plugin_repository: Some("https://maven.lavalink.dev/releases".to_string()),
            default_plugin_snapshot_repository: Some(
                "https://maven.lavalink.dev/snapshots".to_string(),
            ),
        };

        Self::with_config(config)
    }

    /// Create a new plugin manager with configuration
    pub fn with_config(config: PluginsConfig) -> Self {
        let dynamic_loader = DynamicPluginLoader::new(&config);

        Self {
            plugins: HashMap::new(),
            dynamic_loader,
        }
    }

    /// Register a plugin
    pub async fn register_plugin(&mut self, mut plugin: Box<dyn LavalinkPlugin + Send + Sync>) -> Result<()> {
        let name = plugin.name().to_string();

        // Check if plugin is already registered
        if self.plugins.contains_key(&name) {
            return Err(anyhow::anyhow!("Plugin '{}' is already registered", name));
        }

        // Initialize the plugin
        plugin.initialize().await?;

        // Store the plugin
        self.plugins.insert(name.clone(), plugin);
        tracing::info!("Registered plugin: {}", name);
        Ok(())
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&mut self, name: &str) -> Result<()> {
        if let Some(mut plugin) = self.plugins.remove(name) {
            // Shutdown the plugin
            if let Err(e) = plugin.shutdown().await {
                tracing::warn!("Error shutting down plugin '{}': {}", name, e);
            }
            tracing::info!("Unregistered plugin: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin '{}' not found", name))
        }
    }

    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&(dyn LavalinkPlugin + Send + Sync)> {
        self.plugins.get(name).map(|boxed| boxed.as_ref())
    }

    /// Get all plugin names
    pub fn get_plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Load all dynamic plugins from the plugins directory
    pub fn load_dynamic_plugins(&mut self) -> Result<Vec<LoadedPlugin>> {
        self.dynamic_loader.load_all_plugins()
    }

    /// Get dynamic plugin metadata
    pub fn get_dynamic_plugin_metadata(&self, name: &str) -> Option<&PluginMetadata> {
        self.dynamic_loader.get_plugin_metadata(name)
    }

    /// Get all loaded dynamic plugin names
    pub fn get_dynamic_plugin_names(&self) -> Vec<String> {
        self.dynamic_loader.get_loaded_plugin_names()
    }

    /// Check if a dynamic plugin is loaded
    pub fn is_dynamic_plugin_loaded(&self, name: &str) -> bool {
        self.dynamic_loader.is_plugin_loaded(name)
    }

    /// Unload all plugins
    pub async fn unload_all_plugins(&mut self) {
        // Shutdown all registered plugins
        let plugin_names: Vec<String> = self.plugins.keys().cloned().collect();
        for name in plugin_names {
            if let Err(e) = self.unregister_plugin(&name).await {
                tracing::error!("Failed to unregister plugin '{}': {}", name, e);
            }
        }

        // Unload dynamic plugins
        if let Err(e) = self.dynamic_loader.unload_all_plugins() {
            tracing::error!("Failed to unload dynamic plugins: {}", e);
        }

        tracing::info!("Unloaded all plugins");
    }
}

/// Example plugin implementation
pub struct ExamplePlugin {
    name: String,
    version: String,
    initialized: bool,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            name: "example-plugin".to_string(),
            version: "1.0.0".to_string(),
            initialized: false,
        }
    }

    pub fn with_name(name: String) -> Self {
        Self {
            name,
            version: "1.0.0".to_string(),
            initialized: false,
        }
    }
}

impl Default for ExamplePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LavalinkPlugin for ExamplePlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Example plugin for testing purposes"
    }

    async fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        tracing::info!("Initialized example plugin: {}", self.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Plugin is not initialized"));
        }
        self.initialized = false;
        tracing::info!("Shutdown example plugin: {}", self.name);
        Ok(())
    }

    async fn on_track_load(&self, identifier: &str) -> Result<Option<String>> {
        let result = format!("Processed by {} - {}", self.name, identifier);
        Ok(Some(result))
    }

    async fn on_player_event(&self, event: &str) -> Result<()> {
        tracing::debug!("Plugin {} received player event: {}", self.name, event);
        Ok(())
    }

    fn get_config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable or disable the plugin"
                },
                "debug": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enable debug logging"
                }
            }
        }))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

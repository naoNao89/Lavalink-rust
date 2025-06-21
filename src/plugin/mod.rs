// Plugin system module
// This will handle loading and managing plugins

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::config::PluginsConfig;

pub mod interface;
pub mod loader;

pub use interface::*;
pub use loader::*;

/// Enhanced plugin manager for handling Lavalink plugins
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn LavalinkPlugin + Send + Sync>>,
    dynamic_loader: DynamicPluginLoader,
}

/// Trait for Lavalink plugins
#[async_trait]
pub trait LavalinkPlugin {
    /// Get the plugin name
    fn name(&self) -> &str;

    /// Get the plugin version
    fn version(&self) -> &str;
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
}

/// Example plugin implementation
pub struct ExamplePlugin {
    name: String,
    version: String,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            name: "example-plugin".to_string(),
            version: "1.0.0".to_string(),
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
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

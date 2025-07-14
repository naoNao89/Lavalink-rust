// Dynamic plugin loader using libloading
// Handles loading and managing dynamic plugin libraries

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[cfg(feature = "plugins")]
use libloading::{Library, Symbol};

use super::interface::{PluginInterface, PluginInterfaceWrapper, PLUGIN_INTERFACE_SYMBOL};
use crate::config::PluginsConfig;

/// Dynamic plugin loader
pub struct DynamicPluginLoader {
    plugins_dir: PathBuf,
    #[cfg(feature = "plugins")]
    loaded_libraries: HashMap<String, Arc<Library>>,
    loaded_plugins: HashMap<String, PluginInterfaceWrapper>,
}

/// Plugin loading result
#[derive(Debug)]
pub struct LoadedPlugin {
    pub name: String,
    pub version: String,
}

impl DynamicPluginLoader {
    /// Create a new dynamic plugin loader
    pub fn new(config: &PluginsConfig) -> Self {
        let plugins_dir = config
            .plugins_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./plugins"));

        Self {
            plugins_dir,
            #[cfg(feature = "plugins")]
            loaded_libraries: HashMap::new(),
            loaded_plugins: HashMap::new(),
        }
    }

    /// Discover plugins in the plugins directory
    pub fn discover_plugins(&self) -> Result<Vec<PathBuf>> {
        let mut plugin_paths = Vec::new();

        if !self.plugins_dir.exists() {
            info!("Plugins directory does not exist: {:?}", self.plugins_dir);
            return Ok(plugin_paths);
        }

        if !self.plugins_dir.is_dir() {
            warn!("Plugins path is not a directory: {:?}", self.plugins_dir);
            return Ok(plugin_paths);
        }

        let entries = std::fs::read_dir(&self.plugins_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && self.is_plugin_library(&path) {
                debug!("Discovered plugin library: {:?}", path);
                plugin_paths.push(path);
            }
        }

        info!("Discovered {} plugin libraries", plugin_paths.len());
        Ok(plugin_paths)
    }

    /// Check if a file is a plugin library based on extension
    fn is_plugin_library(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            match std::env::consts::OS {
                "windows" => ext == "dll",
                "macos" => ext == "dylib",
                _ => ext == "so", // Linux and other Unix-like systems
            }
        } else {
            false
        }
    }

    /// Load a plugin from a library file
    #[cfg(feature = "plugins")]
    pub fn load_plugin(&mut self, library_path: &Path) -> Result<LoadedPlugin> {
        info!("Loading plugin from: {:?}", library_path);

        // Load the library
        let library = unsafe { Library::new(library_path)? };

        // Get the plugin interface symbol
        let interface_fn: Symbol<fn() -> PluginInterface> =
            unsafe { library.get(PLUGIN_INTERFACE_SYMBOL)? };

        // Get the plugin interface
        let interface = interface_fn();

        // Create wrapper for safe interaction
        let wrapper = PluginInterfaceWrapper::new(interface)?;
        let metadata = wrapper.metadata();

        // Check if plugin is already loaded
        if self.loaded_plugins.contains_key(&metadata.name) {
            return Err(anyhow::anyhow!(
                "Plugin '{}' is already loaded",
                metadata.name
            ));
        }

        // Initialize the plugin
        wrapper.initialize()?;

        let loaded_plugin = LoadedPlugin {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
        };

        // Store the library and plugin
        let library_arc = Arc::new(library);
        self.loaded_libraries
            .insert(metadata.name.clone(), library_arc);
        self.loaded_plugins.insert(metadata.name.clone(), wrapper);

        info!(
            "Successfully loaded plugin '{}' version '{}' from {:?}",
            loaded_plugin.name, loaded_plugin.version, library_path
        );

        Ok(loaded_plugin)
    }

    /// Load a plugin from a library file (fallback for non-plugins builds)
    #[cfg(not(feature = "plugins"))]
    pub fn load_plugin(&mut self, _library_path: &Path) -> Result<LoadedPlugin> {
        anyhow::bail!("Plugin loading requires 'plugins' feature")
    }

    /// Load all plugins from the plugins directory
    pub fn load_all_plugins(&mut self) -> Result<Vec<LoadedPlugin>> {
        let plugin_paths = self.discover_plugins()?;
        let mut loaded_plugins = Vec::new();

        for path in plugin_paths {
            match self.load_plugin(&path) {
                Ok(plugin) => {
                    loaded_plugins.push(plugin);
                }
                Err(e) => {
                    error!("Failed to load plugin from {:?}: {}", path, e);
                    // Continue loading other plugins even if one fails
                }
            }
        }

        info!("Loaded {} plugins successfully", loaded_plugins.len());
        Ok(loaded_plugins)
    }

    /// Unload a plugin by name
    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        if let Some(wrapper) = self.loaded_plugins.get(name) {
            // Shutdown the plugin
            if let Err(e) = wrapper.shutdown() {
                warn!("Error shutting down plugin '{}': {}", name, e);
            }
        }

        // Remove from collections
        self.loaded_plugins.remove(name);
        #[cfg(feature = "plugins")]
        {
            self.loaded_libraries.remove(name);
        }

        info!("Unloaded plugin '{}'", name);
        Ok(())
    }

    /// Unload all plugins
    pub fn unload_all_plugins(&mut self) -> Result<()> {
        let plugin_names: Vec<String> = self.loaded_plugins.keys().cloned().collect();

        for name in plugin_names {
            if let Err(e) = self.unload_plugin(&name) {
                error!("Failed to unload plugin '{}': {}", name, e);
            }
        }

        info!("Unloaded all plugins");
        Ok(())
    }

    /// Get all loaded plugin names
    pub fn get_loaded_plugin_names(&self) -> Vec<String> {
        self.loaded_plugins.keys().cloned().collect()
    }

    /// Check if a plugin is loaded
    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.loaded_plugins.contains_key(name)
    }

    /// Get plugin metadata
    pub fn get_plugin_metadata(&self, name: &str) -> Option<&super::interface::PluginMetadata> {
        self.loaded_plugins.get(name).map(|p| p.metadata())
    }
}

impl Drop for DynamicPluginLoader {
    fn drop(&mut self) {
        if let Err(e) = self.unload_all_plugins() {
            error!("Error unloading plugins during drop: {}", e);
        }
    }
}

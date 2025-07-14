// C-compatible plugin interface for dynamic loading
// This defines the interface that plugins must implement

use anyhow::Result;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

#[cfg(feature = "plugins")]
use serde_json::Value;

/// C-compatible plugin interface structure
/// This is the main interface that dynamic plugins must export
#[repr(C)]
pub struct PluginInterface {
    /// Get plugin name
    pub get_name: extern "C" fn() -> *const c_char,

    /// Get plugin version
    pub get_version: extern "C" fn() -> *const c_char,

    /// Get plugin description
    pub get_description: extern "C" fn() -> *const c_char,

    /// Initialize the plugin
    pub initialize: extern "C" fn() -> c_int,

    /// Shutdown the plugin
    pub shutdown: extern "C" fn() -> c_int,

    /// Handle track loading (optional)
    pub on_track_load: Option<extern "C" fn(*const c_char) -> *const c_char>,

    /// Handle filter application (optional)
    pub on_filters_apply: Option<extern "C" fn(*const c_char) -> *const c_char>,

    /// Handle player events (optional)
    pub on_player_event: Option<extern "C" fn(*const c_char) -> c_int>,

    /// Get plugin configuration schema (optional)
    pub get_config_schema: Option<extern "C" fn() -> *const c_char>,

    /// Update plugin configuration (optional)
    pub update_config: Option<extern "C" fn(*const c_char) -> c_int>,
}

/// Plugin metadata structure
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    #[cfg(feature = "plugins")]
    pub config_schema: Option<Value>,
}

/// Plugin interface wrapper for safe Rust interaction
pub struct PluginInterfaceWrapper {
    interface: PluginInterface,
    metadata: PluginMetadata,
}

impl PluginInterfaceWrapper {
    /// Create a new plugin interface wrapper
    pub fn new(interface: PluginInterface) -> Result<Self> {
        // Extract metadata from the plugin
        let name = unsafe {
            let name_ptr = (interface.get_name)();
            if name_ptr.is_null() {
                return Err(anyhow::anyhow!("Plugin name is null"));
            }
            CStr::from_ptr(name_ptr).to_string_lossy().to_string()
        };

        let version = unsafe {
            let version_ptr = (interface.get_version)();
            if version_ptr.is_null() {
                return Err(anyhow::anyhow!("Plugin version is null"));
            }
            CStr::from_ptr(version_ptr).to_string_lossy().to_string()
        };

        let description = unsafe {
            let desc_ptr = (interface.get_description)();
            if desc_ptr.is_null() {
                "No description provided".to_string()
            } else {
                CStr::from_ptr(desc_ptr).to_string_lossy().to_string()
            }
        };

        // Get configuration schema if available
        #[cfg(feature = "plugins")]
        let config_schema: Option<Value> = if let Some(get_schema) = interface.get_config_schema {
            unsafe {
                let schema_ptr = get_schema();
                if !schema_ptr.is_null() {
                    let schema_str = CStr::from_ptr(schema_ptr).to_string_lossy();
                    serde_json::from_str(&schema_str).ok()
                } else {
                    None
                }
            }
        } else {
            None
        };

        #[cfg(not(feature = "plugins"))]
        let config_schema: Option<()> = None;

        let metadata = PluginMetadata {
            name,
            version,
            description,
            #[cfg(feature = "plugins")]
            config_schema,
        };

        Ok(Self {
            interface,
            metadata,
        })
    }

    /// Get plugin metadata
    pub fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// Initialize the plugin
    pub fn initialize(&self) -> Result<()> {
        let result = (self.interface.initialize)();
        if result == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Plugin initialization failed with code: {}",
                result
            ))
        }
    }

    /// Shutdown the plugin
    pub fn shutdown(&self) -> Result<()> {
        let result = (self.interface.shutdown)();
        if result == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Plugin shutdown failed with code: {}",
                result
            ))
        }
    }
}

/// Plugin interface constants
pub const PLUGIN_INTERFACE_SYMBOL: &[u8] = b"lavalink_plugin_interface\0";

/// Helper function to free C string (should be called by plugin)
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure:
/// - `ptr` was allocated by a compatible allocator (e.g., CString::into_raw)
/// - `ptr` is not used after this function is called
/// - `ptr` is not freed multiple times
#[no_mangle]
pub unsafe extern "C" fn lavalink_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

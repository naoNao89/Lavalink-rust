use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};

// Plugin state
static INITIALIZED: AtomicBool = AtomicBool::new(false);

// Plugin interface structure matching the Rust Lavalink expectations
#[repr(C)]
pub struct PluginInterface {
    pub get_name: extern "C" fn() -> *const c_char,
    pub get_version: extern "C" fn() -> *const c_char,
    pub get_description: extern "C" fn() -> *const c_char,
    pub initialize: extern "C" fn() -> c_int,
    pub shutdown: extern "C" fn() -> c_int,
    pub on_track_load: Option<extern "C" fn(*const c_char) -> *const c_char>,
    pub on_filters_apply: Option<extern "C" fn(*const c_char) -> *const c_char>,
    pub on_player_event: Option<extern "C" fn(*const c_char) -> c_int>,
    pub get_config_schema: Option<extern "C" fn() -> *const c_char>,
    pub update_config: Option<extern "C" fn(*const c_char) -> c_int>,
}

// Plugin interface functions
extern "C" fn get_name() -> *const c_char {
    CString::new("example-dynamic-plugin").unwrap().into_raw()
}

extern "C" fn get_version() -> *const c_char {
    CString::new("1.0.0").unwrap().into_raw()
}

extern "C" fn get_description() -> *const c_char {
    CString::new("Example dynamic plugin for Rust Lavalink").unwrap().into_raw()
}

extern "C" fn initialize() -> c_int {
    if INITIALIZED.load(Ordering::Relaxed) {
        return 1; // Already initialized
    }
    
    INITIALIZED.store(true, Ordering::Relaxed);
    eprintln!("🔌 Example Dynamic Plugin: Initialized successfully!");
    0 // Success
}

extern "C" fn shutdown() -> c_int {
    if !INITIALIZED.load(Ordering::Relaxed) {
        return 1; // Not initialized
    }
    
    INITIALIZED.store(false, Ordering::Relaxed);
    eprintln!("🔌 Example Dynamic Plugin: Shutdown completed!");
    0 // Success
}

extern "C" fn on_track_load(identifier: *const c_char) -> *const c_char {
    if identifier.is_null() {
        return std::ptr::null();
    }
    
    unsafe {
        let id_str = CStr::from_ptr(identifier).to_string_lossy();
        let result = format!("🎵 Dynamic Plugin processed track: {}", id_str);
        eprintln!("{}", result);
        CString::new(result).unwrap().into_raw()
    }
}

extern "C" fn on_player_event(event: *const c_char) -> c_int {
    if event.is_null() {
        return 1;
    }
    
    unsafe {
        let event_str = CStr::from_ptr(event).to_string_lossy();
        eprintln!("🎧 Dynamic Plugin received event: {}", event_str);
    }
    
    0 // Success
}

extern "C" fn get_config_schema() -> *const c_char {
    let schema = r#"{
        "type": "object",
        "title": "Example Dynamic Plugin Configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "default": true,
                "description": "Enable or disable the plugin"
            },
            "log_level": {
                "type": "string",
                "default": "info",
                "enum": ["debug", "info", "warn", "error"],
                "description": "Plugin logging level"
            }
        }
    }"#;
    
    CString::new(schema).unwrap().into_raw()
}

// Main plugin interface export
#[no_mangle]
pub extern "C" fn lavalink_plugin_interface() -> PluginInterface {
    PluginInterface {
        get_name,
        get_version,
        get_description,
        initialize,
        shutdown,
        on_track_load: Some(on_track_load),
        on_filters_apply: None,
        on_player_event: Some(on_player_event),
        get_config_schema: Some(get_config_schema),
        update_config: None,
    }
}

// Memory cleanup function
#[no_mangle]
pub unsafe extern "C" fn lavalink_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

---
description: Rust plugin system development guide for Lavalink Rust
---

# Plugin Development Guide

Lavalink Rust features a completely redesigned plugin system that uses dynamic libraries instead of JAR files, providing better performance and cross-language compatibility.

## Overview

### Key Differences from Java Lavalink

| Aspect | Java Lavalink | Rust Lavalink |
|--------|---------------|---------------|
| **Plugin Format** | JAR files | Dynamic libraries (.so/.dll/.dylib) |
| **Language Support** | Java/Kotlin only | Any language with C FFI |
| **Loading Mechanism** | JVM ClassLoader | libloading |
| **Interface** | Java interfaces | C-compatible ABI |
| **Performance** | JVM overhead | Native performance |
| **Memory Safety** | JVM managed | Rust ownership model |

### Plugin Architecture

```
┌─────────────────────────────────────┐
│           Lavalink Rust             │
├─────────────────────────────────────┤
│         Plugin Manager              │
├─────────────────────────────────────┤
│    Dynamic Plugin Loader            │
├─────────────────────────────────────┤
│  Plugin Interface (C ABI)           │
├─────────────────────────────────────┤
│  Plugin Libraries (.so/.dll/.dylib) │
└─────────────────────────────────────┘
```

## Plugin Interface

### C-Compatible Interface

All plugins must export a C-compatible interface defined in `PluginInterface`:

```c
// Required exports for all plugins
extern "C" {
    // Plugin metadata
    const char* get_name();
    const char* get_version();
    const char* get_description();
    
    // Lifecycle
    int initialize();
    int shutdown();
    
    // Optional hooks
    const char* on_track_load(const char* track_json);
    const char* on_filters_apply(const char* filters_json);
    int on_player_event(const char* event_json);
    
    // Configuration
    const char* get_config_schema();
    int update_config(const char* config_json);
}
```

### Plugin Metadata

Every plugin must provide basic metadata:

```rust
#[no_mangle]
pub extern "C" fn get_name() -> *const c_char {
    CString::new("my-plugin").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn get_version() -> *const c_char {
    CString::new("1.0.0").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn get_description() -> *const c_char {
    CString::new("My awesome Lavalink plugin").unwrap().into_raw()
}
```

## Creating a Rust Plugin

### Project Setup

1. **Create a new Rust library project:**
   ```bash
   cargo new --lib my-lavalink-plugin
   cd my-lavalink-plugin
   ```

2. **Configure Cargo.toml:**
   ```toml
   [package]
   name = "my-lavalink-plugin"
   version = "1.0.0"
   edition = "2021"
   
   [lib]
   crate-type = ["cdylib"]  # Important: creates dynamic library
   
   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   libc = "0.2"
   ```

3. **Basic plugin structure:**
   ```rust
   use std::ffi::{CStr, CString};
   use std::os::raw::c_char;
   use serde::{Deserialize, Serialize};
   
   // Plugin state (if needed)
   static mut PLUGIN_STATE: Option<MyPluginState> = None;
   
   struct MyPluginState {
       initialized: bool,
       config: PluginConfig,
   }
   
   #[derive(Deserialize, Serialize)]
   struct PluginConfig {
       enabled: bool,
       custom_setting: String,
   }
   ```

### Implementing Required Functions

```rust
#[no_mangle]
pub extern "C" fn get_name() -> *const c_char {
    CString::new("my-plugin").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn get_version() -> *const c_char {
    CString::new("1.0.0").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn get_description() -> *const c_char {
    CString::new("A sample Lavalink Rust plugin").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn initialize() -> i32 {
    unsafe {
        PLUGIN_STATE = Some(MyPluginState {
            initialized: true,
            config: PluginConfig {
                enabled: true,
                custom_setting: "default".to_string(),
            },
        });
    }
    
    // Return 0 for success, non-zero for error
    0
}

#[no_mangle]
pub extern "C" fn shutdown() -> i32 {
    unsafe {
        PLUGIN_STATE = None;
    }
    0
}
```

### Implementing Optional Hooks

#### Track Loading Hook

```rust
#[no_mangle]
pub extern "C" fn on_track_load(track_json: *const c_char) -> *const c_char {
    if track_json.is_null() {
        return std::ptr::null();
    }
    
    let track_str = unsafe {
        CStr::from_ptr(track_json).to_string_lossy()
    };
    
    // Parse track JSON
    if let Ok(mut track) = serde_json::from_str::<serde_json::Value>(&track_str) {
        // Modify track (example: add custom metadata)
        if let Some(info) = track.get_mut("info") {
            info["customField"] = serde_json::Value::String("modified by my-plugin".to_string());
        }
        
        // Return modified track JSON
        if let Ok(modified_json) = serde_json::to_string(&track) {
            return CString::new(modified_json).unwrap().into_raw();
        }
    }
    
    // Return null if no modification
    std::ptr::null()
}
```

#### Filter Application Hook

```rust
#[no_mangle]
pub extern "C" fn on_filters_apply(filters_json: *const c_char) -> *const c_char {
    if filters_json.is_null() {
        return std::ptr::null();
    }
    
    let filters_str = unsafe {
        CStr::from_ptr(filters_json).to_string_lossy()
    };
    
    // Parse and modify filters
    if let Ok(mut filters) = serde_json::from_str::<serde_json::Value>(&filters_str) {
        // Example: Add custom filter
        filters["customFilter"] = serde_json::json!({
            "enabled": true,
            "intensity": 0.5
        });
        
        if let Ok(modified_json) = serde_json::to_string(&filters) {
            return CString::new(modified_json).unwrap().into_raw();
        }
    }
    
    std::ptr::null()
}
```

#### Player Event Hook

```rust
#[no_mangle]
pub extern "C" fn on_player_event(event_json: *const c_char) -> i32 {
    if event_json.is_null() {
        return -1;
    }
    
    let event_str = unsafe {
        CStr::from_ptr(event_json).to_string_lossy()
    };
    
    // Parse and handle event
    if let Ok(event) = serde_json::from_str::<serde_json::Value>(&event_str) {
        if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
            match event_type {
                "TrackStartEvent" => {
                    // Handle track start
                    println!("Track started: {:?}", event);
                }
                "TrackEndEvent" => {
                    // Handle track end
                    println!("Track ended: {:?}", event);
                }
                _ => {
                    // Handle other events
                }
            }
        }
    }
    
    0 // Success
}
```

### Configuration Support

```rust
#[no_mangle]
pub extern "C" fn get_config_schema() -> *const c_char {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "enabled": {
                "type": "boolean",
                "default": true,
                "description": "Enable the plugin"
            },
            "custom_setting": {
                "type": "string",
                "default": "default",
                "description": "Custom plugin setting"
            }
        }
    });
    
    CString::new(schema.to_string()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn update_config(config_json: *const c_char) -> i32 {
    if config_json.is_null() {
        return -1;
    }
    
    let config_str = unsafe {
        CStr::from_ptr(config_json).to_string_lossy()
    };
    
    if let Ok(new_config) = serde_json::from_str::<PluginConfig>(&config_str) {
        unsafe {
            if let Some(ref mut state) = PLUGIN_STATE {
                state.config = new_config;
                return 0; // Success
            }
        }
    }
    
    -1 // Error
}
```

## Building and Testing

### Building the Plugin

```bash
# Build the plugin
cargo build --release

# The plugin will be created as:
# target/release/libmy_lavalink_plugin.so (Linux)
# target/release/my_lavalink_plugin.dll (Windows)
# target/release/libmy_lavalink_plugin.dylib (macOS)
```

### Testing the Plugin

1. **Copy to plugins directory:**
   ```bash
   cp target/release/libmy_lavalink_plugin.so /path/to/lavalink/plugins/
   ```

2. **Configure Lavalink:**
   ```yaml
   lavalink:
     plugins:
       plugins_dir: "./plugins"
   ```

3. **Start Lavalink and check logs:**
   ```bash
   ./lavalink-rust
   # Look for plugin loading messages
   ```

## Cross-Language Plugin Development

### C Plugin Example

```c
// my_plugin.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

const char* get_name() {
    return "c-plugin";
}

const char* get_version() {
    return "1.0.0";
}

const char* get_description() {
    return "A C plugin for Lavalink Rust";
}

int initialize() {
    printf("C plugin initialized\n");
    return 0;
}

int shutdown() {
    printf("C plugin shutdown\n");
    return 0;
}

// Compile with: gcc -shared -fPIC -o libmy_plugin.so my_plugin.c
```

### Python Plugin Example (using ctypes)

```python
# my_plugin.py
import ctypes
from ctypes import c_char_p, c_int

# Create shared library
# This would need to be compiled to a shared library using tools like Cython

def get_name():
    return b"python-plugin"

def get_version():
    return b"1.0.0"

def get_description():
    return b"A Python plugin for Lavalink Rust"

def initialize():
    print("Python plugin initialized")
    return 0

def shutdown():
    print("Python plugin shutdown")
    return 0
```

## Best Practices

### Memory Management

```rust
// Always use CString for returning strings
#[no_mangle]
pub extern "C" fn get_name() -> *const c_char {
    // Good: Proper memory management
    CString::new("my-plugin").unwrap().into_raw()
}

// Free memory when needed (if plugin manages its own memory)
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            CString::from_raw(ptr);
        }
    }
}
```

### Error Handling

```rust
#[no_mangle]
pub extern "C" fn on_track_load(track_json: *const c_char) -> *const c_char {
    // Always validate input
    if track_json.is_null() {
        return std::ptr::null();
    }
    
    // Use proper error handling
    match process_track(track_json) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Plugin error: {}", e);
            std::ptr::null()
        }
    }
}
```

### Thread Safety

```rust
use std::sync::Mutex;

// Use proper synchronization for shared state
static PLUGIN_STATE: Mutex<Option<MyPluginState>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn initialize() -> i32 {
    match PLUGIN_STATE.lock() {
        Ok(mut state) => {
            *state = Some(MyPluginState::new());
            0
        }
        Err(_) => -1
    }
}
```

## Debugging and Troubleshooting

### Common Issues

1. **Plugin not loading:**
   - Check file permissions
   - Verify library format (.so/.dll/.dylib)
   - Ensure all required symbols are exported

2. **Crashes on load:**
   - Check for missing dependencies
   - Verify C ABI compatibility
   - Use proper memory management

3. **Functions not called:**
   - Ensure proper `#[no_mangle]` attribute
   - Verify function signatures match interface
   - Check return values (0 = success, non-zero = error)

### Debugging Tools

```bash
# Check exported symbols (Linux)
nm -D libmy_plugin.so | grep get_name

# Check dependencies (Linux)
ldd libmy_plugin.so

# Check symbols (macOS)
nm -gU libmy_plugin.dylib

# Check dependencies (macOS)
otool -L libmy_plugin.dylib
```

## Migration from Java Plugins

### Conceptual Mapping

| Java Concept | Rust Equivalent |
|--------------|-----------------|
| `@Component` | Plugin registration |
| `@EventListener` | Event hooks |
| `ApplicationContext` | Plugin state |
| `@ConfigurationProperties` | Configuration schema |
| JAR loading | Dynamic library loading |

### Migration Steps

1. **Identify plugin functionality**
2. **Design C interface**
3. **Implement core logic in Rust**
4. **Add proper error handling**
5. **Test thoroughly**
6. **Document configuration changes**

For more information, see the [Migration Guide](../migration/from-java.md).

## Next Steps

- **Advanced Topics**: [Plugin Architecture](../advanced/plugin-architecture.md)
- **Examples**: [Plugin Examples](examples/)
- **API Reference**: [Plugin API](../api/plugins.md)
- **Troubleshooting**: [Plugin Troubleshooting](../getting-started/troubleshooting.md#plugin-issues)

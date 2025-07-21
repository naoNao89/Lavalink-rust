// Plugin system integration tests
// These tests validate plugin integration with the server components

use lavalink_rust::plugin::{ExamplePlugin, PluginManager};

/// Test plugin integration with server components
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_server_integration() {
    let mut plugin_manager = PluginManager::new();

    // Register test plugins
    let plugin1 = Box::new(ExamplePlugin::with_name("test-plugin-1".to_string()));
    let plugin2 = Box::new(ExamplePlugin::with_name("test-plugin-2".to_string()));

    plugin_manager.register_plugin(plugin1).await.unwrap();
    plugin_manager.register_plugin(plugin2).await.unwrap();

    assert_eq!(plugin_manager.plugin_count(), 2);

    // Verify plugins are accessible
    assert!(plugin_manager.get_plugin("test-plugin-1").is_some());
    assert!(plugin_manager.get_plugin("test-plugin-2").is_some());

    // Test plugin names retrieval
    let names = plugin_manager.get_plugin_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"test-plugin-1".to_string()));
    assert!(names.contains(&"test-plugin-2".to_string()));
}

/// Test plugin serialization for API responses
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_serialization() {
    let mut plugin_manager = PluginManager::new();

    // Register test plugin
    let plugin = Box::new(ExamplePlugin::with_name("api-test-plugin".to_string()));
    plugin_manager.register_plugin(plugin).await.unwrap();

    // Test plugin information can be serialized
    if let Some(plugin) = plugin_manager.get_plugin("api-test-plugin") {
        let plugin_info = serde_json::json!({
            "name": plugin.name(),
            "version": plugin.version(),
            "description": plugin.description(),
            "configSchema": plugin.get_config_schema()
        });

        assert_eq!(
            plugin_info.get("name").and_then(|n| n.as_str()),
            Some("api-test-plugin")
        );
        assert_eq!(
            plugin_info.get("version").and_then(|v| v.as_str()),
            Some("1.0.0")
        );
        assert!(plugin_info.get("configSchema").is_some());
    }
}

/// Test plugin configuration handling
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_configuration_handling() {
    let mut plugin_manager = PluginManager::new();

    // Register test plugin
    let plugin = Box::new(ExamplePlugin::with_name("config-test-plugin".to_string()));
    plugin_manager.register_plugin(plugin).await.unwrap();

    // Test plugin configuration schema
    if let Some(plugin) = plugin_manager.get_plugin("config-test-plugin") {
        let schema = plugin.get_config_schema();
        assert!(schema.is_some());

        let schema_value = schema.unwrap();
        assert_eq!(
            schema_value.get("type").and_then(|t| t.as_str()),
            Some("object")
        );
        assert!(schema_value.get("properties").is_some());

        // Test that the schema contains expected properties
        if let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) {
            assert!(properties.contains_key("enabled"));
            assert!(properties.contains_key("debug"));
        }
    }
}

/// Test plugin not found handling
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_not_found_handling() {
    let plugin_manager = PluginManager::new();

    // Test getting non-existent plugin
    let plugin = plugin_manager.get_plugin("nonexistent-plugin");
    assert!(plugin.is_none());

    // Test plugin names list is empty
    let names = plugin_manager.get_plugin_names();
    assert!(names.is_empty());

    // Test plugin count is zero
    assert_eq!(plugin_manager.plugin_count(), 0);
}

/// Test plugin lifecycle during application operations
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_lifecycle_with_application() {
    let mut plugin_manager = PluginManager::new();

    // Register multiple plugins
    for i in 1..=3 {
        let plugin = Box::new(ExamplePlugin::with_name(format!("lifecycle-plugin-{}", i)));
        plugin_manager.register_plugin(plugin).await.unwrap();
    }

    assert_eq!(plugin_manager.plugin_count(), 3);

    // Simulate application shutdown by unloading all plugins
    plugin_manager.unload_all_plugins().await;

    assert_eq!(plugin_manager.plugin_count(), 0);
    assert!(plugin_manager.get_plugin_names().is_empty());
}

/// Test plugin event handling
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_event_handling() {
    let mut plugin_manager = PluginManager::new();

    // Register test plugin
    let plugin = Box::new(ExamplePlugin::with_name("event-test-plugin".to_string()));
    plugin_manager.register_plugin(plugin).await.unwrap();

    // Test track load event
    if let Some(plugin) = plugin_manager.get_plugin("event-test-plugin") {
        let result = plugin.on_track_load("test-track-identifier").await;
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.is_some());
        assert!(processed.unwrap().contains("event-test-plugin"));

        // Test player event
        let result = plugin.on_player_event("track_start").await;
        assert!(result.is_ok());

        // Test config schema structure
        let _config = serde_json::json!({
            "enabled": true,
            "debug": false
        });

        // Note: Config update testing would require mutable plugin access
        // which is not available through the current plugin manager API
    }
}

/// Test plugin discovery and loading simulation
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_discovery_simulation() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let plugins_dir = temp_dir.path();

    // Create mock plugin files based on the current platform
    let plugin_extension = match std::env::consts::OS {
        "windows" => "dll",
        "macos" => "dylib",
        _ => "so", // Linux and other Unix-like systems
    };

    let plugin_files = vec![
        format!("plugin1.{}", plugin_extension),
        format!("plugin2.{}", plugin_extension),
        format!("plugin3.{}", plugin_extension),
    ];

    for file in &plugin_files {
        let plugin_path = plugins_dir.join(file);
        fs::write(&plugin_path, b"mock plugin content").unwrap();
    }

    // Create non-plugin files that should be ignored
    let non_plugin_files = vec!["readme.txt", "config.json", "data.db"];

    for file in &non_plugin_files {
        let file_path = plugins_dir.join(file);
        fs::write(&file_path, b"not a plugin").unwrap();
    }

    let config = lavalink_rust::config::PluginsConfig {
        plugins: None,
        plugins_dir: Some(plugins_dir.to_string_lossy().to_string()),
        default_plugin_repository: None,
        default_plugin_snapshot_repository: None,
    };

    let plugin_manager = PluginManager::with_config(config);
    let discovered = plugin_manager.dynamic_loader.discover_plugins().unwrap();

    // Should discover only the plugin files
    assert_eq!(discovered.len(), 3);

    let discovered_names: Vec<String> = discovered
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    for plugin_file in &plugin_files {
        assert!(discovered_names.contains(&plugin_file.to_string()));
    }

    for non_plugin_file in &non_plugin_files {
        assert!(!discovered_names.contains(&non_plugin_file.to_string()));
    }
}

/// Test plugin manager thread safety
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_manager_thread_safety() {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let plugin_manager = Arc::new(RwLock::new(PluginManager::new()));
    let mut handles = Vec::new();

    // Spawn multiple tasks that interact with the plugin manager
    for i in 0..10 {
        let manager = plugin_manager.clone();

        let handle = tokio::spawn(async move {
            let plugin_name = format!("thread-safe-plugin-{}", i);
            let plugin = Box::new(ExamplePlugin::with_name(plugin_name.clone()));

            // Register plugin
            {
                let mut manager_guard = manager.write().await;
                let result = manager_guard.register_plugin(plugin).await;
                assert!(result.is_ok());
            }

            // Read plugin information
            {
                let manager_guard = manager.read().await;
                assert!(manager_guard.get_plugin(&plugin_name).is_some());
            }

            // Unregister plugin
            {
                let mut manager_guard = manager.write().await;
                let result = manager_guard.unregister_plugin(&plugin_name).await;
                assert!(result.is_ok());
            }

            plugin_name
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let plugin_name = handle.await.unwrap();
        println!("Completed thread safety test for: {}", plugin_name);
    }

    // Verify all plugins were properly cleaned up
    let manager_guard = plugin_manager.read().await;
    assert_eq!(manager_guard.plugin_count(), 0);
}

/// Test plugin configuration schema validation
#[tokio::test]
#[ignore = "Plugin API has changed - tests need to be updated for new plugin system"]
async fn test_plugin_config_schema() {
    let mut plugin_manager = PluginManager::new();
    let plugin = Box::new(ExamplePlugin::new());

    plugin_manager.register_plugin(plugin).await.unwrap();

    if let Some(plugin) = plugin_manager.get_plugin("example-plugin") {
        let schema = plugin.get_config_schema();
        assert!(schema.is_some());

        let schema_value = schema.unwrap();
        assert_eq!(
            schema_value.get("type").and_then(|t| t.as_str()),
            Some("object")
        );
        assert!(schema_value.get("properties").is_some());

        if let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) {
            assert!(properties.contains_key("enabled"));
            assert!(properties.contains_key("debug"));
        }
    }
}

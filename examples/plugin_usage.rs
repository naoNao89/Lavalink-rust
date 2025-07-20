// Simple plugin usage example
// This demonstrates basic plugin registration and usage

use anyhow::Result;
use lavalink_rust::plugin::{ExamplePlugin, PluginManager};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔌 Lavalink-rust Plugin System Demo");
    println!("===================================");

    // Create a new plugin manager
    let mut plugin_manager = PluginManager::new();
    println!("✅ Plugin manager created");

    // Register some example plugins
    println!("\n📦 Registering plugins...");

    let plugin1 = Box::new(ExamplePlugin::with_name("demo-plugin-1".to_string()));
    let plugin2 = Box::new(ExamplePlugin::with_name("demo-plugin-2".to_string()));
    let plugin3 = Box::new(ExamplePlugin::with_name("demo-plugin-3".to_string()));

    plugin_manager.register_plugin(plugin1).await?;
    println!("  ✅ Registered: demo-plugin-1");

    plugin_manager.register_plugin(plugin2).await?;
    println!("  ✅ Registered: demo-plugin-2");

    plugin_manager.register_plugin(plugin3).await?;
    println!("  ✅ Registered: demo-plugin-3");

    // Display registered plugins
    println!("\n📋 Currently registered plugins:");
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

    println!("\n📊 Plugin count: {}", plugin_manager.plugin_count());

    // Test plugin functionality
    println!("\n🧪 Testing plugin functionality...");

    if let Some(plugin) = plugin_manager.get_plugin("demo-plugin-1") {
        // Test track loading
        let track_result = plugin.on_track_load("example-track-123").await?;
        if let Some(result) = track_result {
            println!("  🎵 Track load result: {}", result);
        }

        // Test player events
        plugin.on_player_event("track_start").await?;
        println!("  🎧 Sent track_start event");

        plugin.on_player_event("volume_change").await?;
        println!("  🔊 Sent volume_change event");

        plugin.on_player_event("track_end").await?;
        println!("  ⏹️ Sent track_end event");

        // Display configuration schema
        if let Some(schema) = plugin.get_config_schema() {
            println!("  📝 Configuration schema available:");
            println!(
                "     Type: {}",
                schema.get("type").unwrap_or(&serde_json::Value::Null)
            );
            if let Some(properties) = schema.get("properties") {
                println!("     Properties: {}", properties.as_object().unwrap().len());
            }
        }
    }

    // Test plugin discovery
    println!("\n🔍 Testing dynamic plugin discovery...");
    let discovered = plugin_manager.dynamic_loader.discover_plugins()?;
    println!(
        "  📁 Discovered {} potential plugin files",
        discovered.len()
    );

    for path in discovered {
        println!("    - {}", path.display());
    }

    // Test plugin metadata access
    println!("\n📊 Plugin metadata:");
    let dynamic_names = plugin_manager.get_dynamic_plugin_names();
    println!("  Dynamic plugins loaded: {}", dynamic_names.len());

    for name in &dynamic_names {
        if let Some(metadata) = plugin_manager.get_dynamic_plugin_metadata(name) {
            println!("    - {}: {}", metadata.name, metadata.version);
        }
    }

    // Unregister a plugin
    println!("\n🗑️ Unregistering demo-plugin-2...");
    plugin_manager.unregister_plugin("demo-plugin-2").await?;
    println!("  ✅ Plugin unregistered");
    println!("  📊 Remaining plugins: {}", plugin_manager.plugin_count());

    // List remaining plugins
    println!("\n📋 Remaining plugins:");
    for name in plugin_manager.get_plugin_names() {
        println!("  - {}", name);
    }

    // Test error handling
    println!("\n❌ Testing error handling...");

    // Try to register duplicate plugin
    let duplicate_plugin = Box::new(ExamplePlugin::with_name("demo-plugin-1".to_string()));
    match plugin_manager.register_plugin(duplicate_plugin).await {
        Ok(_) => println!("  ⚠️ Unexpected success registering duplicate"),
        Err(e) => println!("  ✅ Correctly rejected duplicate: {}", e),
    }

    // Try to unregister non-existent plugin
    match plugin_manager
        .unregister_plugin("non-existent-plugin")
        .await
    {
        Ok(_) => println!("  ⚠️ Unexpected success unregistering non-existent plugin"),
        Err(e) => println!("  ✅ Correctly rejected non-existent plugin: {}", e),
    }

    // Try to get non-existent plugin
    match plugin_manager.get_plugin("non-existent-plugin") {
        Some(_) => println!("  ⚠️ Unexpected success getting non-existent plugin"),
        None => println!("  ✅ Correctly returned None for non-existent plugin"),
    }

    // Cleanup - unload all plugins
    println!("\n🧹 Cleaning up...");
    plugin_manager.unload_all_plugins().await;
    println!("  ✅ All plugins unloaded");
    println!("  📊 Final plugin count: {}", plugin_manager.plugin_count());

    println!("\n🎉 Plugin system demo completed successfully!");
    println!("   The plugin system is ready for production use.");
    println!("   You can now:");
    println!("   - Create custom plugins by implementing LavalinkPlugin trait");
    println!("   - Load dynamic plugins from shared libraries");
    println!("   - Manage plugin lifecycle (register, unregister, configure)");
    println!("   - Handle plugin events and track processing");

    Ok(())
}

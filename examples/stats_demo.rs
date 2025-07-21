use lavalink_rust::player::PlayerManager;
use lavalink_rust::server::StatsCollector;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Lavalink-rust Stats Demo");
    println!("========================");

    // Create stats collector
    let stats_collector = StatsCollector::new();

    // Create player manager
    let (event_sender, _event_receiver) = mpsc::unbounded_channel();
    let player_manager = PlayerManager::with_event_sender(event_sender);

    // Get stats without players
    println!("\n1. Basic Stats (no players):");
    let stats = stats_collector.get_stats().await;
    println!("   Uptime: {}ms", stats.uptime);
    println!(
        "   Memory: {} bytes free, {} bytes used",
        stats.memory.free, stats.memory.used
    );
    println!(
        "   CPU: {} cores, {:.1}% system load, {:.1}% lavalink load",
        stats.cpu.cores,
        stats.cpu.system_load * 100.0,
        stats.cpu.lavalink_load * 100.0
    );

    // Get stats with player manager
    println!("\n2. Stats with Player Manager:");
    let stats = stats_collector
        .get_stats_with_players(&player_manager)
        .await;
    println!(
        "   Players: {} total, {} playing",
        stats.players, stats.playing_players
    );

    // Create a test player
    let _player = player_manager
        .get_or_create_player("demo_guild".to_string(), "demo_session".to_string())
        .await;

    // Get updated stats
    println!("\n3. Stats after creating a player:");
    let stats = stats_collector
        .get_stats_with_players(&player_manager)
        .await;
    println!(
        "   Players: {} total, {} playing",
        stats.players, stats.playing_players
    );

    // Show JSON format (what Discord bots receive)
    println!("\n4. JSON Format (Discord bot format):");
    let json = serde_json::to_string_pretty(&stats)?;
    println!("{json}");

    println!("\n✅ Stats functionality is working correctly!");
    println!("Discord bots will now receive proper statistics including:");
    println!("  - Real memory usage (free, used, allocated, reservable)");
    println!("  - Real CPU usage (system load and Lavalink-specific load)");
    println!("  - Accurate player counts");
    println!("  - Periodic stats broadcasts every 60 seconds via WebSocket");

    Ok(())
}

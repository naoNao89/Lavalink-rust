// Performance and benchmark tests
// These tests validate performance characteristics and resource usage

use axum_test::TestServer;
use lavalink_rust::player::PlayerManager;
use lavalink_rust::server::LavalinkServer;
use lavalink_rust::test_utils::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Test server startup time
#[tokio::test]
async fn test_server_startup_performance() {
    let start_time = Instant::now();

    let config = create_test_config();
    let _server = LavalinkServer::new(config)
        .await
        .expect("Failed to create server");

    let startup_time = start_time.elapsed();

    // Server should start within 5 seconds
    assert!(
        startup_time < Duration::from_secs(5),
        "Server startup took too long: {startup_time:?}"
    );

    println!("Server startup time: {startup_time:?}");
}

/// Test sequential request handling performance
#[tokio::test]
async fn test_sequential_request_performance() {
    let server = create_test_server().await;
    let num_requests = 20; // Reduced for sequential testing
    let start_time = Instant::now();

    // Make sequential requests
    for _i in 0..num_requests {
        let response = server
            .get("/v4/stats")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status_ok();
    }

    let total_time = start_time.elapsed();

    // Should handle 20 sequential requests within 10 seconds
    assert!(
        total_time < Duration::from_secs(10),
        "Sequential requests took too long: {total_time:?}"
    );

    let avg_time = total_time / num_requests as u32;
    println!("Average request time for {num_requests} sequential requests: {avg_time:?}");
    println!("Total time for {num_requests} sequential requests: {total_time:?}");
}

/// Test track loading performance
#[tokio::test]
async fn test_track_loading_performance() {
    let server = create_test_server().await;
    let num_loads = 10; // Reduced for sequential testing
    let start_time = Instant::now();

    let mut load_times = Vec::new();

    // Make sequential track loading requests
    for i in 0..num_loads {
        let identifier = format!("http://example.com/test{i}.mp3");
        let load_start = Instant::now();

        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", &identifier)
            .await;

        response.assert_status_ok();

        let load_time = load_start.elapsed();
        load_times.push(load_time);
    }

    let total_time = start_time.elapsed();

    // Calculate statistics
    load_times.sort();

    let min_time = load_times.first().unwrap();
    let max_time = load_times.last().unwrap();
    let avg_time = total_time / num_loads as u32;
    let median_time = load_times[load_times.len() / 2];

    println!("Track loading performance for {num_loads} loads:");
    println!("  Total time: {total_time:?}");
    println!("  Average time: {avg_time:?}");
    println!("  Median time: {median_time:?}");
    println!("  Min time: {min_time:?}");
    println!("  Max time: {max_time:?}");

    // Each load should complete within 5 seconds
    assert!(
        max_time < &Duration::from_secs(5),
        "Track loading took too long: {max_time:?}"
    );
}

/// Test player creation and management performance
#[tokio::test]
async fn test_player_management_performance() {
    let player_manager = Arc::new(PlayerManager::new());
    let num_players = 50; // Reduced for sequential testing
    let start_time = Instant::now();

    let mut create_times = Vec::new();

    // Create players sequentially
    for i in 0..num_players {
        let guild_id = format!("guild_{i}");
        let session_id = format!("session_{i}");
        let create_start = Instant::now();

        let player = player_manager
            .get_or_create_player(guild_id.clone(), session_id)
            .await;

        // Perform some operations
        {
            let _player_guard = player.write().await;
            // Just access the player to simulate some work
        }

        let create_time = create_start.elapsed();
        create_times.push(create_time);
    }

    let total_time = start_time.elapsed();

    // Calculate statistics
    create_times.sort();

    let min_time = create_times.first().unwrap();
    let max_time = create_times.last().unwrap();
    let avg_time = total_time / num_players as u32;

    println!("Player management performance for {num_players} players:");
    println!("  Total time: {total_time:?}");
    println!("  Average time: {avg_time:?}");
    println!("  Min time: {min_time:?}");
    println!("  Max time: {max_time:?}");

    // Player creation should be fast
    assert!(
        max_time < &Duration::from_millis(500),
        "Player creation took too long: {max_time:?}"
    );

    // Total time should be reasonable
    assert!(
        total_time < Duration::from_secs(10),
        "Total player creation took too long: {total_time:?}"
    );
}

/// Test memory usage under load
#[tokio::test]
async fn test_memory_usage() {
    let server = create_test_server().await;

    // Get initial memory stats
    let initial_response = server
        .get("/v4/stats")
        .add_header(auth_header().0, auth_header().1)
        .await;

    initial_response.assert_status_ok();
    let initial_stats: serde_json::Value = initial_response.json();
    let initial_memory = initial_stats["memory"]["used"].as_u64().unwrap_or(0);

    // Perform many operations sequentially
    for i in 0..10 {
        // Reduced for sequential testing
        // Load tracks
        for j in 0..3 {
            let identifier = format!("http://example.com/test{i}_{j}.mp3");
            server
                .get("/v4/loadtracks")
                .add_header(auth_header().0, auth_header().1)
                .add_query_param("identifier", &identifier)
                .await;
        }
    }

    // Get final memory stats
    let final_response = server
        .get("/v4/stats")
        .add_header(auth_header().0, auth_header().1)
        .await;

    final_response.assert_status_ok();
    let final_stats: serde_json::Value = final_response.json();
    let final_memory = final_stats["memory"]["used"].as_u64().unwrap_or(0);

    let memory_increase = final_memory.saturating_sub(initial_memory);

    println!("Memory usage test:");
    println!("  Initial memory: {initial_memory} bytes");
    println!("  Final memory: {final_memory} bytes");
    println!("  Memory increase: {memory_increase} bytes");

    // Memory increase should be reasonable (less than 100MB for this test)
    assert!(
        memory_increase < 100_000_000,
        "Memory usage increased too much: {memory_increase} bytes"
    );
}

/// Test response time consistency
#[tokio::test]
async fn test_response_time_consistency() {
    let server = create_test_server().await;
    let num_requests = 30;
    let mut response_times = Vec::new();

    // Make sequential requests to measure consistency
    for _i in 0..num_requests {
        let start_time = Instant::now();

        let response = server
            .get("/v4/info")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status_ok();

        let response_time = start_time.elapsed();
        response_times.push(response_time);
    }

    // Calculate statistics
    response_times.sort();

    let min_time = response_times.first().unwrap();
    let max_time = response_times.last().unwrap();
    let median_time = response_times[response_times.len() / 2];
    let avg_time: Duration = response_times.iter().sum::<Duration>() / num_requests as u32;

    // Calculate 95th percentile
    let p95_index = (num_requests as f64 * 0.95) as usize;
    let p95_time = response_times[p95_index.min(response_times.len() - 1)];

    println!("Response time consistency for {num_requests} requests:");
    println!("  Average: {avg_time:?}");
    println!("  Median: {median_time:?}");
    println!("  Min: {min_time:?}");
    println!("  Max: {max_time:?}");
    println!("  95th percentile: {p95_time:?}");

    // Response times should be consistent
    assert!(
        p95_time < Duration::from_millis(500),
        "95th percentile response time too high: {p95_time:?}"
    );

    // Max response time shouldn't be too much higher than average
    let max_avg_ratio = max_time.as_nanos() as f64 / avg_time.as_nanos() as f64;
    assert!(
        max_avg_ratio < 20.0,
        "Response time variance too high: max/avg ratio = {max_avg_ratio:.2}"
    );
}

// Helper functions
async fn create_test_server() -> TestServer {
    let config = create_test_config();
    let server = LavalinkServer::new(config).await.unwrap();
    let app = server.build_router();
    TestServer::new(app).unwrap()
}

fn auth_header() -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::HeaderName::from_static("authorization"),
        axum::http::HeaderValue::from_static("youshallnotpass"),
    )
}

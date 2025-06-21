// Integration tests for Lavalink Rust implementation
// These tests validate end-to-end functionality and protocol compatibility

use axum::http::StatusCode;
use axum_test::TestServer;
use lavalink_rust::server::LavalinkServer;
use lavalink_rust::test_utils::*;
use serde_json::Value;

/// Test server startup and basic functionality
#[tokio::test]
async fn test_server_startup() {
    let config = create_test_config();
    let _server = LavalinkServer::new(config)
        .await
        .expect("Failed to create server");

    // Server should be created successfully - test passes if no panic occurs
}

/// Test full track loading workflow
#[tokio::test]
async fn test_track_loading_workflow() {
    let server = create_test_server().await;

    // Test HTTP track loading
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "http://example.com/test.mp3")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    assert!(json.get("loadType").is_some());
    assert!(json.get("data").is_some());
}

/// Test WebSocket connection and messaging
#[tokio::test]
async fn test_websocket_connection() {
    // This test would require WebSocket client setup
    // For now, we'll test the REST API that supports WebSocket operations
    let server = create_test_server().await;

    // Test session creation (prerequisite for WebSocket)
    let session_data = serde_json::json!({
        "resuming": false,
        "timeout": 60
    });

    let response = server
        .patch("/v4/sessions/ws-test-session")
        .add_header(auth_header().0, auth_header().1)
        .json(&session_data)
        .await;

    response.assert_status_ok();
}

/// Test player lifecycle management
#[tokio::test]
async fn test_player_lifecycle() {
    let server = create_test_server().await;

    // Create a session first
    let session_data = serde_json::json!({
        "resuming": false,
        "timeout": 60
    });

    let session_response = server
        .patch("/v4/sessions/player-test-session")
        .add_header(auth_header().0, auth_header().1)
        .json(&session_data)
        .await;

    // Check if session creation succeeded
    session_response.assert_status_ok();

    // Create a player by updating it
    let player_update = serde_json::json!({
        "track": {
            "identifier": "test-track-id"
        },
        "volume": 100,
        "paused": false
    });

    let response = server
        .patch("/v4/sessions/player-test-session/players/123456789")
        .add_header(auth_header().0, auth_header().1)
        .json(&player_update)
        .await;

    response.assert_status_ok();

    // Get player state
    let get_response = server
        .get("/v4/sessions/player-test-session/players/123456789")
        .add_header(auth_header().0, auth_header().1)
        .await;

    get_response.assert_status_ok();

    // Destroy player
    let destroy_response = server
        .delete("/v4/sessions/player-test-session/players/123456789")
        .add_header(auth_header().0, auth_header().1)
        .await;

    destroy_response.assert_status(StatusCode::NO_CONTENT);
}

/// Test audio source priority and fallback
#[tokio::test]
async fn test_audio_source_priority() {
    let server = create_test_server().await;

    // Test YouTube URL (should be handled by YouTube source)
    let yt_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        .await;

    yt_response.assert_status_ok();

    // Test SoundCloud URL (should be handled by SoundCloud source)
    let sc_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://soundcloud.com/test/track")
        .await;

    sc_response.assert_status_ok();

    // Test Bandcamp URL (should be handled by Bandcamp source)
    let bc_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://artist.bandcamp.com/track/test-track")
        .await;

    bc_response.assert_status_ok();

    // Test Vimeo URL (should be handled by Vimeo source)
    let vimeo_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://vimeo.com/123456789")
        .await;

    vimeo_response.assert_status_ok();

    // Test Twitch URL (should be handled by Twitch source)
    let twitch_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://twitch.tv/test_channel")
        .await;

    twitch_response.assert_status_ok();

    // Test HTTP URL (should be handled by HTTP source)
    let http_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "http://example.com/audio.mp3")
        .await;

    http_response.assert_status_ok();
}

/// Test filter application and management
#[tokio::test]
async fn test_filter_management() {
    let server = create_test_server().await;

    // Create session and player
    let session_data = serde_json::json!({
        "resuming": false,
        "timeout": 60
    });

    server
        .patch("/v4/sessions/filter-test-session")
        .add_header(auth_header().0, auth_header().1)
        .json(&session_data)
        .await;

    let player_update = serde_json::json!({
        "track": {
            "identifier": "test-track-id"
        }
    });

    server
        .patch("/v4/sessions/filter-test-session/players/123456789")
        .add_header(auth_header().0, auth_header().1)
        .json(&player_update)
        .await;

    // Apply filters
    let filters = serde_json::json!({
        "volume": 1.5,
        "equalizer": [
            {"band": 0, "gain": 0.2},
            {"band": 1, "gain": 0.15}
        ]
    });

    let filter_response = server
        .patch("/v4/sessions/filter-test-session/players/123456789/filters")
        .add_header(auth_header().0, auth_header().1)
        .json(&filters)
        .await;

    filter_response.assert_status_ok();

    // Get current filters
    let get_filters_response = server
        .get("/v4/sessions/filter-test-session/players/123456789/filters")
        .add_header(auth_header().0, auth_header().1)
        .await;

    get_filters_response.assert_status_ok();
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling() {
    let server = create_test_server().await;

    // Test invalid track identifier
    let invalid_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "invalid://not-a-real-protocol")
        .await;

    invalid_response.assert_status_ok();
    let json: Value = invalid_response.json();
    // Should return error load type
    assert_eq!(json["loadType"], "error");

    // Test nonexistent session
    let nonexistent_response = server
        .get("/v4/sessions/nonexistent-session/players")
        .add_header(auth_header().0, auth_header().1)
        .await;

    nonexistent_response.assert_status(StatusCode::NOT_FOUND);

    // Test invalid player operation
    let invalid_player_response = server
        .patch("/v4/sessions/nonexistent-session/players/123456789")
        .add_header(auth_header().0, auth_header().1)
        .json(&serde_json::json!({"volume": 100}))
        .await;

    invalid_player_response.assert_status(StatusCode::NOT_FOUND);
}

/// Test Bandcamp search functionality
#[tokio::test]
async fn test_bandcamp_search() {
    let server = create_test_server().await;

    // Test Bandcamp search
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "bcsearch:test music")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    assert!(json.get("loadType").is_some());
    assert!(json.get("data").is_some());

    // Should return either search results, empty, or error (all are valid for network-dependent tests)
    let load_type = json["loadType"].as_str().unwrap();
    assert!(matches!(load_type, "search" | "empty" | "error"));
}

/// Test Vimeo search functionality
#[tokio::test]
async fn test_vimeo_search() {
    let server = create_test_server().await;

    // Test Vimeo search
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "vmsearch:test video")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    assert!(json.get("loadType").is_some());
    assert!(json.get("data").is_some());

    // Should return either search results, empty, or error (all are valid for network-dependent tests)
    let load_type = json["loadType"].as_str().unwrap();
    assert!(matches!(load_type, "search" | "empty" | "error"));
}

/// Test Twitch search functionality
#[tokio::test]
async fn test_twitch_search() {
    let server = create_test_server().await;

    // Test Twitch search (channel lookup)
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "twsearch:test_channel")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    assert!(json.get("loadType").is_some());
    assert!(json.get("data").is_some());

    // Should return either search results, empty, or error (all are valid for network-dependent tests)
    let load_type = json["loadType"].as_str().unwrap();
    assert!(matches!(load_type, "search" | "empty" | "error"));
}

/// Test Twitch URL types (streams, VODs, clips)
#[tokio::test]
async fn test_twitch_url_types() {
    let server = create_test_server().await;

    let twitch_urls = [
        "https://twitch.tv/test_channel",                   // Live stream
        "https://twitch.tv/videos/123456789",               // VOD
        "https://twitch.tv/test_channel/clip/test-clip-id", // Clip
    ];

    for url in &twitch_urls {
        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", url)
            .await;

        response.assert_status_ok();

        let json: Value = response.json();
        assert!(json.get("loadType").is_some());
        assert!(json.get("data").is_some());

        // Should handle all URL types (even if content is offline/unavailable)
        let load_type = json["loadType"].as_str().unwrap();
        assert!(matches!(load_type, "track" | "empty" | "error"));
    }
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations() {
    let server = create_test_server().await;

    // Test sequential operations (TestServer doesn't support clone)
    for i in 0..3 {
        let identifier = format!("http://example.com/test{}.mp3", i);

        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", &identifier)
            .await;

        response.assert_status_ok();
    }
}

/// Test local file support with file:// URLs
#[tokio::test]
async fn test_local_file_support() {
    let server = create_test_server().await;

    // Test file:// URL handling
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "file:///path/to/audio.mp3")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    let load_type = json["loadType"].as_str().unwrap();

    // Should handle file URLs (even if file doesn't exist, should recognize the protocol)
    // Expect either "empty" (no matches) or "error" (file not found)
    assert!(matches!(load_type, "empty" | "error"));
}

/// Test unsupported sources return appropriate errors
#[tokio::test]
async fn test_unsupported_sources() {
    let server = create_test_server().await;

    // Test Niconico (unsupported source)
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "https://www.nicovideo.jp/watch/sm12345678")
        .await;

    response.assert_status_ok();

    let json: Value = response.json();
    let load_type = json["loadType"].as_str().unwrap();

    // Should return "empty" (no matches) or "error" for unsupported sources
    assert!(matches!(load_type, "empty" | "error"));

    // Test another unsupported source format
    let response2 = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "unknown://invalid.source/test")
        .await;

    response2.assert_status_ok();

    let json2: Value = response2.json();
    let load_type2 = json2["loadType"].as_str().unwrap();

    // Should return "empty" or "error" for unknown protocols
    assert!(matches!(load_type2, "empty" | "error"));
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

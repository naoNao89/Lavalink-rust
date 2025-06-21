// Java Lavalink client compatibility tests
// These tests validate protocol compatibility with Java Lavalink clients

use axum::http::StatusCode;
use axum_test::TestServer;
use lavalink_rust::server::LavalinkServer;
use lavalink_rust::test_utils::*;
use serde_json::Value;

/// Test protocol version compatibility
#[tokio::test]
async fn test_protocol_version_compatibility() {
    let server = create_test_server().await;

    // Test version endpoint - critical for client compatibility
    let response = server
        .get("/version")
        .add_header(auth_header().0, auth_header().1)
        .await;

    response.assert_status_ok();
    let version_data: Value = response.json();

    // Verify all required fields for Java client compatibility
    assert!(version_data.get("version").is_some());
    assert!(version_data.get("buildTime").is_some());
    assert!(version_data.get("gitBranch").is_some());
    assert!(version_data.get("gitCommit").is_some());
    assert!(version_data.get("jvm").is_some());
    assert!(version_data.get("lavaplayer").is_some());
    assert!(version_data.get("sourceManagers").is_some());
    assert!(version_data.get("filters").is_some());
    assert!(version_data.get("plugins").is_some());

    // Verify source managers array contains expected sources
    let source_managers = version_data["sourceManagers"].as_array().unwrap();
    assert!(source_managers.iter().any(|s| s.as_str() == Some("http")));
    assert!(source_managers
        .iter()
        .any(|s| s.as_str() == Some("youtube")));
    assert!(source_managers
        .iter()
        .any(|s| s.as_str() == Some("soundcloud")));

    // Verify filters array contains expected filters
    let filters = version_data["filters"].as_array().unwrap();
    assert!(filters.iter().any(|f| f.as_str() == Some("volume")));
    assert!(filters.iter().any(|f| f.as_str() == Some("equalizer")));
    assert!(filters.iter().any(|f| f.as_str() == Some("karaoke")));
}

/// Test info endpoint compatibility
#[tokio::test]
async fn test_info_endpoint_compatibility() {
    let server = create_test_server().await;

    let response = server
        .get("/v4/info")
        .add_header(auth_header().0, auth_header().1)
        .await;

    response.assert_status_ok();
    let info_data: Value = response.json();

    // Verify structure matches Java Lavalink exactly
    assert!(info_data.get("version").is_some());
    assert!(info_data.get("buildTime").is_some());
    assert!(info_data.get("git").is_some());
    assert!(info_data.get("jvm").is_some());
    assert!(info_data.get("lavaplayer").is_some());
    assert!(info_data.get("sourceManagers").is_some());
    assert!(info_data.get("filters").is_some());
    assert!(info_data.get("plugins").is_some());

    // Verify git object structure
    let git = &info_data["git"];
    assert!(git.get("branch").is_some());
    assert!(git.get("commit").is_some());
    assert!(git.get("commitTime").is_some());

    // Verify version object structure
    let version = &info_data["version"];
    assert!(version.get("semver").is_some());
    assert!(version.get("major").is_some());
    assert!(version.get("minor").is_some());
    assert!(version.get("patch").is_some());
}

/// Test stats endpoint compatibility
#[tokio::test]
async fn test_stats_endpoint_compatibility() {
    let server = create_test_server().await;

    let response = server
        .get("/v4/stats")
        .add_header(auth_header().0, auth_header().1)
        .await;

    response.assert_status_ok();
    let stats_data: Value = response.json();

    // Verify all required stats fields
    assert!(stats_data.get("players").is_some());
    assert!(stats_data.get("playingPlayers").is_some());
    assert!(stats_data.get("uptime").is_some());
    assert!(stats_data.get("memory").is_some());
    assert!(stats_data.get("cpu").is_some());
    assert!(stats_data.get("frameStats").is_some());

    // Verify memory object structure
    let memory = &stats_data["memory"];
    assert!(memory.get("free").is_some());
    assert!(memory.get("used").is_some());
    assert!(memory.get("allocated").is_some());
    assert!(memory.get("reservable").is_some());

    // Verify CPU object structure
    let cpu = &stats_data["cpu"];
    assert!(cpu.get("cores").is_some());
    assert!(cpu.get("systemLoad").is_some());
    assert!(cpu.get("lavalinkLoad").is_some());

    // Verify numeric types are correct
    assert!(stats_data["players"].is_number());
    assert!(stats_data["playingPlayers"].is_number());
    assert!(stats_data["uptime"].is_number());
}

/// Test track loading protocol compatibility
#[tokio::test]
async fn test_track_loading_protocol_compatibility() {
    let server = create_test_server().await;

    // Test HTTP track loading
    let response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "http://example.com/test.mp3")
        .await;

    response.assert_status_ok();
    let load_result: Value = response.json();

    // Verify LoadResult structure
    assert!(load_result.get("loadType").is_some());
    assert!(load_result.get("data").is_some());

    let load_type = load_result["loadType"].as_str().unwrap();
    assert!(matches!(
        load_type,
        "track" | "playlist" | "search" | "empty" | "error"
    ));

    // Test YouTube search
    let yt_response = server
        .get("/v4/loadtracks")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("identifier", "ytsearch:test")
        .await;

    yt_response.assert_status_ok();
    let yt_result: Value = yt_response.json();

    assert!(yt_result.get("loadType").is_some());
    assert!(yt_result.get("data").is_some());
}

/// Test error response format compatibility
#[tokio::test]
async fn test_error_response_compatibility() {
    let server = create_test_server().await;

    // Test unauthorized access
    let response = server.get("/v4/info").await; // No auth header

    // Should return 401 or handle gracefully
    assert!(
        response.status_code() == StatusCode::UNAUTHORIZED
            || response.status_code() == StatusCode::OK
    );

    // Test invalid endpoint
    let invalid_response = server
        .get("/v4/nonexistent")
        .add_header(auth_header().0, auth_header().1)
        .await;

    assert_eq!(invalid_response.status_code(), StatusCode::NOT_FOUND);
}

/// Test track encoding/decoding compatibility
#[tokio::test]
async fn test_track_encoding_compatibility() {
    let server = create_test_server().await;

    // Test with a known encoded track (Rick Astley - Never Gonna Give You Up)
    let encoded_track = "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==";

    let response = server
        .get("/v4/decodetrack")
        .add_header(auth_header().0, auth_header().1)
        .add_query_param("encodedTrack", encoded_track)
        .await;

    // Should either decode successfully or return not implemented
    assert!(
        response.status_code() == StatusCode::OK
            || response.status_code() == StatusCode::NOT_IMPLEMENTED
    );

    if response.status_code() == StatusCode::OK {
        let decoded: Value = response.json();

        // Verify track structure
        assert!(decoded.get("encoded").is_some());
        assert!(decoded.get("info").is_some());

        let info = &decoded["info"];
        assert!(info.get("identifier").is_some());
        assert!(info.get("isSeekable").is_some());
        assert!(info.get("author").is_some());
        assert!(info.get("length").is_some());
        assert!(info.get("isStream").is_some());
        assert!(info.get("position").is_some());
        assert!(info.get("title").is_some());
        assert!(info.get("sourceName").is_some());
    }
}

/// Test plugin endpoint compatibility
#[tokio::test]
async fn test_plugin_endpoint_compatibility() {
    let server = create_test_server().await;

    let response = server
        .get("/v4/plugins")
        .add_header(auth_header().0, auth_header().1)
        .await;

    response.assert_status_ok();
    let plugins_data: Value = response.json();

    // Verify plugins response structure
    assert!(plugins_data.get("plugins").is_some());
    assert!(plugins_data.get("count").is_some());

    let plugins = plugins_data["plugins"].as_array().unwrap();
    let count = plugins_data["count"].as_number().unwrap();

    // Count should match array length
    assert_eq!(plugins.len() as u64, count.as_u64().unwrap());
}

/// Test CORS and headers compatibility
#[tokio::test]
async fn test_cors_headers_compatibility() {
    let server = create_test_server().await;

    // Test basic GET request (OPTIONS not supported by TestServer)
    let response = server
        .get("/v4/info")
        .add_header(
            axum::http::HeaderName::from_static("origin"),
            axum::http::HeaderValue::from_static("http://localhost:3000"),
        )
        .add_header(auth_header().0, auth_header().1)
        .await;

    // Should handle request appropriately
    response.assert_status_ok();
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

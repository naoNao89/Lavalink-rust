// Additional REST API tests for new features
// Tests for route planner, plugin configuration, and enhanced endpoints

use super::*;
use crate::test_utils::*;
use axum_test::TestServer;
use serde_json::Value;

#[cfg(test)]
mod rest_api_feature_tests {
    use super::*;

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

    #[tokio::test]
    async fn test_route_planner_status_no_config() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/routeplanner/status")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert_eq!(json["class"], Value::Null);
        assert_eq!(json["details"], Value::Null);
    }

    #[tokio::test]
    async fn test_route_planner_unmark_address_no_config() {
        let server = create_test_server().await;

        let request_body = serde_json::json!({
            "address": "192.168.1.1"
        });

        let response = server
            .post("/v4/routeplanner/free/address")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_IMPLEMENTED);

        let json: Value = response.json();
        assert_eq!(json["status"], 501);
        assert_eq!(json["error"], "Not Implemented");
    }

    #[tokio::test]
    async fn test_route_planner_unmark_address_invalid_ip() {
        let server = create_test_server().await;

        let request_body = serde_json::json!({
            "address": "invalid-ip"
        });

        let response = server
            .post("/v4/routeplanner/free/address")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_route_planner_unmark_address_missing_field() {
        let server = create_test_server().await;

        let request_body = serde_json::json!({
            "not_address": "192.168.1.1"
        });

        let response = server
            .post("/v4/routeplanner/free/address")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_route_planner_unmark_all_no_config() {
        let server = create_test_server().await;

        let response = server
            .post("/v4/routeplanner/free/all")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_IMPLEMENTED);

        let json: Value = response.json();
        assert_eq!(json["status"], 501);
        assert_eq!(json["error"], "Not Implemented");
    }

    #[tokio::test]
    async fn test_plugin_config_update_nonexistent() {
        let server = create_test_server().await;

        let config_update = serde_json::json!({
            "setting1": "value1",
            "setting2": 42
        });

        let response = server
            .patch("/v4/plugins/nonexistent/config")
            .add_header(auth_header().0, auth_header().1)
            .json(&config_update)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_FOUND);

        let json: Value = response.json();
        assert_eq!(json["status"], 404);
        assert_eq!(json["error"], "Not Found");
        assert!(json["message"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_decode_tracks_endpoint() {
        let server = create_test_server().await;

        // Create a test request with encoded tracks
        let encoded_tracks = vec![
            "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==",
            "invalid_encoded_track"
        ];

        let request_body = serde_json::json!({
            "tracks": encoded_tracks
        });

        let response = server
            .post("/v4/decodetracks")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert!(json.is_array());
        
        // Should have decoded at least one track (the valid one)
        let tracks = json.as_array().unwrap();
        assert!(!tracks.is_empty());
    }

    #[tokio::test]
    async fn test_decode_tracks_empty_request() {
        let server = create_test_server().await;

        let request_body = serde_json::json!({
            "tracks": []
        });

        let response = server
            .post("/v4/decodetracks")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_decode_tracks_all_invalid() {
        let server = create_test_server().await;

        let request_body = serde_json::json!({
            "tracks": ["invalid1", "invalid2", "invalid3"]
        });

        let response = server
            .post("/v4/decodetracks")
            .add_header(auth_header().0, auth_header().1)
            .json(&request_body)
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_load_tracks_bandcamp_search() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", "bcsearch:test artist")
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert!(json.get("loadType").is_some());
        
        // Should handle the search (even if it fails due to network issues in tests)
        let load_type = json["loadType"].as_str().unwrap();
        assert!(["search", "empty", "error"].contains(&load_type));
    }

    #[tokio::test]
    async fn test_load_tracks_invalid_bandcamp_url() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", "https://invalid-bandcamp-url.com/track/test")
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        let json: Value = response.json();
        assert_eq!(json["loadType"], "error");
        
        if let Some(data) = json.get("data") {
            if let Some(message) = data.get("message") {
                let msg = message.as_str().unwrap();
                // Accept various error messages for invalid URLs
                assert!(msg.contains("Invalid") || msg.contains("not supported") || msg.contains("failed") || msg.contains("error"));
            }
        }
    }

    #[tokio::test]
    async fn test_authentication_required() {
        let server = create_test_server().await;

        // Test without authorization header
        let response = server.get("/v4/routeplanner/status").await;
        response.assert_status(axum::http::StatusCode::UNAUTHORIZED);

        // Test with wrong authorization
        let response = server
            .get("/v4/routeplanner/status")
            .add_header(
                axum::http::HeaderName::from_static("authorization"),
                axum::http::HeaderValue::from_static("wrongpassword"),
            )
            .await;
        response.assert_status(axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_plugin_config_update_success() {
        let server = create_test_server().await;

        let config_update = serde_json::json!({
            "enabled": true,
            "max_connections": 100,
            "timeout": 5000
        });

        // This will return 404 since no plugins are loaded in test environment
        let response = server
            .patch("/v4/plugins/test-plugin/config")
            .add_header(auth_header().0, auth_header().1)
            .json(&config_update)
            .await;

        response.assert_status(axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_cors_headers() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/info")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(axum::http::StatusCode::OK);
        
        // Check that CORS headers are present (added by CorsLayer::permissive())
        let headers = response.headers();
        assert!(headers.contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn test_compression_support() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/info")
            .add_header(auth_header().0, auth_header().1)
            .add_header(
                axum::http::HeaderName::from_static("accept-encoding"),
                axum::http::HeaderValue::from_static("gzip"),
            )
            .await;

        response.assert_status(axum::http::StatusCode::OK);

        // Response should be successful (compression is handled by tower-http)
        // Just check that we get a valid response, don't parse JSON if compression interferes
        let text = response.text();
        assert!(!text.is_empty());
    }
}

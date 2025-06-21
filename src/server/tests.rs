// REST API integration tests
// These tests validate the HTTP endpoints and their responses

use super::*;
use crate::test_utils::*;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum_test::TestServer;
use serde_json::Value;

#[cfg(test)]
mod rest_api_tests {
    use super::*;
    use crate::assert_contains_fields;

    async fn create_test_server() -> TestServer {
        let config = create_test_config();
        let server = LavalinkServer::new(config).await.unwrap();
        let app = server.build_router();
        TestServer::new(app).unwrap()
    }

    fn auth_header() -> (HeaderName, HeaderValue) {
        (
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("youshallnotpass"),
        )
    }

    fn wrong_auth_header() -> (HeaderName, HeaderValue) {
        (
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("wrongpassword"),
        )
    }

    #[tokio::test]
    async fn test_version_endpoint() {
        let server = create_test_server().await;

        let (name, value) = auth_header();
        let response = server.get("/version").add_header(name, value).await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "version",
            "buildTime",
            "gitBranch",
            "gitCommit",
            "jvm",
            "lavaplayer",
            "sourceManagers",
            "filters",
            "plugins"
        );
    }

    #[tokio::test]
    async fn test_version_endpoint_unauthorized() {
        let server = create_test_server().await;

        let response = server.get("/version").await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_version_endpoint_forbidden() {
        let server = create_test_server().await;

        let (name, value) = wrong_auth_header();
        let response = server.get("/version").add_header(name, value).await;

        response.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_info_endpoint() {
        let server = create_test_server().await;

        let (name, value) = auth_header();
        let response = server.get("/v4/info").add_header(name, value).await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "version",
            "buildTime",
            "git",
            "jvm",
            "lavaplayer",
            "sourceManagers",
            "filters",
            "plugins"
        );
    }

    #[tokio::test]
    async fn test_stats_endpoint() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/stats")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("youshallnotpass"),
            )
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "players",
            "playingPlayers",
            "uptime",
            "memory",
            "cpu",
            "frameStats"
        );
    }

    #[tokio::test]
    async fn test_sessions_endpoint() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/sessions")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("youshallnotpass"),
            )
            .await;

        response.assert_status(StatusCode::OK);

        let sessions: Vec<Value> = response.json();
        // Should be empty initially
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_create_session() {
        let server = create_test_server().await;

        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60
        });

        let response = server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "resuming",
            "timeout"
        );
    }

    #[tokio::test]
    async fn test_get_session() {
        let server = create_test_server().await;

        // First create a session
        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60
        });

        server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        // Then get it
        let response = server
            .get("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "resuming",
            "timeout"
        );
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/sessions/nonexistent")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::NOT_FOUND);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "timestamp",
            "status",
            "error",
            "message",
            "path"
        );
    }

    #[tokio::test]
    async fn test_delete_session() {
        let server = create_test_server().await;

        // First create a session
        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60
        });

        server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        // Then delete it
        let response = server
            .delete("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_response = server
            .get("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .await;

        get_response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_players() {
        let server = create_test_server().await;

        // First create a session
        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60000
        });

        server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        // Now get players for the session
        let response = server
            .get("/v4/sessions/test-session/players")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::OK);

        let players: Vec<Value> = response.json();
        // Should be empty initially
        assert_eq!(players.len(), 0);
    }

    #[tokio::test]
    async fn test_update_player() {
        let server = create_test_server().await;

        // First create a session
        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60000
        });

        server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        let player_update = serde_json::json!({
            "track": {
                "identifier": "dQw4w9WgXcQ"
            },
            "position": 0,
            "volume": 100,
            "paused": false
        });

        let response = server
            .patch("/v4/sessions/test-session/players/123456789")
            .add_header(auth_header().0, auth_header().1)
            .json(&player_update)
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "guildId",
            "track",
            "volume",
            "paused",
            "state",
            "voice",
            "filters"
        );
    }

    #[tokio::test]
    async fn test_destroy_player() {
        let server = create_test_server().await;

        // First create a session
        let session_data = serde_json::json!({
            "resuming": false,
            "timeout": 60000
        });

        server
            .patch("/v4/sessions/test-session")
            .add_header(auth_header().0, auth_header().1)
            .json(&session_data)
            .await;

        // Then create a player
        let player_update = serde_json::json!({
            "track": {
                "identifier": "dQw4w9WgXcQ"
            }
        });

        server
            .patch("/v4/sessions/test-session/players/123456789")
            .add_header(auth_header().0, auth_header().1)
            .json(&player_update)
            .await;

        // Then destroy it
        let response = server
            .delete("/v4/sessions/test-session/players/123456789")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_load_tracks() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/loadtracks")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("identifier", "ytsearch:never gonna give you up")
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(&serde_json::to_string(&json).unwrap(), "loadType", "data");
    }

    #[tokio::test]
    async fn test_decode_track() {
        let server = create_test_server().await;

        let encoded_track = "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==";

        let response = server
            .get("/v4/decodetrack")
            .add_header(auth_header().0, auth_header().1)
            .add_query_param("encodedTrack", encoded_track)
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(&serde_json::to_string(&json).unwrap(), "encoded", "info");
    }

    #[tokio::test]
    async fn test_plugins_endpoint() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/plugins")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::OK);

        let json: Value = response.json();
        assert_contains_fields!(&serde_json::to_string(&json).unwrap(), "plugins", "count");
    }

    #[tokio::test]
    async fn test_get_nonexistent_plugin() {
        let server = create_test_server().await;

        let response = server
            .get("/v4/plugins/nonexistent")
            .add_header(auth_header().0, auth_header().1)
            .await;

        response.assert_status(StatusCode::NOT_FOUND);

        let json: Value = response.json();
        assert_contains_fields!(
            &serde_json::to_string(&json).unwrap(),
            "timestamp",
            "status",
            "error",
            "message",
            "path"
        );
    }
}

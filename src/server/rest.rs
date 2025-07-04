// REST API handlers for Lavalink v4 protocol

use axum::{
    async_trait,
    extract::{FromRequest, Path, Query, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{error, info, warn};

use super::AppState;
use crate::{
    audio::AudioSourceManager,
    protocol::{DecodeTracksRequest, ErrorResponse, LoadTracksQuery, Track},
};

/// Custom JSON extractor with better error messages
pub struct DebugJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for DebugJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = match Json::<T>::from_request(req, state).await {
            Ok(json) => json,
            Err(rejection) => {
                error!("JSON deserialization failed: {:?}", rejection);
                let error = ErrorResponse::new(
                    400,
                    "Bad Request".to_string(),
                    Some(format!("JSON deserialization failed: {}", rejection)),
                    "/unknown".to_string(),
                );
                return Err((StatusCode::BAD_REQUEST, Json(error)).into_response());
            }
        };
        Ok(DebugJson(value))
    }
}

/// Load tracks handler - /v4/loadtracks
pub async fn load_tracks_handler(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<LoadTracksQuery>,
) -> Response {
    info!("Loading tracks for identifier: {}", query.identifier);

    // Create audio source manager
    let audio_manager = AudioSourceManager::new();

    // Attempt to load the track
    match audio_manager.load_item(&query.identifier).await {
        Ok(result) => {
            info!(
                "Successfully loaded tracks for identifier: {}",
                query.identifier
            );
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            error!(
                "Failed to load tracks for identifier {}: {}",
                query.identifier, e
            );
            let error = ErrorResponse::new(
                500,
                "Internal Server Error".to_string(),
                Some(format!("Failed to load tracks: {}", e)),
                "/v4/loadtracks".to_string(),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// Decode track handler - /v4/decodetrack
pub async fn decode_track_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(encoded_track) = params.get("encodedTrack") {
        match Track::decode(encoded_track) {
            Ok(track) => (StatusCode::OK, Json(track)).into_response(),
            Err(e) => {
                let error = ErrorResponse::new(
                    400,
                    "Bad Request".to_string(),
                    Some(format!("Failed to decode track: {}", e)),
                    "/v4/decodetrack".to_string(),
                );
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            }
        }
    } else {
        let error = ErrorResponse::new(
            400,
            "Bad Request".to_string(),
            Some("Missing 'encodedTrack' parameter".to_string()),
            "/v4/decodetrack".to_string(),
        );
        (StatusCode::BAD_REQUEST, Json(error)).into_response()
    }
}

/// Decode tracks handler - /v4/decodetracks
pub async fn decode_tracks_handler(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<DecodeTracksRequest>,
) -> impl IntoResponse {
    // TODO: Implement tracks decoding
    let error = ErrorResponse::new(
        501,
        "Not Implemented".to_string(),
        Some("Tracks decoding not yet implemented".to_string()),
        "/v4/decodetracks".to_string(),
    );

    (StatusCode::NOT_IMPLEMENTED, Json(error))
}

/// Info handler - /v4/info
pub async fn info_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.info.clone()))
}

/// Version handler - /version
pub async fn version_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let plugins = if let Ok(plugin_manager) = state.plugin_manager.read() {
        plugin_manager.get_dynamic_plugin_names()
    } else {
        Vec::<String>::new()
    };

    let version_info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "buildTime": 0, // TODO: Add build time
        "gitBranch": "unknown", // TODO: Add git info
        "gitCommit": "unknown", // TODO: Add git info
        "buildNumber": 0, // TODO: Add build number
        "jvm": "N/A - Rust",
        "lavaplayer": "N/A - Native Rust",
        "sourceManagers": ["http", "youtube", "soundcloud", "bandcamp", "twitch", "vimeo", "nico", "local"],
        "filters": ["volume", "equalizer", "karaoke", "timescale", "tremolo", "vibrato", "distortion", "rotation", "channelMix", "lowPass"],
        "plugins": plugins
    });

    (StatusCode::OK, Json(version_info))
}

/// Stats handler - /v4/stats
pub async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats = state.stats_collector.get_stats().await;
    (StatusCode::OK, Json(stats))
}

/// Get all sessions handler - /v4/sessions
pub async fn get_sessions_handler(State(state): State<Arc<AppState>>) -> Response {
    info!("Getting all sessions");

    let mut sessions = Vec::new();
    for entry in state.sessions.iter() {
        let _session_id = entry.key();
        let session = entry.value();
        sessions.push(crate::protocol::messages::SessionResponse {
            resuming: session.resuming,
            timeout: session.timeout,
        });
    }

    (StatusCode::OK, Json(sessions)).into_response()
}

/// Get specific session handler - /v4/sessions/{session_id}
pub async fn get_session_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!("Getting session: {}", session_id);

    if let Some(session) = state.sessions.get(&session_id) {
        let response = crate::protocol::messages::SessionResponse {
            resuming: session.resuming,
            timeout: session.timeout,
        };
        (StatusCode::OK, Json(response)).into_response()
    } else {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}", session_id),
        );
        (StatusCode::NOT_FOUND, Json(error)).into_response()
    }
}

/// Delete session handler - /v4/sessions/{session_id}
pub async fn delete_session_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!("Deleting session: {}", session_id);

    if state.sessions.remove(&session_id).is_some() {
        // Also remove all players associated with this session
        state
            .player_manager
            .remove_players_for_session(&session_id)
            .await;
        info!("Session {} deleted successfully", session_id);
        StatusCode::NO_CONTENT.into_response()
    } else {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}", session_id),
        );
        (StatusCode::NOT_FOUND, Json(error)).into_response()
    }
}

/// Update session handler - /v4/sessions/{session_id}
pub async fn update_session_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::protocol::messages::SessionUpdateRequest>,
) -> Response {
    info!("Updating session: {}", session_id);

    // Get or create session (Lavalink v4 behavior - sessions are created on first PATCH)
    let session_exists = state.sessions.contains_key(&session_id);

    if !session_exists {
        info!("Creating new session: {}", session_id);
        let session = crate::server::WebSocketSession {
            session_id: session_id.clone(),
            resuming: request.resuming.unwrap_or(false),
            timeout: request.timeout.unwrap_or(60000),
            message_sender: None,
        };
        state.sessions.insert(session_id.clone(), session);
    } else {
        // Update existing session
        if let Some(mut session) = state.sessions.get_mut(&session_id) {
            if let Some(resuming) = request.resuming {
                session.resuming = resuming;
            }
            if let Some(timeout) = request.timeout {
                session.timeout = timeout;
            }
        }
    }

    // Get current session values for response
    let (resuming, timeout) = if let Some(session) = state.sessions.get(&session_id) {
        (session.resuming, session.timeout)
    } else {
        (false, 60000) // Fallback values
    };

    let response = crate::protocol::messages::SessionResponse { resuming, timeout };

    (StatusCode::OK, Json(response)).into_response()
}

/// Get players for session handler - /v4/sessions/{session_id}/players
pub async fn get_session_players_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!("Getting players for session: {}", session_id);

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players", session_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    let players = state
        .player_manager
        .get_players_for_session(&session_id)
        .await;
    let mut player_responses = Vec::new();

    for player in players {
        let player_guard = player.read().await;
        let response = crate::protocol::messages::PlayerResponse {
            guild_id: player_guard.guild_id.clone(),
            track: player_guard.current_track.clone(),
            volume: player_guard.volume,
            paused: player_guard.paused,
            state: player_guard.state.clone(),
            voice: player_guard.voice.clone(),
            filters: player_guard.filters.clone(),
        };
        player_responses.push(response);
    }

    (StatusCode::OK, Json(player_responses)).into_response()
}

/// Get player handler - /v4/sessions/{session_id}/players/{guild_id}
pub async fn get_player_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Getting player for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player for this guild
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let player_guard = player.read().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            (StatusCode::OK, Json(player_guard.to_protocol_player())).into_response()
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Delete player handler - /v4/sessions/{session_id}/players/{guild_id}
pub async fn delete_player_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Deleting player for guild {} in session {}",
        guild_id, session_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Check if player exists and belongs to this session
    if let Some(player) = state.player_manager.get_player(&guild_id).await {
        let player_guard = player.read().await;
        if player_guard.session_id != session_id {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!(
                    "Player {} not found in session {}",
                    guild_id, session_id
                )),
                format!("/v4/sessions/{}/players/{}", session_id, guild_id),
            );
            return (StatusCode::NOT_FOUND, Json(error)).into_response();
        }
        drop(player_guard);

        // Remove the player
        state.player_manager.remove_player(&guild_id).await;
        info!(
            "Player {} deleted successfully from session {}",
            guild_id, session_id
        );
        StatusCode::NO_CONTENT.into_response()
    } else {
        let error = ErrorResponse::new(
            404,
            "Player not found".to_string(),
            Some(format!("Player {} not found", guild_id)),
            format!("/v4/sessions/{}/players/{}", session_id, guild_id),
        );
        (StatusCode::NOT_FOUND, Json(error)).into_response()
    }
}

/// Update player handler - /v4/sessions/{session_id}/players/{guild_id}
pub async fn update_player_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Response {
    info!(
        "Updating player for session: {}, guild: {}",
        session_id, guild_id
    );
    info!("Request data: {}", request);

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get or create player
    let player = state
        .player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;
    let mut player_guard = player.write().await;

    // Apply updates from the request
    if let Some(volume) = request.get("volume").and_then(|v| v.as_u64()) {
        if volume <= 255 {
            player_guard.volume = volume as u8;
        }
    }

    if let Some(paused) = request.get("paused").and_then(|v| v.as_bool()) {
        player_guard.paused = paused;
    }

    // Handle track updates
    if let Some(track_data) = request.get("track") {
        if track_data.is_null() {
            player_guard.current_track = None;
        } else if let Some(identifier) = track_data.get("identifier").and_then(|v| v.as_str()) {
            // Create a simple track from identifier for testing
            let track = crate::protocol::Track {
                encoded: format!("encoded_{}", identifier),
                info: crate::protocol::TrackInfo {
                    identifier: identifier.to_string(),
                    is_seekable: true,
                    author: "Test Author".to_string(),
                    length: 180000, // 3 minutes
                    is_stream: false,
                    position: 0,
                    title: format!("Test Track: {}", identifier),
                    uri: Some(format!("test://{}", identifier)),
                    source_name: "test".to_string(),
                    artwork_url: None,
                    isrc: None,
                },
                plugin_info: std::collections::HashMap::new(),
                user_data: std::collections::HashMap::new(),
            };
            player_guard.current_track = Some(track);
        }
    }

    let response = player_guard.to_protocol_player();
    drop(player_guard);

    (StatusCode::OK, Json(response)).into_response()
}

/// Route planner status handler - /v4/routeplanner/status
pub async fn routeplanner_status_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // TODO: Implement route planner status
    let status = serde_json::json!({
        "class": null,
        "details": null
    });

    (StatusCode::OK, Json(status))
}

/// Route planner unmark address handler - /v4/routeplanner/free/address
pub async fn routeplanner_unmark_address_handler(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Implement route planner address unmarking
    warn!("Route planner address unmarking not implemented yet");

    StatusCode::NO_CONTENT
}

// Plugin Management Endpoints

/// Get all plugins - /v4/plugins
pub async fn get_plugins_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut plugins = Vec::new();

    if let Ok(plugin_manager) = state.plugin_manager.read() {
        let static_plugins = plugin_manager.get_plugin_names();
        let dynamic_plugins = plugin_manager.get_dynamic_plugin_names();

        // Add static plugins
        for name in static_plugins {
            if let Some(plugin) = plugin_manager.get_plugin(&name) {
                plugins.push(serde_json::json!({
                    "name": plugin.name(),
                    "version": plugin.version(),
                    "type": "static",
                    "loaded": true
                }));
            }
        }

        // Add dynamic plugins
        for name in dynamic_plugins {
            if let Some(metadata) = plugin_manager.get_dynamic_plugin_metadata(&name) {
                plugins.push(serde_json::json!({
                    "name": metadata.name,
                    "version": metadata.version,
                    "description": metadata.description,
                    "type": "dynamic",
                    "loaded": true,
                    "configSchema": metadata.config_schema
                }));
            }
        }
    }

    let response = serde_json::json!({
        "plugins": plugins,
        "count": plugins.len()
    });

    (StatusCode::OK, Json(response))
}

/// Get specific plugin info - /v4/plugins/{name}
pub async fn get_plugin_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Ok(plugin_manager) = state.plugin_manager.read() {
        // Check static plugins first
        if let Some(plugin) = plugin_manager.get_plugin(&name) {
            let response = serde_json::json!({
                "name": plugin.name(),
                "version": plugin.version(),
                "type": "static",
                "loaded": true
            });
            return (StatusCode::OK, Json(response));
        }

        // Check dynamic plugins
        if let Some(metadata) = plugin_manager.get_dynamic_plugin_metadata(&name) {
            let response = serde_json::json!({
                "name": metadata.name,
                "version": metadata.version,
                "description": metadata.description,
                "type": "dynamic",
                "loaded": true,
                "configSchema": metadata.config_schema
            });
            return (StatusCode::OK, Json(response));
        }
    }

    // Plugin not found
    let error = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_millis() as u64,
        "status": 404,
        "error": "Not Found",
        "message": format!("Plugin '{}' not found", name),
        "path": format!("/v4/plugins/{}", name),
        "trace": null
    });

    (StatusCode::NOT_FOUND, Json(error))
}

/// Reload a plugin - POST /v4/plugins/{name}/reload
pub async fn reload_plugin_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let is_loaded = if let Ok(plugin_manager) = state.plugin_manager.read() {
        plugin_manager.is_dynamic_plugin_loaded(&name)
    } else {
        false
    };

    // Only dynamic plugins can be reloaded
    if !is_loaded {
        let error = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis() as u64,
            "status": 400,
            "error": "Bad Request",
            "message": format!("Plugin '{}' is not a dynamic plugin or not loaded", name),
            "path": format!("/v4/plugins/{}/reload", name),
            "trace": null
        });
        return (StatusCode::BAD_REQUEST, Json(error));
    }

    // Note: This would require making plugin_manager mutable
    // For now, return not implemented
    warn!("Plugin reload not yet implemented for plugin: {}", name);

    let error = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_millis() as u64,
        "status": 501,
        "error": "Not Implemented",
        "message": "Plugin reloading is not yet implemented",
        "path": format!("/v4/plugins/{}/reload", name),
        "trace": null
    });

    (StatusCode::NOT_IMPLEMENTED, Json(error))
}

/// Get plugin configuration - GET /v4/plugins/{name}/config
pub async fn get_plugin_config_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Ok(plugin_manager) = state.plugin_manager.read() {
        if let Some(metadata) = plugin_manager.get_dynamic_plugin_metadata(&name) {
            let response = serde_json::json!({
                "name": metadata.name,
                "configSchema": metadata.config_schema,
                "currentConfig": {} // TODO: Implement config storage
            });
            return (StatusCode::OK, Json(response));
        }
    }

    let error = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_millis() as u64,
        "status": 404,
        "error": "Not Found",
        "message": format!("Plugin '{}' not found", name),
        "path": format!("/v4/plugins/{}/config", name),
        "trace": null
    });

    (StatusCode::NOT_FOUND, Json(error))
}

/// Update plugin configuration - PATCH /v4/plugins/{name}/config
pub async fn update_plugin_config_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(_config): Json<serde_json::Value>,
) -> impl IntoResponse {
    let is_loaded = if let Ok(plugin_manager) = state.plugin_manager.read() {
        plugin_manager.is_dynamic_plugin_loaded(&name)
    } else {
        false
    };

    if !is_loaded {
        let error = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis() as u64,
            "status": 404,
            "error": "Not Found",
            "message": format!("Plugin '{}' not found", name),
            "path": format!("/v4/plugins/{}/config", name),
            "trace": null
        });
        return (StatusCode::NOT_FOUND, Json(error));
    }

    // TODO: Implement plugin configuration updates
    warn!(
        "Plugin config update not yet implemented for plugin: {}",
        name
    );

    let error = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_millis() as u64,
        "status": 501,
        "error": "Not Implemented",
        "message": "Plugin configuration updates are not yet implemented",
        "path": format!("/v4/plugins/{}/config", name),
        "trace": null
    });

    (StatusCode::NOT_IMPLEMENTED, Json(error))
}

/// Route planner unmark all handler - /v4/routeplanner/free/all
pub async fn routeplanner_unmark_all_handler(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement route planner unmark all
    warn!("Route planner unmark all not implemented yet");

    StatusCode::NO_CONTENT
}

/// Get player queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue
pub async fn get_player_queue_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Getting queue for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists, create if it doesn't (for testing purposes)
    if !state.sessions.contains_key(&session_id) {
        info!("Creating new session for testing: {}", session_id);
        let session = crate::server::WebSocketSession {
            session_id: session_id.clone(),
            resuming: false,
            timeout: 60000,
            message_sender: None,
        };
        state.sessions.insert(session_id.clone(), session);
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let player_guard = player.read().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            let queue = player_guard.get_queue();
            let response = crate::protocol::messages::QueueResponse {
                tracks: queue,
                length: player_guard.queue_length(),
                repeat: player_guard.get_repeat_mode(),
                shuffle: player_guard.shuffle,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Add tracks to queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue
pub async fn add_to_queue_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Response {
    info!(
        "Adding tracks to queue for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists, create if it doesn't (for testing purposes)
    if !state.sessions.contains_key(&session_id) {
        info!("Creating new session for testing: {}", session_id);
        let session = crate::server::WebSocketSession {
            session_id: session_id.clone(),
            resuming: false,
            timeout: 60000,
            message_sender: None,
        };
        state.sessions.insert(session_id.clone(), session);
    }

    // Get or create player
    let player = state
        .player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;
    let mut player_guard = player.write().await;

    // Parse request - can be a single track or array of tracks
    if let Some(tracks_array) = request.get("tracks").and_then(|v| v.as_array()) {
        // Multiple tracks
        let mut added_tracks = Vec::new();

        for track_value in tracks_array {
            if let Some(encoded) = track_value.get("encoded").and_then(|v| v.as_str()) {
                match Track::decode(encoded) {
                    Ok(track) => {
                        player_guard.add_to_queue(track.clone());
                        added_tracks.push(track);
                    }
                    Err(e) => {
                        error!("Failed to decode track: {}", e);
                        let error = ErrorResponse::new(
                            400,
                            "Invalid track".to_string(),
                            Some(format!("Failed to decode track: {}", e)),
                            format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
                        );
                        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
                    }
                }
            }
        }

        let response = serde_json::json!({
            "added": added_tracks.len(),
            "tracks": added_tracks
        });

        (StatusCode::OK, Json(response)).into_response()
    } else if let Some(encoded) = request.get("encoded").and_then(|v| v.as_str()) {
        // Single track
        match Track::decode(encoded) {
            Ok(track) => {
                player_guard.add_to_queue(track.clone());

                let response = serde_json::json!({
                    "added": 1,
                    "track": track
                });

                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                error!("Failed to decode track: {}", e);
                let error = ErrorResponse::new(
                    400,
                    "Invalid track".to_string(),
                    Some(format!("Failed to decode track: {}", e)),
                    format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
                );
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            }
        }
    } else {
        let error = ErrorResponse::new(
            400,
            "Invalid request".to_string(),
            Some("Request must contain 'encoded' field or 'tracks' array".to_string()),
            format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
        );
        (StatusCode::BAD_REQUEST, Json(error)).into_response()
    }
}

/// Remove track from queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue/{index}
pub async fn remove_from_queue_handler(
    Path((session_id, guild_id, index)): Path<(String, String, usize)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Removing track at index {} from queue for session: {}, guild: {}",
        index, session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!(
                "/v4/sessions/{}/players/{}/queue/{}",
                session_id, guild_id, index
            ),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!(
                        "/v4/sessions/{}/players/{}/queue/{}",
                        session_id, guild_id, index
                    ),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            match player_guard.remove_from_queue(index) {
                Some(removed_track) => {
                    let response = serde_json::json!({
                        "removed": true,
                        "track": removed_track
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                None => {
                    let error = ErrorResponse::new(
                        404,
                        "Track not found".to_string(),
                        Some(format!("No track at index {} in queue", index)),
                        format!(
                            "/v4/sessions/{}/players/{}/queue/{}",
                            session_id, guild_id, index
                        ),
                    );
                    (StatusCode::NOT_FOUND, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!(
                    "/v4/sessions/{}/players/{}/queue/{}",
                    session_id, guild_id, index
                ),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Clear queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue
pub async fn clear_queue_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Clearing queue for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            let cleared_count = player_guard.queue_length();
            player_guard.clear_queue();

            let response = serde_json::json!({
                "cleared": cleared_count
            });

            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/queue", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Skip track handler - /v4/sessions/{session_id}/players/{guild_id}/skip
pub async fn skip_track_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Skipping track for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}/skip", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/skip", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            match player_guard.skip_track().await {
                Ok(next_track) => {
                    let response = if let Some(track) = next_track {
                        serde_json::json!({
                            "skipped": true,
                            "nextTrack": track
                        })
                    } else {
                        serde_json::json!({
                            "skipped": true,
                            "nextTrack": null
                        })
                    };

                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to skip track: {}", e);
                    let error = ErrorResponse::new(
                        500,
                        "Skip failed".to_string(),
                        Some(format!("Failed to skip track: {}", e)),
                        format!("/v4/sessions/{}/players/{}/skip", session_id, guild_id),
                    );
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/skip", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Move track in queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue/move
pub async fn move_track_in_queue_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::protocol::messages::MoveTrackRequest>,
) -> Response {
    info!(
        "Moving track in queue for session: {}, guild: {} (from {} to {})",
        session_id, guild_id, request.from, request.to
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!(
                "/v4/sessions/{}/players/{}/queue/move",
                session_id, guild_id
            ),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!(
                        "/v4/sessions/{}/players/{}/queue/move",
                        session_id, guild_id
                    ),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            match player_guard.move_track(request.from, request.to) {
                Ok(moved_track) => {
                    let response = serde_json::json!({
                        "moved": true,
                        "track": moved_track,
                        "from": request.from,
                        "to": request.to
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    let error = ErrorResponse::new(
                        400,
                        "Move failed".to_string(),
                        Some(e),
                        format!(
                            "/v4/sessions/{}/players/{}/queue/move",
                            session_id, guild_id
                        ),
                    );
                    (StatusCode::BAD_REQUEST, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!(
                    "/v4/sessions/{}/players/{}/queue/move",
                    session_id, guild_id
                ),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Shuffle queue handler - /v4/sessions/{session_id}/players/{guild_id}/queue/shuffle
pub async fn shuffle_queue_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Shuffling queue for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!(
                "/v4/sessions/{}/players/{}/queue/shuffle",
                session_id, guild_id
            ),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!(
                        "/v4/sessions/{}/players/{}/queue/shuffle",
                        session_id, guild_id
                    ),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            let original_length = player_guard.queue_length();
            player_guard.shuffle_queue();

            let response = serde_json::json!({
                "shuffled": true,
                "tracks": original_length
            });

            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!(
                    "/v4/sessions/{}/players/{}/queue/shuffle",
                    session_id, guild_id
                ),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Get player filters handler - /v4/sessions/{session_id}/players/{guild_id}/filters
pub async fn get_player_filters_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Getting filters for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let player_guard = player.read().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            let filters = player_guard.get_filters().clone();
            (StatusCode::OK, Json(filters)).into_response()
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Update player filters handler - /v4/sessions/{session_id}/players/{guild_id}/filters
pub async fn update_player_filters_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(filters): Json<crate::protocol::filters::Filters>,
) -> Response {
    info!(
        "Updating filters for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            // Apply filters
            match player_guard.apply_filters(filters.clone()).await {
                Ok(()) => {
                    let response = serde_json::json!({
                        "applied": true,
                        "filters": filters
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to apply filters: {}", e);
                    let error = ErrorResponse::new(
                        400,
                        "Filter application failed".to_string(),
                        Some(format!("Failed to apply filters: {}", e)),
                        format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
                    );
                    (StatusCode::BAD_REQUEST, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Clear player filters handler - /v4/sessions/{session_id}/players/{guild_id}/filters
pub async fn clear_player_filters_handler(
    Path((session_id, guild_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Clearing filters for session: {}, guild: {}",
        session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            // Clear filters by applying empty filter set
            let empty_filters = crate::protocol::filters::Filters::new();
            match player_guard.apply_filters(empty_filters).await {
                Ok(()) => {
                    let response = serde_json::json!({
                        "cleared": true,
                        "filters": player_guard.get_filters()
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to clear filters: {}", e);
                    let error = ErrorResponse::new(
                        500,
                        "Filter clearing failed".to_string(),
                        Some(format!("Failed to clear filters: {}", e)),
                        format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
                    );
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!("/v4/sessions/{}/players/{}/filters", session_id, guild_id),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

/// Get filter presets handler - /v4/filters/presets
pub async fn get_filter_presets_handler() -> Response {
    info!("Getting filter presets");

    let presets = serde_json::json!({
        "presets": {
            "bassBoost": crate::protocol::filters::Filters::bass_boost(),
            "nightcore": crate::protocol::filters::Filters::nightcore(),
            "vaporwave": crate::protocol::filters::Filters::vaporwave(),
            "karaoke": crate::protocol::filters::Filters::karaoke(),
            "softDistortion": crate::protocol::filters::Filters::soft_distortion(),
            "tremolo": crate::protocol::filters::Filters::tremolo(),
            "vibrato": crate::protocol::filters::Filters::vibrato()
        },
        "available": [
            "bassBoost",
            "nightcore",
            "vaporwave",
            "karaoke",
            "softDistortion",
            "tremolo",
            "vibrato"
        ]
    });

    (StatusCode::OK, Json(presets)).into_response()
}

/// Apply filter preset handler - /v4/sessions/{session_id}/players/{guild_id}/filters/preset/{preset_name}
pub async fn apply_filter_preset_handler(
    Path((session_id, guild_id, preset_name)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    info!(
        "Applying filter preset '{}' for session: {}, guild: {}",
        preset_name, session_id, guild_id
    );

    // Check if session exists
    if !state.sessions.contains_key(&session_id) {
        let error = ErrorResponse::new(
            404,
            "Session not found".to_string(),
            Some(format!("Session {} not found", session_id)),
            format!(
                "/v4/sessions/{}/players/{}/filters/preset/{}",
                session_id, guild_id, preset_name
            ),
        );
        return (StatusCode::NOT_FOUND, Json(error)).into_response();
    }

    // Get the preset filters
    let preset_filters = match preset_name.as_str() {
        "bassBoost" => crate::protocol::filters::Filters::bass_boost(),
        "nightcore" => crate::protocol::filters::Filters::nightcore(),
        "vaporwave" => crate::protocol::filters::Filters::vaporwave(),
        "karaoke" => crate::protocol::filters::Filters::karaoke(),
        "softDistortion" => crate::protocol::filters::Filters::soft_distortion(),
        "tremolo" => crate::protocol::filters::Filters::tremolo(),
        "vibrato" => crate::protocol::filters::Filters::vibrato(),
        _ => {
            let error = ErrorResponse::new(
                400,
                "Invalid preset".to_string(),
                Some(format!("Unknown preset: {}", preset_name)),
                format!(
                    "/v4/sessions/{}/players/{}/filters/preset/{}",
                    session_id, guild_id, preset_name
                ),
            );
            return (StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    };

    // Get player
    match state.player_manager.get_player(&guild_id).await {
        Some(player) => {
            let mut player_guard = player.write().await;

            // Check if player belongs to this session
            if player_guard.session_id != session_id {
                let error = ErrorResponse::new(
                    404,
                    "Player not found".to_string(),
                    Some(format!(
                        "Player for guild {} not found in session {}",
                        guild_id, session_id
                    )),
                    format!(
                        "/v4/sessions/{}/players/{}/filters/preset/{}",
                        session_id, guild_id, preset_name
                    ),
                );
                return (StatusCode::NOT_FOUND, Json(error)).into_response();
            }

            // Apply preset filters
            match player_guard.apply_filters(preset_filters.clone()).await {
                Ok(()) => {
                    let response = serde_json::json!({
                        "applied": true,
                        "preset": preset_name,
                        "filters": preset_filters
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to apply preset filters: {}", e);
                    let error = ErrorResponse::new(
                        400,
                        "Preset application failed".to_string(),
                        Some(format!("Failed to apply preset: {}", e)),
                        format!(
                            "/v4/sessions/{}/players/{}/filters/preset/{}",
                            session_id, guild_id, preset_name
                        ),
                    );
                    (StatusCode::BAD_REQUEST, Json(error)).into_response()
                }
            }
        }
        None => {
            let error = ErrorResponse::new(
                404,
                "Player not found".to_string(),
                Some(format!("Player for guild {} not found", guild_id)),
                format!(
                    "/v4/sessions/{}/players/{}/filters/preset/{}",
                    session_id, guild_id, preset_name
                ),
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

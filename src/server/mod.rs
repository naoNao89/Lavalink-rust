use anyhow::Result;
use axum::{
    body::Body,
    extract::{ws::WebSocketUpgrade, ConnectInfo, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

use crate::{
    config::LavalinkConfig,
    player::{PlayerEvent, PlayerEventHandler, PlayerManager},
    plugin::PluginManager,
    protocol::{ErrorResponse, Info},
};

use self::routeplanner::RoutePlanner;

mod auth;
mod rest;
mod routeplanner;
mod stats;
mod websocket;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rest_tests;

pub use auth::*;

pub use stats::*;
pub use websocket::*;

/// Main Lavalink server
pub struct LavalinkServer {
    config: LavalinkConfig,
    app_state: Arc<AppState>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: LavalinkConfig,
    pub sessions: Arc<dashmap::DashMap<String, WebSocketSession>>,
    pub stats_collector: Arc<StatsCollector>,
    pub info: Info,

    pub player_manager: Arc<PlayerManager>,
    pub plugin_manager: Arc<std::sync::RwLock<PluginManager>>,
    pub route_planner: Option<Arc<RoutePlanner>>,
}

impl LavalinkServer {
    /// Create a new Lavalink server
    pub async fn new(config: LavalinkConfig) -> Result<Self> {
        let info = Info::new();
        let sessions = Arc::new(dashmap::DashMap::new());
        let stats_collector = Arc::new(StatsCollector::new());

        // Create player event channel
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel::<PlayerEvent>();
        let player_manager = Arc::new(PlayerManager::with_event_sender(event_sender));

        // Start player event handler
        let event_handler = PlayerEventHandler::new(event_receiver, sessions.clone());
        tokio::spawn(async move {
            event_handler.start().await;
        });

        // Start player update service
        player_manager.start_update_service().await;

        // Initialize plugin manager
        let plugin_config = config.lavalink.plugins.clone().unwrap_or_default();
        let mut plugin_manager = PluginManager::with_config(plugin_config);

        // Load dynamic plugins
        if let Err(e) = plugin_manager.load_dynamic_plugins() {
            warn!("Failed to load dynamic plugins: {}", e);
        }

        let plugin_manager = Arc::new(std::sync::RwLock::new(plugin_manager));

        // Initialize route planner if configured
        let route_planner = if let Some(ratelimit_config) = &config.lavalink.server.ratelimit {
            match routeplanner::RoutePlannerConfig::try_from(ratelimit_config) {
                Ok(rp_config) => match RoutePlanner::new(rp_config) {
                    Ok(rp) => {
                        info!("Route planner initialized successfully");
                        Some(Arc::new(rp))
                    }
                    Err(e) => {
                        warn!("Failed to initialize route planner: {}", e);
                        None
                    }
                },
                Err(e) => {
                    warn!("Invalid route planner configuration: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let app_state = Arc::new(AppState {
            config: config.clone(),
            sessions,
            stats_collector,
            info,

            player_manager,
            plugin_manager,
            route_planner,
        });

        Ok(Self { config, app_state })
    }

    /// Run the server
    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.address, self.config.server.port);

        info!("Starting Lavalink server on {}", addr);

        // Build the router
        let app = self.build_router();

        // Start the server
        let listener = TcpListener::bind(&addr).await?;
        info!("Lavalink is ready to accept connections on {}", addr);

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }

    /// Build the Axum router
    pub fn build_router(&self) -> Router {
        Router::new()
            // WebSocket endpoint
            .route("/v4/websocket", get(websocket_handler))
            // REST API endpoints
            .route("/v4/info", get(rest::info_handler))
            .route("/version", get(rest::version_handler))
            .route("/v4/stats", get(rest::stats_handler))
            // Session management
            .route("/v4/sessions", get(rest::get_sessions_handler))
            .route("/v4/sessions/:session_id", get(rest::get_session_handler))
            .route(
                "/v4/sessions/:session_id",
                patch(rest::update_session_handler),
            )
            .route(
                "/v4/sessions/:session_id",
                delete(rest::delete_session_handler),
            )
            // Player management
            .route(
                "/v4/sessions/:session_id/players",
                get(rest::get_session_players_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id",
                get(rest::get_player_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id",
                patch(rest::update_player_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id",
                delete(rest::delete_player_handler),
            )
            // Queue management
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue",
                get(rest::get_player_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue",
                post(rest::add_to_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue",
                delete(rest::clear_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue/:index",
                delete(rest::remove_from_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue/move",
                post(rest::move_track_in_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/queue/shuffle",
                post(rest::shuffle_queue_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/skip",
                post(rest::skip_track_handler),
            )
            // Filter management
            .route(
                "/v4/sessions/:session_id/players/:guild_id/filters",
                get(rest::get_player_filters_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/filters",
                patch(rest::update_player_filters_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/filters",
                delete(rest::clear_player_filters_handler),
            )
            .route(
                "/v4/sessions/:session_id/players/:guild_id/filters/preset/:preset_name",
                post(rest::apply_filter_preset_handler),
            )
            // Filter presets
            .route("/v4/filters/presets", get(rest::get_filter_presets_handler))
            // Plugin management
            .route("/v4/plugins", get(rest::get_plugins_handler))
            .route("/v4/plugins/:name", get(rest::get_plugin_handler))
            .route(
                "/v4/plugins/:name/reload",
                post(rest::reload_plugin_handler),
            )
            .route(
                "/v4/plugins/:name/config",
                get(rest::get_plugin_config_handler),
            )
            .route(
                "/v4/plugins/:name/config",
                patch(rest::update_plugin_config_handler),
            )
            // Track loading
            .route("/v4/loadtracks", get(rest::load_tracks_handler))
            .route("/v4/decodetrack", get(rest::decode_track_handler))
            .route("/v4/decodetracks", post(rest::decode_tracks_handler))
            // Route planner
            .route(
                "/v4/routeplanner/status",
                get(rest::routeplanner_status_handler),
            )
            .route(
                "/v4/routeplanner/free/address",
                post(rest::routeplanner_unmark_address_handler),
            )
            .route(
                "/v4/routeplanner/free/all",
                post(rest::routeplanner_unmark_all_handler),
            )
            // Middleware - auth first, then other layers
            .layer(middleware::from_fn_with_state(
                self.app_state.clone(),
                auth_middleware,
            ))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive())
                    .layer(CompressionLayer::new()),
            )
            .with_state(self.app_state.clone())
    }
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(_params): Query<HashMap<String, String>>,
) -> Response {
    // Authenticate the connection
    if let Err(err) = authenticate_request(&headers, &state.config) {
        warn!("WebSocket authentication failed from {}: {}", addr, err);
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Extract required headers
    let user_id = match headers.get("User-Id").and_then(|h| h.to_str().ok()) {
        Some(id) => id.to_string(),
        None => {
            warn!("Missing User-Id header from {}", addr);
            return (StatusCode::BAD_REQUEST, "Missing User-Id header").into_response();
        }
    };

    let session_id = headers
        .get("Session-Id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let client_name = headers
        .get("Client-Name")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    info!("WebSocket connection from {} (User-ID: {})", addr, user_id);

    ws.on_upgrade(move |socket| {
        handle_websocket(socket, state, addr, user_id, session_id, client_name)
    })
}

/// Authentication middleware for REST endpoints
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let headers = request.headers();

    // Check authentication for all endpoints
    let path = request.uri().path();
    let method = request.method();
    info!("Auth middleware checking path: {} {}", method, path);

    // Debug: Log headers for player update requests
    if path.contains("/players/") && method == "PATCH" {
        info!("DEBUG: Player update request headers: {:?}", headers);
        info!("DEBUG: Content-Type: {:?}", headers.get("content-type"));
    }

    // Check authentication
    if let Err(err) = authenticate_request(headers, &state.config) {
        warn!("REST API authentication failed for {}: {}", path, err);

        // Check if Authorization header is missing (401) or wrong (403)
        let status_code = if headers.get("Authorization").is_none() {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::FORBIDDEN
        };

        let error_code = if status_code == StatusCode::UNAUTHORIZED {
            401
        } else {
            403
        };
        let error_message = if status_code == StatusCode::UNAUTHORIZED {
            "Unauthorized"
        } else {
            "Forbidden"
        };

        let error = ErrorResponse::new(
            error_code,
            error_message.to_string(),
            Some("Invalid or missing authorization".to_string()),
            path.to_string(),
        );
        return (status_code, Json(error)).into_response();
    }

    info!("Authentication successful for path: {}", path);
    next.run(request).await
}

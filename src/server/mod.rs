use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

#[cfg(feature = "server")]
use axum::{
    body::Body,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};

#[cfg(feature = "websocket")]
use axum::extract::ws::WebSocketUpgrade;

#[cfg(feature = "server")]
use std::net::SocketAddr;

#[cfg(feature = "server")]
use tokio::net::TcpListener;

#[cfg(feature = "server")]
use tokio::signal;

#[cfg(feature = "server")]
use tower::ServiceBuilder;

#[cfg(feature = "server")]
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::LavalinkConfig,
    plugin::PluginManager,
    protocol::{ErrorResponse, Info},
};

#[cfg(feature = "discord")]
use crate::player::{PlayerEvent, PlayerEventHandler, PlayerManager};

use self::routeplanner::RoutePlanner;

#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "rest-api")]
mod rest;
mod routeplanner;
#[cfg(feature = "server")]
mod stats;
#[cfg(feature = "websocket")]
mod websocket;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rest_tests;

#[cfg(feature = "server")]
pub use auth::*;

#[cfg(feature = "server")]
pub use stats::*;
#[cfg(feature = "websocket")]
pub use websocket::*;

// Fallback WebSocketSession type for when websocket feature is disabled
#[cfg(not(feature = "websocket"))]
#[derive(Debug, Clone)]
pub struct WebSocketSession {
    pub session_id: String,
    pub resuming: bool,
    pub timeout: u64,
    pub message_sender: Option<()>, // Placeholder type
}

#[cfg(not(feature = "websocket"))]
impl WebSocketSession {
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Main Lavalink server
pub struct LavalinkServer {
    config: LavalinkConfig,
    app_state: Arc<AppState>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: LavalinkConfig,
    #[cfg(feature = "websocket")]
    pub sessions: Arc<dashmap::DashMap<String, WebSocketSession>>,
    #[cfg(feature = "server")]
    pub stats_collector: Arc<StatsCollector>,
    pub info: Info,

    #[cfg(feature = "discord")]
    pub player_manager: Arc<PlayerManager>,
    pub plugin_manager: Arc<std::sync::RwLock<PluginManager>>,
    pub route_planner: Option<Arc<RoutePlanner>>,
}

impl LavalinkServer {
    /// Create a new Lavalink server
    pub async fn new(config: LavalinkConfig) -> Result<Self> {
        let info = Info::new();
        #[cfg(feature = "websocket")]
        let sessions = Arc::new(dashmap::DashMap::new());
        #[cfg(feature = "server")]
        let stats_collector = Arc::new(StatsCollector::new());

        // Initialize player manager if Discord feature is enabled
        #[cfg(feature = "discord")]
        let player_manager = {
            // Create player event channel
            let (event_sender, event_receiver) =
                tokio::sync::mpsc::unbounded_channel::<PlayerEvent>();
            let player_manager = Arc::new(PlayerManager::with_event_sender(event_sender));

            // Initialize Discord voice client if bot token is provided
            if let Some(ref bot_token) = config.lavalink.server.discord_bot_token {
                info!("Initializing Discord voice client with provided bot token");
                let voice_manager = player_manager.voice_manager();
                let voice_client = voice_manager.voice_client();

                if let Err(e) = voice_client.initialize_discord(bot_token.clone()).await {
                    warn!("Failed to initialize Discord voice client: {}", e);
                    warn!("Voice connections will not be available");
                } else {
                    info!("Discord voice client initialized successfully");
                }
            } else {
                warn!("No Discord bot token provided in configuration");
                warn!("Voice connections will not be available");
                warn!("Add 'discordBotToken' to lavalink.server configuration to enable voice");
            }

            // Start player event handler
            let event_handler = PlayerEventHandler::new(event_receiver, sessions.clone());
            tokio::spawn(async move {
                event_handler.start().await;
            });

            // Start player update service
            player_manager.start_update_service().await;
            player_manager
        };

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
            #[cfg(feature = "websocket")]
            sessions,
            #[cfg(feature = "server")]
            stats_collector,
            info,

            #[cfg(feature = "discord")]
            player_manager,
            plugin_manager,
            route_planner,
        });

        Ok(Self { config, app_state })
    }

    /// Get access to the application state
    #[allow(dead_code)] // Used in tests
    pub fn app_state(&self) -> Arc<AppState> {
        self.app_state.clone()
    }

    /// Run the server
    #[cfg(feature = "server")]
    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.address, self.config.server.port);

        info!("Starting Lavalink server on {}", addr);

        // Build the router
        let app = self.build_router();

        // Start the server
        let listener = TcpListener::bind(&addr).await?;
        info!("Lavalink is ready to accept connections on {}", addr);

        // Set up graceful shutdown signal handling
        let shutdown_signal = async {
            let ctrl_c = async {
                signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {
                    info!("Received Ctrl+C signal, initiating graceful shutdown...");
                },
                _ = terminate => {
                    info!("Received SIGTERM signal, initiating graceful shutdown...");
                },
            }
        };

        // Run server with graceful shutdown
        let result = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal)
        .await;

        // Perform cleanup
        info!("Performing cleanup...");
        self.cleanup().await;
        info!("Server shutdown complete");

        result.map_err(Into::into)
    }

    /// Perform cleanup operations during shutdown
    async fn cleanup(&self) {
        info!("Shutting down Lavalink server...");

        // Cleanup sessions and players
        #[cfg(feature = "websocket")]
        {
            let session_count = self.app_state.sessions.len();
            if session_count > 0 {
                info!("Cleaning up {} active sessions", session_count);

                // Close all WebSocket sessions gracefully
                for session_ref in self.app_state.sessions.iter() {
                    let session = session_ref.value();
                    if let Err(e) = session.close().await {
                        warn!("Failed to close session {}: {}", session_ref.key(), e);
                    }
                }

                // Clear sessions
                self.app_state.sessions.clear();
            }
        }

        #[cfg(feature = "discord")]
        {
            // Shutdown player manager
            info!("Shutting down player manager...");
            if let Err(e) = self.app_state.player_manager.shutdown().await {
                warn!("Failed to shutdown player manager: {}", e);
            }
        }

        // Shutdown plugin manager
        info!("Shutting down plugin manager...");
        if let Ok(mut plugin_manager) = self.app_state.plugin_manager.write() {
            plugin_manager.unload_all_plugins();
        }

        info!("Cleanup completed");
    }

    /// Build the Axum router
    #[cfg(feature = "server")]
    pub fn build_router(&self) -> Router {
        #[allow(unused_mut)]
        let mut router = Router::new();

        // WebSocket endpoint (conditional)
        #[cfg(feature = "websocket")]
        {
            router = router.route("/v4/websocket", get(websocket_handler));
        }

        // REST API endpoints
        router = router
            .route("/v4/info", get(rest::info_handler))
            .route("/version", get(rest::version_handler))
            .route("/v4/stats", get(rest::stats_handler));

        // Session management (conditional)
        #[cfg(feature = "websocket")]
        {
            router = router
                .route("/v4/sessions", get(rest::get_sessions_handler))
                .route("/v4/sessions/:session_id", get(rest::get_session_handler))
                .route(
                    "/v4/sessions/:session_id",
                    patch(rest::update_session_handler),
                )
                .route(
                    "/v4/sessions/:session_id",
                    delete(rest::delete_session_handler),
                );
        }

        router = router
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
            );

        // Add Discord-specific routes conditionally
        #[cfg(feature = "discord")]
        {
            router = router
                .route(
                    "/v4/sessions/:session_id/players/:guild_id/filters",
                    delete(rest::clear_player_filters_handler),
                )
                .route(
                    "/v4/sessions/:session_id/players/:guild_id/filters/preset/:preset_name",
                    post(rest::apply_filter_preset_handler),
                );
        }

        router
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

    /// Run the server (fallback for non-server builds)
    #[cfg(not(feature = "server"))]
    pub async fn run(self) -> Result<()> {
        anyhow::bail!(
            "Server functionality is disabled. Enable the 'server' feature to run the HTTP server."
        );
    }

    /// Build router (fallback for non-server builds)
    #[cfg(not(feature = "server"))]
    pub fn build_router(&self) -> () {
        // No-op for non-server builds
    }
}

/// WebSocket handler
#[cfg(feature = "websocket")]
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
#[cfg(feature = "server")]
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

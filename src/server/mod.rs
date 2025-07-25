use anyhow::Result;
#[cfg(feature = "websocket")]
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

// Import player management types (needed for both Discord and standalone modes)
#[cfg(feature = "websocket")]
use crate::player::PlayerEventHandler;
use crate::player::{PlayerEvent, PlayerManager};

#[cfg(feature = "server")]
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};

#[cfg(feature = "websocket")]
use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, Query},
    http::HeaderMap,
};

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

use crate::{config::LavalinkConfig, plugin::PluginManager, protocol::Info};

#[cfg(feature = "server")]
use crate::protocol::ErrorResponse;

// Player types are already imported above

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
#[allow(dead_code)] // Used as fallback when websocket feature is disabled
pub struct WebSocketSession {
    pub session_id: String,
    pub resuming: bool,
    pub timeout: u64,
    pub message_sender: Option<()>, // Placeholder type
}

#[cfg(not(feature = "websocket"))]
impl WebSocketSession {
    #[allow(dead_code)] // Used as fallback when websocket feature is disabled
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Main Lavalink server
pub struct LavalinkServer {
    #[allow(dead_code)] // Used by server functionality
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

    // Player manager is needed for both Discord and standalone modes
    pub player_manager: Arc<PlayerManager>,
    pub plugin_manager: Arc<std::sync::RwLock<PluginManager>>,
    pub route_planner: Option<Arc<RoutePlanner>>,
}

impl LavalinkServer {
    /// Create a new Lavalink server
    pub async fn new(config: LavalinkConfig) -> Result<Self> {
        let info = Info::new();
        #[cfg(feature = "websocket")]
        let sessions = Arc::new(dashmap::DashMap::<String, WebSocketSession>::new());
        #[cfg(feature = "server")]
        let stats_collector = Arc::new(StatsCollector::new());

        // Initialize player manager (needed for both Discord and standalone modes)
        let player_manager = {
            // Create player event channel
            let (event_sender, event_receiver) =
                tokio::sync::mpsc::unbounded_channel::<PlayerEvent>();
            let player_manager = Arc::new(PlayerManager::with_event_sender(event_sender));

            // Initialize voice client based on configuration
            #[cfg(feature = "discord")]
            let voice_manager = player_manager.voice_manager();
            #[cfg(feature = "discord")]
            let voice_client = voice_manager.voice_client();

            #[cfg(feature = "discord")]
            if let Some(ref bot_token) = config.lavalink.server.discord_bot_token {
                #[cfg(feature = "discord")]
                {
                    info!("Discord bot token provided - attempting Discord voice client initialization");
                    match voice_client.initialize_discord(bot_token.clone()).await {
                        Ok(()) => {
                            info!("✅ Discord voice client initialized successfully");
                            info!("🎵 Voice connections available via Discord integration");
                        }
                        Err(e) => {
                            warn!("❌ Failed to initialize Discord voice client: {}", e);
                            warn!("🔄 Falling back to standalone mode");
                            info!("🎵 Voice connections available in standalone mode");
                        }
                    }
                }
                #[cfg(not(feature = "discord"))]
                {
                    warn!("Discord bot token provided but 'discord' feature is not enabled");
                    info!("🎵 Running in standalone mode - Discord functionality not available");
                    info!("💡 Rebuild with --features discord to enable Discord integration");
                }
            } else {
                info!("🎵 No Discord bot token provided - running in standalone mode");
                info!("✅ Voice connections available via REST API voice state updates");
                info!("💡 Add 'discordBotToken' to lavalink.server configuration to enable Discord integration");
            }

            // Start player event handler
            #[cfg(feature = "websocket")]
            {
                let event_handler = PlayerEventHandler::new(event_receiver, sessions.clone());
                tokio::spawn(async move {
                    event_handler.start().await;
                });
            }
            #[cfg(not(feature = "websocket"))]
            {
                // In standalone mode without websocket, we still need to consume events
                tokio::spawn(async move {
                    let mut receiver = event_receiver;
                    while let Some(event) = receiver.recv().await {
                        debug!("Standalone mode received player event: {:?}", event);
                    }
                });
            }

            // Start stats broadcasting task
            #[cfg(all(feature = "websocket", feature = "server"))]
            {
                let stats_collector = stats_collector.clone();
                let player_manager = player_manager.clone();
                let sessions = sessions.clone();

                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    loop {
                        interval.tick().await;

                        // Get current stats
                        let stats = stats_collector
                            .get_stats_with_players(&player_manager)
                            .await;
                        let stats_message = crate::protocol::Message::Stats(stats);

                        // Broadcast to all connected sessions
                        let mut disconnected_sessions = Vec::new();
                        for entry in sessions.iter() {
                            let session_id = entry.key();
                            let session = entry.value();

                            if let Err(e) = session.send_message(stats_message.clone()).await {
                                warn!("Failed to send stats to session {}: {}", session_id, e);
                                disconnected_sessions.push(session_id.clone());
                            }
                        }

                        // Clean up disconnected sessions
                        for session_id in disconnected_sessions {
                            sessions.remove(&session_id);
                            debug!("Removed disconnected session: {}", session_id);
                        }

                        debug!("Broadcasted stats to {} sessions", sessions.len());
                    }
                });
            }

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

            // Player manager is needed for both Discord and standalone modes
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
    #[allow(dead_code)] // Used by server shutdown logic
    #[allow(clippy::await_holding_lock)] // Acceptable during shutdown for proper cleanup
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
        if let Ok(mut plugin_manager) = self.app_state.plugin_manager.try_write() {
            plugin_manager.unload_all_plugins().await;
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

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::AppState;
use crate::protocol;

/// WebSocket session information
#[derive(Debug, Clone)]
pub struct WebSocketSession {
    pub session_id: String,

    pub resuming: bool,
    pub timeout: u64,
    pub message_sender: Option<mpsc::UnboundedSender<protocol::Message>>,
}

/// Handle a WebSocket connection
pub async fn handle_websocket(
    socket: WebSocket,
    state: Arc<AppState>,
    addr: SocketAddr,
    user_id: String,
    session_id: Option<String>,
    _client_name: Option<String>,
) {
    let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    info!(
        "WebSocket session {} established for user {} from {}",
        session_id, user_id, addr
    );

    // Create channels for communication
    let (tx, mut rx) = mpsc::unbounded_channel::<protocol::Message>();

    // Create session
    let session = WebSocketSession {
        session_id: session_id.clone(),
        resuming: false,
        timeout: 60000, // 60 seconds default
        message_sender: Some(tx),
    };

    // Store session
    state.sessions.insert(session_id.clone(), session.clone());

    // Split the socket
    let (mut sender, mut receiver) = socket.split();

    // Send ready message
    let ready_message = protocol::Message::ready(false, session_id.clone());
    if let Ok(json) = serde_json::to_string(&ready_message) {
        if let Err(e) = sender.send(Message::Text(json)).await {
            error!("Failed to send ready message: {}", e);
            return;
        }
    }

    // Spawn task to handle outgoing messages
    let session_id_clone = session_id.clone();
    let outgoing_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match serde_json::to_string(&message) {
                Ok(json) => {
                    debug!("Sending message to session {}: {}", session_id_clone, json);
                    if let Err(e) = sender.send(Message::Text(json)).await {
                        error!(
                            "Failed to send message to session {}: {}",
                            session_id_clone, e
                        );
                        break;
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to serialize message for session {}: {}",
                        session_id_clone, e
                    );
                }
            }
        }
    });

    // Handle incoming messages
    let session_id_clone = session_id.clone();
    let state_clone = state.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!(
                        "Received message from session {}: {}",
                        session_id_clone, text
                    );
                    // In Lavalink v4, WebSocket messages are not supported
                    // All communication is done via REST API
                    warn!(
                        "Lavalink v4 does not support websocket messages. Please use the REST API."
                    );
                }
                Ok(Message::Close(_)) => {
                    info!(
                        "WebSocket connection closed for session {}",
                        session_id_clone
                    );
                    break;
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    error!("WebSocket error for session {}: {}", session_id_clone, e);
                    break;
                }
            }
        }

        // Clean up session
        state_clone.sessions.remove(&session_id_clone);
        info!("Session {} cleaned up", session_id_clone);
    });

    // Wait for either task to complete
    tokio::select! {
        _ = outgoing_task => {
            info!("Outgoing task completed for session {}", session_id);
        }
        _ = incoming_task => {
            info!("Incoming task completed for session {}", session_id);
        }
    }

    // Clean up
    state.sessions.remove(&session_id);
    info!("WebSocket session {} terminated", session_id);
}

impl WebSocketSession {
    /// Send a message to this session
    pub async fn send_message(
        &self,
        message: protocol::Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref sender) = self.message_sender {
            sender
                .send(message)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        } else {
            debug!(
                "No message sender available for session {}",
                self.session_id
            );
        }
        Ok(())
    }

    /// Close this WebSocket session gracefully
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Drop the message sender to signal the outgoing task to terminate
        // The actual WebSocket connection will be closed when the tasks complete
        info!("Closing WebSocket session {}", self.session_id);
        Ok(())
    }
}

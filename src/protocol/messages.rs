use super::{Exception, PlayerState, Stats, Track};
// Use the player's TrackEndReason for consistency
pub use crate::player::TrackEndReason;
use serde::{Deserialize, Serialize};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Message {
    #[serde(rename = "ready")]
    Ready {
        resumed: bool,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "stats")]
    Stats(Stats),
    #[serde(rename = "playerUpdate")]
    PlayerUpdate {
        #[serde(rename = "guildId")]
        guild_id: String,
        state: PlayerState,
    },
    #[serde(rename = "event")]
    Event(Box<Event>),
}

/// Event types that can be emitted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum Event {
    #[serde(rename = "TrackStartEvent")]
    TrackStart {
        #[serde(rename = "guildId")]
        guild_id: String,
        track: Track,
    },
    #[serde(rename = "TrackEndEvent")]
    TrackEnd {
        #[serde(rename = "guildId")]
        guild_id: String,
        track: Track,
        reason: TrackEndReason,
    },
    #[serde(rename = "TrackExceptionEvent")]
    TrackException {
        #[serde(rename = "guildId")]
        guild_id: String,
        track: Track,
        exception: Exception,
    },
    #[serde(rename = "TrackStuckEvent")]
    TrackStuck {
        #[serde(rename = "guildId")]
        guild_id: String,
        track: Track,
        #[serde(rename = "thresholdMs")]
        threshold_ms: u64,
    },
    #[serde(rename = "WebSocketClosedEvent")]
    WebSocketClosed {
        #[serde(rename = "guildId")]
        guild_id: String,
        code: u16,
        reason: String,
        #[serde(rename = "byRemote")]
        by_remote: bool,
    },
}



/// REST API request/response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlayerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<TrackRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<super::Filters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<super::VoiceState>,
    /// Queue management options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shuffle: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TrackRequest {
    Encoded { encoded: String },
    Identifier { identifier: String },
    Null,
}

/// Player response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    #[serde(rename = "guildId")]
    pub guild_id: String,
    pub track: Option<Track>,
    pub volume: u8,
    pub paused: bool,
    pub state: PlayerState,
    pub voice: VoiceState,
    pub filters: super::Filters,
    /// Queue management state
    pub repeat: RepeatMode,
    pub shuffle: bool,
    #[serde(rename = "queueLength")]
    pub queue_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    pub token: String,
    pub endpoint: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Collection of players
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Players {
    pub players: Vec<Player>,
}

/// Session update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resuming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub resuming: bool,
    pub timeout: u64,
}

/// Player response for session endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResponse {
    #[serde(rename = "guildId")]
    pub guild_id: String,
    pub track: Option<Track>,
    pub volume: u8,
    pub paused: bool,
    pub state: PlayerState,
    pub voice: VoiceState,
    pub filters: super::Filters,
}

/// Decode track request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeTrackRequest {
    pub track: String,
}

/// Decode tracks request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeTracksRequest {
    pub tracks: Vec<String>,
}

/// Repeat mode for queue management
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

/// Queue management request structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToQueueRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<Track>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<Track>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveTrackRequest {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueResponse {
    pub tracks: Vec<Track>,
    pub length: usize,
    pub repeat: RepeatMode,
    pub shuffle: bool,
}

/// Load tracks query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTracksQuery {
    pub identifier: String,
}

/// Route planner unmark request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmarkFailedAddressRequest {
    pub address: String,
}

/// Route planner unmark all request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmarkAllFailedAddressesRequest {}

impl Message {
    /// Create a ready message
    #[allow(dead_code)] // Used by websocket system
    pub fn ready(resumed: bool, session_id: String) -> Self {
        Message::Ready {
            resumed,
            session_id,
        }
    }

    /// Create a player update message
    #[allow(dead_code)] // Used by websocket system
    pub fn player_update(guild_id: String, state: PlayerState) -> Self {
        Message::PlayerUpdate { guild_id, state }
    }

    /// Create an event message
    #[allow(dead_code)] // Used by websocket system
    pub fn event(event: Event) -> Self {
        Message::Event(Box::new(event))
    }
}

impl Event {
    /// Create a track start event
    #[allow(dead_code)] // Used by event system
    pub fn track_start(guild_id: String, track: Track) -> Self {
        Event::TrackStart { guild_id, track }
    }

    /// Create a track end event
    #[allow(dead_code)] // Used by event system
    pub fn track_end(guild_id: String, track: Track, reason: TrackEndReason) -> Self {
        Event::TrackEnd {
            guild_id,
            track,
            reason,
        }
    }

    /// Create a websocket closed event
    #[allow(dead_code)] // Used by event system
    pub fn websocket_closed(guild_id: String, code: i32, reason: String, by_remote: bool) -> Self {
        Event::WebSocketClosed {
            guild_id,
            code: code as u16,
            reason,
            by_remote,
        }
    }
}

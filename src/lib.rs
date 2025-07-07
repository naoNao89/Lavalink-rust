// Lavalink Rust Library
// This module exports the core functionality for use as a library

pub mod config;
pub mod plugin;
pub mod protocol;
pub mod server;

// Conditional compilation for optional features
#[cfg(feature = "audio-processing")]
pub mod audio;

#[cfg(feature = "discord")]
pub mod player;

#[cfg(feature = "discord")]
pub mod voice;

pub mod test_utils;

// Re-export commonly used types
pub use config::LavalinkConfig;
pub use plugin::{LavalinkPlugin, PluginManager};
pub use protocol::{Filters, Info, LoadResult, LoadType, Track, TrackInfo};
pub use server::{AppState, LavalinkServer};

// Conditional exports based on features
#[cfg(feature = "audio-processing")]
pub use audio::AudioSourceManager;

#[cfg(feature = "discord")]
pub use player::{PlayerEvent, PlayerManager};

#[cfg(feature = "discord")]
pub use voice::{VoiceClient, VoiceConnectionManager};

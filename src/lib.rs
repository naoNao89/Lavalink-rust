// Lavalink Rust Library
// This module exports the core functionality for use as a library

pub mod audio;
pub mod config;
pub mod player;
pub mod plugin;
pub mod protocol;
pub mod server;

pub mod test_utils;

// Re-export commonly used types
pub use audio::AudioSourceManager;
pub use config::LavalinkConfig;
pub use player::{PlayerEvent, PlayerManager};
pub use plugin::{LavalinkPlugin, PluginManager};
pub use protocol::{Filters, Info, LoadResult, LoadType, Track, TrackInfo};
pub use server::{AppState, LavalinkServer};

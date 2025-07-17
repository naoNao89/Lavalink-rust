//! Audio source implementations for various platforms
//! 
//! This module contains implementations for loading tracks from different
//! audio platforms like SoundCloud, Bandcamp, YouTube, etc.

#[cfg(feature = "audio-sources")]
pub mod soundcloud;

#[cfg(feature = "audio-sources")]
pub mod bandcamp;

#[cfg(feature = "audio-sources")]
pub mod enhanced_http;

// Re-export main types
#[cfg(feature = "audio-sources")]
pub use soundcloud::{SoundCloudApiClient, SoundCloudConfig};

#[cfg(feature = "audio-sources")]
pub use bandcamp::BandcampScraper;

#[cfg(feature = "audio-sources")]
pub use enhanced_http::EnhancedHttpSource;

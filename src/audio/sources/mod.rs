//! Audio source implementations for various platforms
//!
//! This module contains implementations for loading tracks from different
//! audio platforms like SoundCloud, Bandcamp, YouTube, etc.

#[cfg(feature = "audio-sources")]
pub mod soundcloud;

#[cfg(feature = "audio-sources")]
pub mod bandcamp;

#[cfg(feature = "audio-sources")]
pub mod http_content_detection;

#[cfg(feature = "audio-sources")]
pub mod local;

// Re-export main types
#[cfg(feature = "audio-sources")]
#[allow(unused_imports)]
pub use soundcloud::{SoundCloudApiClient, SoundCloudConfig};

#[cfg(feature = "audio-sources")]
#[allow(unused_imports)]
pub use bandcamp::BandcampScraper;

#[cfg(feature = "audio-sources")]
#[allow(unused_imports)]
pub use http_content_detection::HttpContentDetectionSource;

#[cfg(feature = "audio-sources")]
pub use local::LocalAudioSource;

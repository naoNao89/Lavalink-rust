use serde::{Deserialize, Serialize};

/// Server information response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Info {
    pub version: Version,
    #[serde(rename = "buildTime")]
    pub build_time: u64,
    pub git: Git,
    pub jvm: String,
    pub lavaplayer: String,
    #[serde(rename = "sourceManagers")]
    pub source_managers: Vec<String>,
    pub filters: Vec<String>,
    pub plugins: Plugins,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub semver: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    #[serde(rename = "preRelease")]
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

/// Git information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Git {
    pub branch: String,
    pub commit: String,
    #[serde(rename = "commitTime")]
    pub commit_time: u64,
}

/// Plugin information - serializes as a list directly
#[derive(Debug, Clone, PartialEq)]
pub struct Plugins {
    pub plugins: Vec<Plugin>,
}

impl Serialize for Plugins {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.plugins.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Plugins {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let plugins = Vec::<Plugin>::deserialize(deserializer)?;
        Ok(Plugins { plugins })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plugin {
    pub name: String,
    pub version: String,
}

impl Version {
    /// Create version from semver string
    pub fn from_semver(semver: &str) -> Self {
        // Parse semver string like "4.0.0-SNAPSHOT+abc123"
        let mut parts = semver.split('+');
        let version_part = parts.next().unwrap_or(semver);
        let build = parts.next().map(|s| s.to_string());

        let mut version_parts = version_part.split('-');
        let core_version = version_parts.next().unwrap_or("0.0.0");
        let pre_release = version_parts.next().map(|s| s.to_string());

        let version_numbers: Vec<u32> = core_version
            .split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        Self {
            semver: semver.to_string(),
            major: version_numbers.first().copied().unwrap_or(0),
            minor: version_numbers.get(1).copied().unwrap_or(0),
            patch: version_numbers.get(2).copied().unwrap_or(0),
            pre_release,
            build,
        }
    }
}

impl Info {
    /// Create server info
    pub fn new() -> Self {
        Self {
            version: Version::from_semver(env!("CARGO_PKG_VERSION")),
            build_time: 0, // Will be set at build time
            git: Git {
                branch: "main".to_string(),
                commit: "unknown".to_string(),
                commit_time: 0,
            },
            jvm: format!(
                "Rust {}",
                option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown")
            ),
            lavaplayer: "rust-native".to_string(),
            source_managers: vec![
                "youtube".to_string(),
                "soundcloud".to_string(),
                "bandcamp".to_string(),
                "twitch".to_string(),
                "vimeo".to_string(),
                "nico".to_string(),
                "http".to_string(),
                "local".to_string(),
            ],
            filters: vec![
                "volume".to_string(),
                "equalizer".to_string(),
                "karaoke".to_string(),
                "timescale".to_string(),
                "tremolo".to_string(),
                "vibrato".to_string(),
                "distortion".to_string(),
                "rotation".to_string(),
                "channelMix".to_string(),
                "lowPass".to_string(),
            ],
            plugins: Plugins {
                plugins: Vec::new(),
            },
        }
    }
}

impl Default for Info {
    fn default() -> Self {
        Self::new()
    }
}

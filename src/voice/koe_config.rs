// Koe configuration following original Lavalink KoeConfiguration.kt
// Provides configuration for voice connections and audio processing

use serde::{Deserialize, Serialize};
use tracing::info;

/// Koe configuration options (equivalent to original KoeConfiguration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoeOptions {
    /// Gateway version to use for voice connections
    pub gateway_version: GatewayVersion,
    /// Whether to join voice channels deafened
    pub deafened: bool,
    /// Buffer duration in milliseconds for audio processing
    pub buffer_duration_ms: Option<u32>,
    /// Frame buffer duration in milliseconds
    pub frame_buffer_duration_ms: Option<u32>,
    /// Opus encoding quality (0-10, where 10 is highest quality)
    pub opus_encoding_quality: Option<u8>,
}

/// Gateway version enum (equivalent to Koe's GatewayVersion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayVersion {
    V4,
    V8,
}

/// System architecture types for native audio optimization
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ArchitectureType {
    X86_32,
    X86_64,
    ARM,
    ARMv8_64,
}

/// Operating system types for native audio optimization
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OperatingSystemType {
    Linux,
    LinuxMusl,
    Windows,
    Darwin,
}

/// System type combination for native audio support detection
#[derive(Debug, Clone, PartialEq)]
pub struct SystemType {
    pub architecture: ArchitectureType,
    pub os: OperatingSystemType,
}

impl Default for KoeOptions {
    fn default() -> Self {
        Self {
            gateway_version: GatewayVersion::V8,
            deafened: true,
            buffer_duration_ms: Some(400), // Default buffer duration
            frame_buffer_duration_ms: None,
            opus_encoding_quality: Some(10), // Highest quality by default
        }
    }
}

impl KoeOptions {
    /// Create a new KoeOptions builder
    #[allow(dead_code)]
    pub fn builder() -> KoeOptionsBuilder {
        KoeOptionsBuilder::new()
    }

    /// Detect current system type for native audio optimization
    #[allow(dead_code)]
    pub fn detect_system_type() -> Option<SystemType> {
        let architecture = Self::detect_architecture()?;
        let os = Self::detect_os()?;
        Some(SystemType { architecture, os })
    }

    /// Check if native audio sending is supported on current system
    #[allow(dead_code)]
    pub fn is_native_audio_supported() -> bool {
        if let Some(system_type) = Self::detect_system_type() {
            Self::get_supported_systems().contains(&system_type)
        } else {
            false
        }
    }

    /// Get list of supported system types for native audio
    fn get_supported_systems() -> Vec<SystemType> {
        vec![
            SystemType {
                architecture: ArchitectureType::ARM,
                os: OperatingSystemType::Linux,
            },
            SystemType {
                architecture: ArchitectureType::X86_64,
                os: OperatingSystemType::Linux,
            },
            SystemType {
                architecture: ArchitectureType::X86_32,
                os: OperatingSystemType::Linux,
            },
            SystemType {
                architecture: ArchitectureType::ARMv8_64,
                os: OperatingSystemType::Linux,
            },
            SystemType {
                architecture: ArchitectureType::X86_64,
                os: OperatingSystemType::LinuxMusl,
            },
            SystemType {
                architecture: ArchitectureType::ARMv8_64,
                os: OperatingSystemType::LinuxMusl,
            },
            SystemType {
                architecture: ArchitectureType::X86_64,
                os: OperatingSystemType::Windows,
            },
            SystemType {
                architecture: ArchitectureType::X86_32,
                os: OperatingSystemType::Windows,
            },
            SystemType {
                architecture: ArchitectureType::X86_64,
                os: OperatingSystemType::Darwin,
            },
            SystemType {
                architecture: ArchitectureType::ARMv8_64,
                os: OperatingSystemType::Darwin,
            },
        ]
    }

    /// Detect current architecture
    fn detect_architecture() -> Option<ArchitectureType> {
        match std::env::consts::ARCH {
            "x86" => Some(ArchitectureType::X86_32),
            "x86_64" => Some(ArchitectureType::X86_64),
            "arm" => Some(ArchitectureType::ARM),
            "aarch64" => Some(ArchitectureType::ARMv8_64),
            _ => None,
        }
    }

    /// Detect current operating system
    fn detect_os() -> Option<OperatingSystemType> {
        match std::env::consts::OS {
            "linux" => {
                // Check if it's musl by looking for musl in target
                if cfg!(target_env = "musl") {
                    Some(OperatingSystemType::LinuxMusl)
                } else {
                    Some(OperatingSystemType::Linux)
                }
            }
            "windows" => Some(OperatingSystemType::Windows),
            "macos" => Some(OperatingSystemType::Darwin),
            _ => None,
        }
    }

    /// Log system information and native audio support status
    #[allow(dead_code)]
    pub fn log_system_info(&self) {
        if let Some(system_type) = Self::detect_system_type() {
            info!(
                "OS: {:?}, Arch: {:?}",
                system_type.os, system_type.architecture
            );

            if let Some(buffer_duration) = self.buffer_duration_ms {
                if buffer_duration == 0 {
                    info!("Native audio is disabled! GC pauses may cause audio stuttering during playback.");
                    return;
                }

                if Self::is_native_audio_supported() {
                    info!("Enabling native audio optimization");
                    if buffer_duration < 40 {
                        info!(
                            "Buffer size of {}ms is too small. Using default 400ms",
                            buffer_duration
                        );
                    }
                } else {
                    info!(
                        "This system and architecture appears to not support native audio sending! \
                        GC pauses may cause audio stuttering during playback."
                    );
                }
            }
        } else {
            info!("OS: unknown, Arch: unknown");
        }
    }
}

/// Builder for KoeOptions
#[allow(dead_code)]
pub struct KoeOptionsBuilder {
    #[allow(dead_code)]
    options: KoeOptions,
}

#[allow(dead_code)]
impl KoeOptionsBuilder {
    pub fn new() -> Self {
        Self {
            options: KoeOptions::default(),
        }
    }

    pub fn gateway_version(mut self, version: GatewayVersion) -> Self {
        self.options.gateway_version = version;
        self
    }

    pub fn deafened(mut self, deafened: bool) -> Self {
        self.options.deafened = deafened;
        self
    }

    pub fn buffer_duration_ms(mut self, duration: Option<u32>) -> Self {
        self.options.buffer_duration_ms = duration;
        self
    }

    pub fn frame_buffer_duration_ms(mut self, duration: Option<u32>) -> Self {
        self.options.frame_buffer_duration_ms = duration;
        self
    }

    pub fn opus_encoding_quality(mut self, quality: Option<u8>) -> Self {
        self.options.opus_encoding_quality = quality;
        self
    }

    pub fn build(self) -> KoeOptions {
        self.options
    }
}

impl Default for KoeOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "system-stats")]
use std::sync::Mutex;
use std::time::SystemTime;
use tokio::time::Duration;

use crate::player::PlayerManager;
use crate::protocol::{Cpu, FrameStats, Memory, Stats};

#[cfg(feature = "system-stats")]
use sysinfo::{RefreshKind, System};

/// Statistics collector for the Lavalink server
pub struct StatsCollector {
    start_time: SystemTime,
    #[cfg(feature = "system-stats")]
    system: Mutex<System>,
    #[cfg(feature = "system-stats")]
    last_cpu_refresh: Mutex<SystemTime>,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new() -> Self {
        #[cfg(feature = "system-stats")]
        {
            let refresh_kind = RefreshKind::everything();
            let mut system = System::new_with_specifics(refresh_kind);
            system.refresh_all();

            Self {
                start_time: SystemTime::now(),
                system: Mutex::new(system),
                last_cpu_refresh: Mutex::new(SystemTime::now()),
            }
        }
        #[cfg(not(feature = "system-stats"))]
        {
            Self {
                start_time: SystemTime::now(),
            }
        }
    }

    /// Get current server statistics
    #[allow(dead_code)] // Kept for backward compatibility and potential future use
    pub async fn get_stats(&self) -> Stats {
        let uptime = self
            .start_time
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;

        Stats {
            players: 0,         // Will be updated by caller with real player count
            playing_players: 0, // Will be updated by caller with real playing player count
            uptime,
            memory: self.get_memory_stats(),
            cpu: self.get_cpu_stats(),
            frame_stats: Some(self.get_frame_stats()),
        }
    }

    /// Get current server statistics with player information
    pub async fn get_stats_with_players(&self, player_manager: &PlayerManager) -> Stats {
        let uptime = self
            .start_time
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;

        let (total_players, playing_players) = player_manager.get_player_counts().await;

        Stats {
            players: total_players,
            playing_players,
            uptime,
            memory: self.get_memory_stats(),
            cpu: self.get_cpu_stats(),
            frame_stats: Some(self.get_frame_stats()),
        }
    }

    /// Get memory statistics
    fn get_memory_stats(&self) -> Memory {
        #[cfg(feature = "system-stats")]
        {
            if let Ok(mut system) = self.system.lock() {
                system.refresh_memory();

                let total_memory = system.total_memory();
                let used_memory = system.used_memory();
                let free_memory = system.free_memory();
                let available_memory = system.available_memory();

                Memory {
                    free: free_memory,
                    used: used_memory,
                    allocated: total_memory,
                    reservable: available_memory.max(total_memory),
                }
            } else {
                // Fallback if mutex is poisoned
                self.get_fallback_memory_stats()
            }
        }
        #[cfg(not(feature = "system-stats"))]
        {
            self.get_fallback_memory_stats()
        }
    }

    /// Fallback memory statistics when system monitoring is unavailable
    fn get_fallback_memory_stats(&self) -> Memory {
        Memory {
            free: 1024 * 1024 * 512,        // 512 MB
            used: 1024 * 1024 * 256,        // 256 MB
            allocated: 1024 * 1024 * 768,   // 768 MB
            reservable: 1024 * 1024 * 1024, // 1 GB
        }
    }

    /// Get CPU statistics
    fn get_cpu_stats(&self) -> Cpu {
        #[cfg(feature = "system-stats")]
        {
            if let (Ok(mut system), Ok(mut last_refresh)) =
                (self.system.lock(), self.last_cpu_refresh.lock())
            {
                let now = SystemTime::now();

                // Only refresh CPU if enough time has passed (minimum 1 second for accurate readings)
                if now
                    .duration_since(*last_refresh)
                    .unwrap_or(Duration::from_secs(0))
                    >= Duration::from_secs(1)
                {
                    system.refresh_cpu_usage();
                    *last_refresh = now;
                }

                let cores = system.cpus().len() as u32;
                let global_cpu_usage = system.global_cpu_usage() as f64 / 100.0; // Convert percentage to ratio

                // Calculate process-specific CPU usage (approximation)
                let process_cpu_usage = self.get_process_cpu_usage(&system);

                Cpu {
                    cores,
                    system_load: global_cpu_usage.clamp(0.0, 1.0),
                    lavalink_load: process_cpu_usage.clamp(0.0, 1.0),
                }
            } else {
                // Fallback if mutex is poisoned
                self.get_fallback_cpu_stats()
            }
        }
        #[cfg(not(feature = "system-stats"))]
        {
            self.get_fallback_cpu_stats()
        }
    }

    /// Get process-specific CPU usage
    #[cfg(feature = "system-stats")]
    fn get_process_cpu_usage(&self, system: &System) -> f64 {
        let current_pid = sysinfo::get_current_pid().ok();
        if let Some(pid) = current_pid {
            if let Some(process) = system.process(pid) {
                return process.cpu_usage() as f64 / 100.0; // Convert percentage to ratio
            }
        }

        // Fallback: estimate based on system load
        system.global_cpu_usage() as f64 / 100.0 * 0.1 // Assume 10% of system load
    }

    /// Fallback CPU statistics when system monitoring is unavailable
    fn get_fallback_cpu_stats(&self) -> Cpu {
        #[cfg(feature = "rest-api")]
        let cores = num_cpus::get() as u32;
        #[cfg(not(feature = "rest-api"))]
        let cores = 1;

        Cpu {
            cores,
            system_load: 0.1,    // 10%
            lavalink_load: 0.05, // 5%
        }
    }

    /// Get frame statistics
    fn get_frame_stats(&self) -> FrameStats {
        // TODO: Implement actual frame statistics collection
        // For now, return placeholder values
        FrameStats {
            sent: 1000,
            nulled: 5,
            deficit: 2,
        }
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_stats_collector_creation() {
        let stats_collector = StatsCollector::new();
        let stats = stats_collector.get_stats().await;

        // Basic validation - uptime is always valid (u64 is always >= 0)
        assert!(stats.memory.free > 0);
        // Remove useless comparison for unsigned integer
        assert!(stats.memory.allocated > 0);
        assert!(stats.memory.reservable > 0);
        assert!(stats.cpu.cores > 0);
        assert!(stats.cpu.system_load >= 0.0);
        assert!(stats.cpu.lavalink_load >= 0.0);
        assert!(stats.frame_stats.is_some());
    }

    #[tokio::test]
    async fn test_stats_with_players() {
        let stats_collector = StatsCollector::new();
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();
        let player_manager = PlayerManager::with_event_sender(event_sender);

        let stats = stats_collector
            .get_stats_with_players(&player_manager)
            .await;

        // Should have zero players initially
        assert_eq!(stats.players, 0);
        assert_eq!(stats.playing_players, 0);

        // Create a player
        let _player = player_manager
            .get_or_create_player("test_guild".to_string(), "test_session".to_string())
            .await;

        let stats = stats_collector
            .get_stats_with_players(&player_manager)
            .await;

        // Should now have one player
        assert_eq!(stats.players, 1);
        assert_eq!(stats.playing_players, 0); // Not playing anything yet
    }

    #[cfg(feature = "system-stats")]
    #[tokio::test]
    async fn test_real_system_stats() {
        let stats_collector = StatsCollector::new();
        let stats = stats_collector.get_stats().await;

        // With real system stats, we should get actual values
        #[cfg(feature = "system-stats")]
        {
            // Memory should be realistic (more than 100MB, less than 1TB)
            assert!(stats.memory.allocated > 100 * 1024 * 1024); // > 100MB
            assert!(stats.memory.allocated < 1024 * 1024 * 1024 * 1024); // < 1TB

            // CPU cores should be reasonable (1-128)
            assert!(stats.cpu.cores >= 1);
            assert!(stats.cpu.cores <= 128);

            // CPU usage should be valid percentages
            assert!(stats.cpu.system_load >= 0.0);
            assert!(stats.cpu.system_load <= 1.0);
            assert!(stats.cpu.lavalink_load >= 0.0);
            assert!(stats.cpu.lavalink_load <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_stats_format_compatibility() {
        let stats_collector = StatsCollector::new();
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();
        let player_manager = PlayerManager::with_event_sender(event_sender);

        let stats = stats_collector
            .get_stats_with_players(&player_manager)
            .await;

        // Test JSON serialization (what Discord bots will receive)
        let json = serde_json::to_string(&stats).expect("Stats should serialize to JSON");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should be valid");

        // Verify required fields are present
        assert!(parsed.get("players").is_some());
        assert!(parsed.get("playingPlayers").is_some());
        assert!(parsed.get("uptime").is_some());
        assert!(parsed.get("memory").is_some());
        assert!(parsed.get("cpu").is_some());
        assert!(parsed.get("frameStats").is_some());

        // Verify memory structure
        let memory = parsed.get("memory").unwrap();
        assert!(memory.get("free").is_some());
        assert!(memory.get("used").is_some());
        assert!(memory.get("allocated").is_some());
        assert!(memory.get("reservable").is_some());

        // Verify CPU structure
        let cpu = parsed.get("cpu").unwrap();
        assert!(cpu.get("cores").is_some());
        assert!(cpu.get("systemLoad").is_some());
        assert!(cpu.get("lavalinkLoad").is_some());
    }
}

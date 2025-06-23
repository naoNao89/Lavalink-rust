// Route planner implementation for IP rotation and management
// This module handles IP rotation strategies and failing address management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{
    config::RateLimitConfig,
    protocol::{FailingAddress, IpBlock, RoutePlannerDetails},
};

#[cfg(test)]
mod tests;

/// Route planner strategy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutePlannerStrategy {
    #[serde(rename = "RotateOnBan")]
    RotateOnBan,
    #[serde(rename = "LoadBalance")]
    LoadBalance,
    #[serde(rename = "NanoSwitch")]
    NanoSwitch,
    #[serde(rename = "RotatingNanoSwitch")]
    RotatingNanoSwitch,
}

/// Route planner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlannerConfig {
    #[serde(rename = "ipBlocks")]
    pub ip_blocks: Vec<String>,
    #[serde(rename = "excludedIps")]
    pub excluded_ips: Option<Vec<String>>,
    pub strategy: RoutePlannerStrategy,
    #[serde(rename = "searchTriggersFail")]
    pub search_triggers_fail: Option<bool>,
    #[serde(rename = "retryLimit")]
    pub retry_limit: Option<i32>,
}

/// Failing address information
#[derive(Debug, Clone)]
pub struct FailingAddressInfo {
    pub address: IpAddr,
    pub failing_timestamp: u64,
    #[allow(dead_code)] // Used in filtering logic
    pub retry_count: u32,
}

/// Route planner implementation
#[derive(Debug)]
pub struct RoutePlanner {
    config: RoutePlannerConfig,
    available_ips: Vec<IpAddr>,
    #[allow(dead_code)] // Used for validation and future features
    excluded_ips: HashSet<IpAddr>,
    failing_addresses: Arc<RwLock<HashMap<IpAddr, FailingAddressInfo>>>,
    current_index: Arc<RwLock<usize>>,
    rotate_index: Arc<RwLock<String>>,
}

impl RoutePlanner {
    /// Create a new route planner with the given configuration
    pub fn new(config: RoutePlannerConfig) -> Result<Self> {
        let mut available_ips = Vec::new();
        let mut excluded_ips = HashSet::new();

        // Parse IP blocks and generate available IPs
        for block in &config.ip_blocks {
            let ips = Self::parse_ip_block(block)?;
            available_ips.extend(ips);
        }

        // Parse excluded IPs
        if let Some(excluded) = &config.excluded_ips {
            for ip_str in excluded {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    excluded_ips.insert(ip);
                } else {
                    warn!("Invalid excluded IP address: {}", ip_str);
                }
            }
        }

        // Remove excluded IPs from available IPs
        available_ips.retain(|ip| !excluded_ips.contains(ip));

        info!(
            "Route planner initialized with {} available IPs, {} excluded IPs",
            available_ips.len(),
            excluded_ips.len()
        );

        Ok(Self {
            config,
            available_ips,
            excluded_ips,
            failing_addresses: Arc::new(RwLock::new(HashMap::new())),
            current_index: Arc::new(RwLock::new(0)),
            rotate_index: Arc::new(RwLock::new("0".to_string())),
        })
    }

    /// Parse an IP block string (e.g., "192.168.1.0/24") into individual IPs
    fn parse_ip_block(block: &str) -> Result<Vec<IpAddr>> {
        let mut ips = Vec::new();

        if block.contains('/') {
            // CIDR notation
            let parts: Vec<&str> = block.split('/').collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid CIDR notation: {}", block));
            }

            let base_ip: IpAddr = parts[0].parse()?;
            let prefix_len: u8 = parts[1].parse()?;

            match base_ip {
                IpAddr::V4(ipv4) => {
                    ips.extend(Self::generate_ipv4_range(ipv4, prefix_len)?);
                }
                IpAddr::V6(ipv6) => {
                    ips.extend(Self::generate_ipv6_range(ipv6, prefix_len)?);
                }
            }
        } else {
            // Single IP
            let ip: IpAddr = block.parse()?;
            ips.push(ip);
        }

        Ok(ips)
    }

    /// Generate IPv4 range from CIDR
    fn generate_ipv4_range(base: Ipv4Addr, prefix_len: u8) -> Result<Vec<IpAddr>> {
        if prefix_len > 32 {
            return Err(anyhow::anyhow!("Invalid IPv4 prefix length: {}", prefix_len));
        }

        let mut ips = Vec::new();
        let base_u32 = u32::from(base);
        let host_bits = 32 - prefix_len;
        let num_hosts = 1u32 << host_bits;

        // Limit to reasonable number of IPs to prevent memory issues
        let max_ips = 1000;
        let actual_hosts = std::cmp::min(num_hosts, max_ips);

        for i in 0..actual_hosts {
            let ip_u32 = base_u32 + i;
            let ip = Ipv4Addr::from(ip_u32);
            ips.push(IpAddr::V4(ip));
        }

        Ok(ips)
    }

    /// Generate IPv6 range from CIDR (simplified implementation)
    fn generate_ipv6_range(base: Ipv6Addr, prefix_len: u8) -> Result<Vec<IpAddr>> {
        if prefix_len > 128 {
            return Err(anyhow::anyhow!("Invalid IPv6 prefix length: {}", prefix_len));
        }

        // For IPv6, we'll only generate a limited number of addresses
        // to prevent memory issues. In practice, IPv6 ranges are huge.
        let mut ips = Vec::new();
        let base_segments = base.segments();
        
        // Generate up to 100 IPv6 addresses by incrementing the last segment
        for i in 0..100 {
            let mut segments = base_segments;
            segments[7] = segments[7].wrapping_add(i);
            let ip = Ipv6Addr::from(segments);
            ips.push(IpAddr::V6(ip));
        }

        Ok(ips)
    }

    /// Get the next IP address according to the strategy
    #[allow(dead_code)] // Used in tests
    pub async fn get_next_ip(&self) -> Option<IpAddr> {
        if self.available_ips.is_empty() {
            return None;
        }

        let failing_addresses = self.failing_addresses.read().await;
        let available_ips: Vec<IpAddr> = self
            .available_ips
            .iter()
            .filter(|ip| !failing_addresses.contains_key(ip))
            .copied()
            .collect();

        if available_ips.is_empty() {
            warn!("No available IPs - all are marked as failing");
            return None;
        }

        match self.config.strategy {
            RoutePlannerStrategy::RotateOnBan | RoutePlannerStrategy::LoadBalance => {
                let mut index = self.current_index.write().await;
                let ip = available_ips[*index % available_ips.len()];
                *index = (*index + 1) % available_ips.len();
                Some(ip)
            }
            RoutePlannerStrategy::NanoSwitch | RoutePlannerStrategy::RotatingNanoSwitch => {
                // For nano strategies, use a more complex rotation
                let mut index = self.current_index.write().await;
                let ip = available_ips[*index % available_ips.len()];
                *index = (*index + 1) % available_ips.len();
                Some(ip)
            }
        }
    }

    /// Mark an IP address as failing
    #[allow(dead_code)] // Used in tests
    pub async fn mark_failing(&self, ip: IpAddr) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut failing_addresses = self.failing_addresses.write().await;
        
        let retry_count = failing_addresses
            .get(&ip)
            .map(|info| info.retry_count + 1)
            .unwrap_or(1);

        let info = FailingAddressInfo {
            address: ip,
            failing_timestamp: timestamp,
            retry_count,
        };

        failing_addresses.insert(ip, info);
        
        info!(
            "Marked IP {} as failing (retry count: {})",
            ip, retry_count
        );
    }

    /// Unmark a specific IP address
    pub async fn unmark_address(&self, ip: IpAddr) -> bool {
        let mut failing_addresses = self.failing_addresses.write().await;
        let removed = failing_addresses.remove(&ip).is_some();
        
        if removed {
            info!("Unmarked IP {} as failing", ip);
        } else {
            debug!("IP {} was not marked as failing", ip);
        }
        
        removed
    }

    /// Unmark all failing addresses
    pub async fn unmark_all(&self) -> usize {
        let mut failing_addresses = self.failing_addresses.write().await;
        let count = failing_addresses.len();
        failing_addresses.clear();
        
        info!("Unmarked {} failing IP addresses", count);
        count
    }

    /// Get current route planner status
    pub async fn get_status(&self) -> RoutePlannerDetails {
        let failing_addresses = self.failing_addresses.read().await;
        let current_index = self.current_index.read().await;
        let rotate_index = self.rotate_index.read().await;

        let failing_addrs: Vec<FailingAddress> = failing_addresses
            .values()
            .filter(|info| info.retry_count > 0) // Only include addresses that have actually failed
            .map(|info| FailingAddress {
                address: info.address.to_string(),
                failing_timestamp: info.failing_timestamp,
                failing_time: chrono::DateTime::from_timestamp_millis(info.failing_timestamp as i64)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            })
            .collect();

        match self.config.strategy {
            RoutePlannerStrategy::RotateOnBan | RoutePlannerStrategy::LoadBalance => {
                RoutePlannerDetails::Rotating {
                    ip_block: IpBlock {
                        ip_type: "ipv4".to_string(),
                        size: self.available_ips.len().to_string(),
                    },
                    failing_addresses: failing_addrs,
                    rotate_index: rotate_index.clone(),
                    ip_index: current_index.to_string(),
                    current_address: self
                        .available_ips
                        .get(*current_index % self.available_ips.len())
                        .map(|ip| ip.to_string())
                        .unwrap_or_default(),
                }
            }
            RoutePlannerStrategy::NanoSwitch => RoutePlannerDetails::Nano {
                current_address_index: *current_index as u64,
            },
            RoutePlannerStrategy::RotatingNanoSwitch => RoutePlannerDetails::RotatingNano {
                block_index: rotate_index.clone(),
                current_address_index: *current_index as u64,
            },
        }
    }

    /// Get the current IP address
    #[allow(dead_code)] // Used in tests
    pub async fn get_current_ip(&self) -> Option<IpAddr> {
        if self.available_ips.is_empty() {
            return None;
        }

        let index = self.current_index.read().await;
        Some(self.available_ips[*index % self.available_ips.len()])
    }

    /// Check if an IP is excluded
    #[allow(dead_code)] // Used for validation and future features
    pub fn is_excluded(&self, ip: &IpAddr) -> bool {
        self.excluded_ips.contains(ip)
    }

    /// Get all excluded IPs
    #[allow(dead_code)] // Used for validation and future features
    pub fn get_excluded_ips(&self) -> &HashSet<IpAddr> {
        &self.excluded_ips
    }
}

impl Default for RoutePlannerConfig {
    fn default() -> Self {
        Self {
            ip_blocks: vec!["0.0.0.0/0".to_string()],
            excluded_ips: None,
            strategy: RoutePlannerStrategy::RotateOnBan,
            search_triggers_fail: Some(true),
            retry_limit: Some(-1),
        }
    }
}

impl TryFrom<&RateLimitConfig> for RoutePlannerConfig {
    type Error = anyhow::Error;

    fn try_from(config: &RateLimitConfig) -> Result<Self> {
        let strategy = match config.strategy.as_deref() {
            Some("RotateOnBan") => RoutePlannerStrategy::RotateOnBan,
            Some("LoadBalance") => RoutePlannerStrategy::LoadBalance,
            Some("NanoSwitch") => RoutePlannerStrategy::NanoSwitch,
            Some("RotatingNanoSwitch") => RoutePlannerStrategy::RotatingNanoSwitch,
            Some(other) => {
                return Err(anyhow::anyhow!("Unknown route planner strategy: {}", other));
            }
            None => RoutePlannerStrategy::RotateOnBan,
        };

        let ip_blocks = config
            .ip_blocks
            .as_ref()
            .cloned()
            .unwrap_or_else(|| vec!["0.0.0.0/0".to_string()]);

        Ok(RoutePlannerConfig {
            ip_blocks,
            excluded_ips: config.excluded_ips.clone(),
            strategy,
            search_triggers_fail: config.search_triggers_fail,
            retry_limit: config.retry_limit,
        })
    }
}

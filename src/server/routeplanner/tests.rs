// Route planner tests
// Comprehensive test suite for route planner functionality

use super::*;
use std::net::{IpAddr, Ipv4Addr};

#[cfg(test)]
mod route_planner_tests {
    use super::*;

    fn create_test_config() -> RoutePlannerConfig {
        RoutePlannerConfig {
            ip_blocks: vec!["192.168.1.0/24".to_string()],
            excluded_ips: Some(vec!["192.168.1.1".to_string()]),
            strategy: RoutePlannerStrategy::RotateOnBan,
            search_triggers_fail: Some(true),
            retry_limit: Some(3),
        }
    }

    #[tokio::test]
    async fn test_route_planner_creation() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        // Should have generated IPs from the CIDR block
        assert!(!route_planner.available_ips.is_empty());
        
        // Should exclude the specified IP
        let excluded_ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!route_planner.available_ips.contains(&excluded_ip));
    }

    #[tokio::test]
    async fn test_ip_block_parsing() {
        // Test single IP
        let ips = RoutePlanner::parse_ip_block("192.168.1.1").unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0], "192.168.1.1".parse::<IpAddr>().unwrap());

        // Test CIDR block
        let ips = RoutePlanner::parse_ip_block("192.168.1.0/30").unwrap();
        assert_eq!(ips.len(), 4); // /30 gives 4 IPs

        // Test invalid CIDR
        assert!(RoutePlanner::parse_ip_block("192.168.1.0/33").is_err());
    }

    #[tokio::test]
    async fn test_ipv4_range_generation() {
        let base = Ipv4Addr::new(192, 168, 1, 0);
        let ips = RoutePlanner::generate_ipv4_range(base, 30).unwrap();
        
        assert_eq!(ips.len(), 4);
        assert_eq!(ips[0], IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)));
        assert_eq!(ips[1], IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(ips[2], IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
        assert_eq!(ips[3], IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)));
    }

    #[tokio::test]
    async fn test_get_next_ip() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        // Should return an IP
        let ip1 = route_planner.get_next_ip().await;
        assert!(ip1.is_some());

        // Should rotate to next IP
        let ip2 = route_planner.get_next_ip().await;
        assert!(ip2.is_some());
        
        // IPs should be different (unless only one available)
        if route_planner.available_ips.len() > 1 {
            assert_ne!(ip1, ip2);
        }
    }

    #[tokio::test]
    async fn test_mark_failing() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        let test_ip: IpAddr = "192.168.1.10".parse().unwrap();
        
        // Mark IP as failing
        route_planner.mark_failing(test_ip).await;

        // Check that it's marked as failing
        let failing_addresses = route_planner.failing_addresses.read().await;
        assert!(failing_addresses.contains_key(&test_ip));
        assert_eq!(failing_addresses[&test_ip].retry_count, 1);
    }

    #[tokio::test]
    async fn test_unmark_address() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        let test_ip: IpAddr = "192.168.1.10".parse().unwrap();
        
        // Mark IP as failing
        route_planner.mark_failing(test_ip).await;
        
        // Unmark the IP
        let unmarked = route_planner.unmark_address(test_ip).await;
        assert!(unmarked);

        // Check that it's no longer marked as failing
        let failing_addresses = route_planner.failing_addresses.read().await;
        assert!(!failing_addresses.contains_key(&test_ip));
    }

    #[tokio::test]
    async fn test_unmark_all() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        let test_ip1: IpAddr = "192.168.1.10".parse().unwrap();
        let test_ip2: IpAddr = "192.168.1.11".parse().unwrap();
        
        // Mark multiple IPs as failing
        route_planner.mark_failing(test_ip1).await;
        route_planner.mark_failing(test_ip2).await;
        
        // Unmark all
        let count = route_planner.unmark_all().await;
        assert_eq!(count, 2);

        // Check that no IPs are marked as failing
        let failing_addresses = route_planner.failing_addresses.read().await;
        assert!(failing_addresses.is_empty());
    }

    #[tokio::test]
    async fn test_get_status() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        let test_ip: IpAddr = "192.168.1.10".parse().unwrap();
        route_planner.mark_failing(test_ip).await;

        let status = route_planner.get_status().await;
        
        match status {
            RoutePlannerDetails::Rotating { failing_addresses, .. } => {
                assert_eq!(failing_addresses.len(), 1);
                assert_eq!(failing_addresses[0].address, test_ip.to_string());
            }
            _ => panic!("Expected Rotating route planner details"),
        }
    }

    #[tokio::test]
    async fn test_strategy_nano() {
        let mut config = create_test_config();
        config.strategy = RoutePlannerStrategy::NanoSwitch;
        
        let route_planner = RoutePlanner::new(config).unwrap();
        let status = route_planner.get_status().await;
        
        match status {
            RoutePlannerDetails::Nano { .. } => {
                // Expected for NanoSwitch strategy
            }
            _ => panic!("Expected Nano route planner details"),
        }
    }

    #[tokio::test]
    async fn test_strategy_rotating_nano() {
        let mut config = create_test_config();
        config.strategy = RoutePlannerStrategy::RotatingNanoSwitch;
        
        let route_planner = RoutePlanner::new(config).unwrap();
        let status = route_planner.get_status().await;
        
        match status {
            RoutePlannerDetails::RotatingNano { .. } => {
                // Expected for RotatingNanoSwitch strategy
            }
            _ => panic!("Expected RotatingNano route planner details"),
        }
    }

    #[tokio::test]
    async fn test_failing_ip_exclusion() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        // Get all available IPs
        let _available_count = route_planner.available_ips.len();
        
        // Mark all IPs as failing
        for ip in &route_planner.available_ips {
            route_planner.mark_failing(*ip).await;
        }

        // Should return None when all IPs are failing
        let next_ip = route_planner.get_next_ip().await;
        assert!(next_ip.is_none());
    }

    #[tokio::test]
    async fn test_retry_count_increment() {
        let config = create_test_config();
        let route_planner = RoutePlanner::new(config).unwrap();

        let test_ip: IpAddr = "192.168.1.10".parse().unwrap();
        
        // Mark IP as failing multiple times
        route_planner.mark_failing(test_ip).await;
        route_planner.mark_failing(test_ip).await;
        route_planner.mark_failing(test_ip).await;

        // Check retry count
        let failing_addresses = route_planner.failing_addresses.read().await;
        assert_eq!(failing_addresses[&test_ip].retry_count, 3);
    }

    #[tokio::test]
    async fn test_config_conversion() {
        use crate::config::RateLimitConfig;
        
        let rate_limit_config = RateLimitConfig {
            ip_blocks: Some(vec!["10.0.0.0/8".to_string()]),
            excluded_ips: Some(vec!["10.0.0.1".to_string()]),
            strategy: Some("LoadBalance".to_string()),
            search_triggers_fail: Some(false),
            retry_limit: Some(5),
        };

        let route_planner_config = RoutePlannerConfig::try_from(&rate_limit_config).unwrap();
        
        assert_eq!(route_planner_config.ip_blocks, vec!["10.0.0.0/8"]);
        assert_eq!(route_planner_config.excluded_ips, Some(vec!["10.0.0.1".to_string()]));
        assert!(matches!(route_planner_config.strategy, RoutePlannerStrategy::LoadBalance));
        assert_eq!(route_planner_config.search_triggers_fail, Some(false));
        assert_eq!(route_planner_config.retry_limit, Some(5));
    }

    #[tokio::test]
    async fn test_invalid_strategy_conversion() {
        use crate::config::RateLimitConfig;
        
        let rate_limit_config = RateLimitConfig {
            ip_blocks: Some(vec!["10.0.0.0/8".to_string()]),
            excluded_ips: None,
            strategy: Some("InvalidStrategy".to_string()),
            search_triggers_fail: None,
            retry_limit: None,
        };

        let result = RoutePlannerConfig::try_from(&rate_limit_config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_ip_blocks() {
        let config = RoutePlannerConfig {
            ip_blocks: vec![],
            excluded_ips: None,
            strategy: RoutePlannerStrategy::RotateOnBan,
            search_triggers_fail: Some(true),
            retry_limit: Some(-1),
        };

        let route_planner = RoutePlanner::new(config).unwrap();
        assert!(route_planner.available_ips.is_empty());
        
        let next_ip = route_planner.get_next_ip().await;
        assert!(next_ip.is_none());
    }
}

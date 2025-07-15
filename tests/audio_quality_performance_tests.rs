#[cfg(test)]
mod audio_quality_performance_tests {
    use lavalink_rust::audio::quality::*;
    use lavalink_rust::audio::streaming::*;
    use lavalink_rust::protocol::{Track, TrackInfo};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    /// Helper function to create test audio quality config
    fn create_test_quality_config() -> AudioQualityConfig {
        AudioQualityConfig {
            bitrate: 128,
            sample_rate: AudioSampleRate::Hz48000,
            channels: AudioChannels::Stereo,
            quality_preset: QualityPreset::High,
            soft_clipping: true,
            buffer_config: BufferConfig {
                playout_buffer_length: 5,
                playout_spike_length: 2,
                decode_state_timeout_ms: 60000,
            },
            opus_quality: 8,
            adaptive_quality: true,
        }
    }

    /// Helper function to create test track
    #[allow(dead_code)]
    fn create_test_track() -> Track {
        Track {
            encoded: "test_encoded_track".to_string(),
            info: TrackInfo {
                identifier: "test_track_001".to_string(),
                is_seekable: true,
                author: "Test Artist".to_string(),
                length: 180000, // 3 minutes
                is_stream: false,
                position: 0,
                title: "Test Track".to_string(),
                uri: Some("http://example.com/test.mp3".to_string()),
                artwork_url: None,
                isrc: None,
                source_name: "test".to_string(),
            },
            plugin_info: std::collections::HashMap::new(),
            user_data: std::collections::HashMap::new(),
        }
    }

    /// Helper function to create network metrics for testing
    fn create_test_network_metrics(rtt: u32, packet_loss: f32, jitter: u32) -> NetworkMetrics {
        NetworkMetrics {
            rtt_ms: rtt,
            packet_loss,
            jitter_ms: jitter,
            bandwidth_kbps: 1000,
        }
    }

    #[tokio::test]
    async fn test_quality_manager_creation_performance() {
        let num_managers = 100;
        let start_time = Instant::now();
        let mut managers = Vec::new();

        // Create multiple quality managers
        for i in 0..num_managers {
            let guild_id = format!("test_guild_{i}");
            let config = create_test_quality_config();
            let manager = AudioQualityManager::new(guild_id, config);
            managers.push(manager);
        }

        let creation_time = start_time.elapsed();

        // Quality manager creation should be fast
        assert!(
            creation_time < Duration::from_secs(2),
            "Quality manager creation took too long: {creation_time:?}"
        );

        println!("Created {num_managers} quality managers in {creation_time:?}");
        println!(
            "Average creation time: {:?}",
            creation_time / num_managers as u32
        );
    }

    #[tokio::test]
    async fn test_quality_metrics_update_performance() {
        let manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let num_updates = 1000;
        let start_time = Instant::now();

        // Perform rapid metrics updates
        for i in 0..num_updates {
            let effective_bitrate = 128 + (i % 64) as u32;
            let buffer_health = 80 + (i % 20) as u8;
            let encoding_performance = 85 + (i % 15) as u8;
            let stream_stability = 90 + (i % 10) as u8;

            manager
                .update_quality_metrics(
                    effective_bitrate,
                    buffer_health,
                    encoding_performance,
                    stream_stability,
                )
                .await
                .expect("Failed to update metrics");
        }

        let update_time = start_time.elapsed();

        // Metrics updates should be very fast
        assert!(
            update_time < Duration::from_secs(1),
            "Quality metrics updates took too long: {update_time:?}"
        );

        println!("Performed {num_updates} metrics updates in {update_time:?}");
        println!(
            "Average update time: {:?}",
            update_time / num_updates as u32
        );
    }

    #[tokio::test]
    async fn test_quality_adjustment_response_time() {
        let mut manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());

        // Simulate poor network conditions
        let poor_metrics = create_test_network_metrics(200, 0.05, 50);
        let start_time = Instant::now();

        // Trigger quality adjustment
        manager.update_network_metrics(poor_metrics);
        manager
            .trigger_quality_adjustment()
            .await
            .expect("Failed to adjust quality");

        let adjustment_time = start_time.elapsed();

        // Quality adjustment should be responsive
        assert!(
            adjustment_time < Duration::from_millis(100),
            "Quality adjustment took too long: {adjustment_time:?}"
        );

        println!("Quality adjustment completed in {adjustment_time:?}");
    }

    #[tokio::test]
    async fn test_streaming_manager_startup_performance() {
        let num_managers = 50;
        let start_time = Instant::now();
        let mut managers = Vec::new();

        // Create multiple streaming managers
        for i in 0..num_managers {
            let guild_id = format!("test_guild_{i}");
            let manager = AudioStreamingManager::new(guild_id);
            managers.push(manager);
        }

        let creation_time = start_time.elapsed();

        // Streaming manager creation should be fast
        assert!(
            creation_time < Duration::from_secs(1),
            "Streaming manager creation took too long: {creation_time:?}"
        );

        println!("Created {num_managers} streaming managers in {creation_time:?}");
        println!(
            "Average creation time: {:?}",
            creation_time / num_managers as u32
        );
    }

    #[tokio::test]
    async fn test_quality_integration_performance() {
        let quality_manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let streaming_manager =
            AudioStreamingManager::with_quality_manager("test_guild".to_string(), quality_manager);

        let start_time = Instant::now();

        // Test quality manager integration
        streaming_manager
            .set_quality_manager(AudioQualityManager::new(
                "test_guild".to_string(),
                create_test_quality_config(),
            ))
            .await;

        let integration_time = start_time.elapsed();

        // Quality integration should be fast
        assert!(
            integration_time < Duration::from_millis(50),
            "Quality integration took too long: {integration_time:?}"
        );

        println!("Quality integration completed in {integration_time:?}");
    }

    #[tokio::test]
    async fn test_bitrate_adjustment_performance() {
        let mut manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let num_adjustments = 100;
        let start_time = Instant::now();

        // Perform rapid bitrate adjustments
        for i in 0..num_adjustments {
            let _network_quality = if i % 2 == 0 { 95 } else { 60 }; // Alternate between good and poor
            let metrics = create_test_network_metrics(
                if i % 2 == 0 { 50 } else { 150 },
                if i % 2 == 0 { 0.001 } else { 0.02 },
                if i % 2 == 0 { 10 } else { 30 },
            );

            manager.update_network_metrics(metrics);
            manager
                .trigger_quality_adjustment()
                .await
                .expect("Failed to adjust quality");
        }

        let adjustment_time = start_time.elapsed();

        // Bitrate adjustments should be efficient
        assert!(
            adjustment_time < Duration::from_secs(5),
            "Bitrate adjustments took too long: {adjustment_time:?}"
        );

        println!("Performed {num_adjustments} bitrate adjustments in {adjustment_time:?}");
        println!(
            "Average adjustment time: {:?}",
            adjustment_time / num_adjustments as u32
        );
    }

    #[tokio::test]
    async fn test_quality_monitoring_overhead() {
        let manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let monitoring_duration = Duration::from_secs(2);
        let start_time = Instant::now();
        let mut update_count = 0;

        // Simulate continuous monitoring
        while start_time.elapsed() < monitoring_duration {
            manager
                .update_quality_metrics(128, 85, 90, 95)
                .await
                .expect("Failed to update metrics");
            manager.get_quality_metrics().await;
            update_count += 1;

            // Small delay to simulate realistic monitoring interval
            sleep(Duration::from_millis(10)).await;
        }

        let total_time = start_time.elapsed();
        let updates_per_second = update_count as f64 / total_time.as_secs_f64();

        // Should handle at least 50 updates per second
        assert!(
            updates_per_second >= 50.0,
            "Quality monitoring performance too low: {updates_per_second:.2} updates/sec"
        );

        println!("Quality monitoring: {updates_per_second:.2} updates/sec over {total_time:?}");
    }

    #[tokio::test]
    async fn test_quality_analytics_performance() {
        let manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());

        // Generate some quality history
        for i in 0..100 {
            let quality_score = 70 + (i % 30) as u8;
            manager
                .update_quality_metrics(128, quality_score, quality_score, quality_score)
                .await
                .expect("Failed to update metrics");
        }

        let start_time = Instant::now();

        // Generate analytics report
        let report = manager
            .generate_quality_report()
            .await
            .expect("Failed to generate report");

        let analytics_time = start_time.elapsed();

        // Analytics generation should be fast
        assert!(
            analytics_time < Duration::from_millis(100),
            "Quality analytics took too long: {analytics_time:?}"
        );

        // Verify report contains expected data
        assert!(report["guild_id"] == "test_guild");
        assert!(
            report["current_metrics"]["average_quality_score"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );

        println!("Quality analytics generated in {analytics_time:?}");
        println!("Report generated for guild: {}", report["guild_id"]);
    }

    #[tokio::test]
    async fn test_concurrent_quality_operations() {
        // Create multiple managers to avoid mutex contention
        let num_concurrent_ops = 50;
        let start_time = Instant::now();

        // Spawn concurrent operations
        let mut handles = Vec::new();
        for i in 0..num_concurrent_ops {
            let handle = tokio::spawn(async move {
                let mut manager = AudioQualityManager::new(
                    "test_guild".to_string(),
                    create_test_quality_config(),
                );

                // Mix of different operations
                match i % 4 {
                    0 => {
                        manager
                            .update_quality_metrics(128, 80, 85, 90)
                            .await
                            .expect("Failed to update metrics");
                    }
                    1 => {
                        let _metrics = manager.get_quality_metrics().await;
                    }
                    2 => {
                        let metrics = create_test_network_metrics(100, 0.01, 20);
                        manager.update_network_metrics(metrics);
                    }
                    3 => {
                        manager
                            .trigger_quality_adjustment()
                            .await
                            .expect("Failed to adjust quality");
                    }
                    _ => unreachable!(),
                }
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.expect("Task failed");
        }

        let concurrent_time = start_time.elapsed();

        // Concurrent operations should complete efficiently
        assert!(
            concurrent_time < Duration::from_secs(2),
            "Concurrent quality operations took too long: {concurrent_time:?}"
        );

        println!("Completed {num_concurrent_ops} concurrent operations in {concurrent_time:?}");
        println!(
            "Average operation time: {:?}",
            concurrent_time / num_concurrent_ops as u32
        );
    }

    #[tokio::test]
    async fn test_quality_preset_switching_performance() {
        let manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let presets = [
            QualityPreset::Voice,
            QualityPreset::Low,
            QualityPreset::Medium,
            QualityPreset::High,
            QualityPreset::Maximum,
        ];

        let num_switches = 100;
        let start_time = Instant::now();

        // Perform rapid preset switches
        for i in 0..num_switches {
            let preset = &presets[i % presets.len()];
            let config = preset.to_config();

            // Simulate preset switch by updating configuration
            manager
                .update_quality_metrics(config.bitrate, 85, 90, 95)
                .await
                .expect("Failed to update metrics");
        }

        let switching_time = start_time.elapsed();

        // Preset switching should be fast
        assert!(
            switching_time < Duration::from_secs(2),
            "Quality preset switching took too long: {switching_time:?}"
        );

        println!("Performed {num_switches} preset switches in {switching_time:?}");
        println!(
            "Average switch time: {:?}",
            switching_time / num_switches as u32
        );
    }

    #[tokio::test]
    async fn test_quality_adaptation_under_stress() {
        let mut manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let stress_duration = Duration::from_secs(5);
        let start_time = Instant::now();
        let mut adaptation_count = 0;

        // Simulate rapidly changing network conditions
        while start_time.elapsed() < stress_duration {
            let cycle_time = start_time.elapsed().as_millis() % 1000;

            // Create varying network conditions
            let (latency, packet_loss, jitter) = if cycle_time < 250 {
                (50, 0.001, 10) // Excellent
            } else if cycle_time < 500 {
                (100, 0.01, 20) // Good
            } else if cycle_time < 750 {
                (200, 0.03, 40) // Poor
            } else {
                (300, 0.08, 80) // Very poor
            };

            let metrics = create_test_network_metrics(latency, packet_loss, jitter);
            manager.update_network_metrics(metrics);
            manager
                .trigger_quality_adjustment()
                .await
                .expect("Failed to adjust quality");

            adaptation_count += 1;
            sleep(Duration::from_millis(50)).await;
        }

        let total_time = start_time.elapsed();
        let adaptations_per_second = adaptation_count as f64 / total_time.as_secs_f64();

        // Should handle at least 10 adaptations per second under stress
        assert!(
            adaptations_per_second >= 10.0,
            "Quality adaptation under stress too slow: {adaptations_per_second:.2} adaptations/sec"
        );

        println!("Quality adaptation under stress: {adaptations_per_second:.2} adaptations/sec");
    }

    #[tokio::test]
    async fn test_memory_usage_during_quality_operations() {
        let mut manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());

        // Perform many operations to test memory usage
        for i in 0..1000 {
            // Update metrics
            manager
                .update_quality_metrics(
                    128 + (i % 64) as u32,
                    70 + (i % 30) as u8,
                    80 + (i % 20) as u8,
                    85 + (i % 15) as u8,
                )
                .await
                .expect("Failed to update metrics");

            // Update network metrics
            let metrics = create_test_network_metrics(
                50 + (i % 100) as u32,
                0.001 + (i as f32 * 0.0001),
                10 + (i % 20) as u32,
            );
            manager.update_network_metrics(metrics);

            // Adjust quality periodically
            if i % 10 == 0 {
                manager
                    .trigger_quality_adjustment()
                    .await
                    .expect("Failed to adjust quality");
            }

            // Generate reports periodically
            if i % 50 == 0 {
                let _report = manager
                    .generate_quality_report()
                    .await
                    .expect("Failed to generate report");
            }
        }

        // Get final metrics to ensure everything is still working
        let final_metrics = manager.get_quality_metrics().await;
        assert!(final_metrics.effective_bitrate > 0);
        assert!(final_metrics.average_quality_score > 0);

        println!("Memory usage test completed successfully");
        println!(
            "Final effective bitrate: {} kbps",
            final_metrics.effective_bitrate
        );
        println!(
            "Final quality score: {}",
            final_metrics.average_quality_score
        );
    }

    #[tokio::test]
    async fn test_streaming_quality_integration_performance() {
        let quality_manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());
        let streaming_manager =
            AudioStreamingManager::with_quality_manager("test_guild".to_string(), quality_manager);

        let num_operations = 100;
        let start_time = Instant::now();

        // Test integrated operations
        for i in 0..num_operations {
            // Check streaming status
            let _is_streaming = streaming_manager.is_streaming().await;

            // Get stream health
            let _health = streaming_manager.get_stream_health().await;

            // Update quality manager if available
            if i % 10 == 0 {
                let new_quality_manager = AudioQualityManager::new(
                    "test_guild".to_string(),
                    create_test_quality_config(),
                );
                streaming_manager
                    .set_quality_manager(new_quality_manager)
                    .await;
            }
        }

        let integration_time = start_time.elapsed();

        // Integration operations should be efficient
        assert!(
            integration_time < Duration::from_secs(2),
            "Streaming-quality integration took too long: {integration_time:?}"
        );

        println!("Completed {num_operations} integration operations in {integration_time:?}");
        println!(
            "Average integration operation time: {:?}",
            integration_time / num_operations as u32
        );
    }

    #[tokio::test]
    async fn test_quality_degradation_detection_performance() {
        let mut manager =
            AudioQualityManager::new("test_guild".to_string(), create_test_quality_config());

        // Start with good quality and network conditions
        manager.update_network_metrics(create_test_network_metrics(50, 0.1, 5));
        manager
            .update_quality_metrics(320, 95, 95, 95)
            .await
            .expect("Failed to update metrics");

        let start_time = Instant::now();

        // Simulate gradual quality degradation
        for i in 0..50 {
            let degradation_factor = i as f32 / 50.0;
            let bitrate = (320.0 * (1.0 - degradation_factor * 0.7)) as u32;
            let buffer_health = (95.0 * (1.0 - degradation_factor * 0.5)) as u8;
            let encoding_perf = (95.0 * (1.0 - degradation_factor * 0.4)) as u8;
            let stream_stability = (95.0 * (1.0 - degradation_factor * 0.6)) as u8;

            // Also degrade network conditions
            let network_metrics = NetworkMetrics {
                packet_loss: degradation_factor * 15.0, // Increase packet loss
                rtt_ms: (50.0 + degradation_factor * 200.0) as u32, // Increase RTT
                jitter_ms: (5.0 + degradation_factor * 50.0) as u32, // Increase jitter
                bandwidth_kbps: (1000.0 * (1.0 - degradation_factor * 0.5)) as u32, // Decrease bandwidth
            };
            manager.update_network_metrics(network_metrics);

            manager
                .update_quality_metrics(bitrate, buffer_health, encoding_perf, stream_stability)
                .await
                .expect("Failed to update metrics");

            // Check if degradation is detected
            let metrics = manager.get_quality_metrics().await;
            println!(
                "Iteration {}: quality_score={}, trend={:?}",
                i, metrics.average_quality_score, metrics.quality_trend
            );
            if metrics.quality_trend == QualityTrend::Degrading {
                let detection_time = start_time.elapsed();

                // Degradation should be detected quickly
                assert!(
                    detection_time < Duration::from_secs(2),
                    "Quality degradation detection took too long: {detection_time:?}"
                );

                println!(
                    "Quality degradation detected in {:?} after {} updates",
                    detection_time,
                    i + 1
                );
                return;
            }
        }

        // Should have detected degradation by now
        panic!("Quality degradation was not detected");
    }
}

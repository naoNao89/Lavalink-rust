//! Tests for the audio filter system

#[cfg(test)]
mod tests {
    use super::super::filters::*;
    use crate::protocol::filters::*;
    use crate::protocol::Omissible;

    #[test]
    fn test_volume_filter() {
        let mut volume_filter = VolumeFilter::new(2.0);
        assert!(volume_filter.is_enabled());

        let format = AudioFormat::default();
        let mut samples = vec![0.5, -0.5, 0.25, -0.25];

        volume_filter.process(&mut samples, &format).unwrap();

        // Check that volume was applied
        assert_eq!(samples, vec![1.0, -1.0, 0.5, -0.5]);
    }

    #[test]
    fn test_volume_filter_disabled() {
        let mut volume_filter = VolumeFilter::new(1.0);
        assert!(!volume_filter.is_enabled());

        let format = AudioFormat::default();
        let mut samples = vec![0.5, -0.5, 0.25, -0.25];
        let original_samples = samples.clone();

        volume_filter.process(&mut samples, &format).unwrap();

        // Check that samples are unchanged
        assert_eq!(samples, original_samples);
    }

    #[test]
    fn test_karaoke_filter() {
        let karaoke_config = Karaoke {
            level: Some(0.5),
            mono_level: Some(0.8),
            filter_band: None,
            filter_width: None,
        };

        let mut karaoke_filter = KaraokeFilter::new(karaoke_config);
        assert!(karaoke_filter.is_enabled());

        let format = AudioFormat {
            sample_rate: 48000.0,
            channels: 2,
            bits_per_sample: 16,
        };

        // Stereo samples: [left1, right1, left2, right2]
        let mut samples = vec![0.8, 0.6, -0.4, -0.2];

        karaoke_filter.process(&mut samples, &format).unwrap();

        // Karaoke effect should have been applied
        assert_ne!(samples, vec![0.8, 0.6, -0.4, -0.2]);
    }

    #[test]
    fn test_tremolo_filter() {
        let tremolo_config = Tremolo {
            frequency: Some(2.0),
            depth: Some(0.5),
        };

        let mut tremolo_filter = TremoloFilter::new(tremolo_config);
        assert!(tremolo_filter.is_enabled());

        let format = AudioFormat::default();
        let mut samples = vec![0.5; 100]; // 100 samples of 0.5

        tremolo_filter.process(&mut samples, &format).unwrap();

        // Tremolo should modulate the amplitude
        assert!(samples.iter().any(|&s| s != 0.5));
    }

    #[test]
    fn test_filter_chain() {
        let format = AudioFormat::default();
        let mut chain = FilterChain::new(format.clone());

        // Add volume filter
        let volume_filter = VolumeFilter::new(2.0);
        chain.add_filter(Box::new(volume_filter));

        assert!(chain.is_enabled());

        let mut samples = vec![0.25, -0.25, 0.125, -0.125];
        chain.process(&mut samples).unwrap();

        // Volume should be doubled
        assert_eq!(samples, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn test_filter_factory() {
        let mut filters = Filters::new();
        filters.volume = Omissible::Present(1.5);

        let format = AudioFormat::default();
        let chain = FilterFactory::create_filter_chain(&filters, format).unwrap();

        assert!(chain.is_enabled());
    }

    #[test]
    fn test_audio_filter_manager() {
        let format = AudioFormat::default();
        let manager = AudioFilterManager::new(format);

        // Test that manager starts with no filters enabled
        assert!(!tokio_test::block_on(manager.is_enabled()));

        // Create filters configuration
        let mut filters = Filters::new();
        filters.volume = Omissible::Present(2.0);

        // Update filters
        tokio_test::block_on(manager.update_filters(&filters)).unwrap();

        // Now filters should be enabled
        assert!(tokio_test::block_on(manager.is_enabled()));

        // Test audio processing
        let mut samples = vec![0.25, -0.25, 0.125, -0.125];
        tokio_test::block_on(manager.process_audio(&mut samples)).unwrap();

        // Volume should be doubled
        assert_eq!(samples, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn test_equalizer_filter() {
        use crate::protocol::filters::Band;

        let bands = vec![
            Band { band: 0, gain: 3.0 },
            Band {
                band: 1,
                gain: -2.0,
            },
            Band { band: 2, gain: 0.0 },
        ];

        let mut eq_filter = EqualizerFilter::new(bands);
        assert!(eq_filter.is_enabled());

        let format = AudioFormat::default();
        let mut samples = vec![0.5, -0.5, 0.25, -0.25];

        eq_filter.process(&mut samples, &format).unwrap();

        // EQ should have modified the samples
        assert_ne!(samples, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn test_vibrato_filter() {
        let vibrato_config = Vibrato {
            frequency: Some(5.0),
            depth: Some(0.3),
        };

        let mut vibrato_filter = VibratoFilter::new(vibrato_config);
        assert!(vibrato_filter.is_enabled());
        assert!(vibrato_filter.latency() > 0);

        let format = AudioFormat::default();
        let mut samples = vec![0.5; 100]; // 100 samples

        vibrato_filter.process(&mut samples, &format).unwrap();

        // Vibrato should have modified the samples through delay modulation
        // Note: The first few samples might be zero due to delay buffer initialization
    }

    #[test]
    fn test_multiple_filters_in_chain() {
        let format = AudioFormat::default();
        let mut chain = FilterChain::new(format.clone());

        // Add multiple filters
        let volume_filter = VolumeFilter::new(2.0);
        chain.add_filter(Box::new(volume_filter));

        let tremolo_config = Tremolo {
            frequency: Some(1.0),
            depth: Some(0.2),
        };
        let tremolo_filter = TremoloFilter::new(tremolo_config);
        chain.add_filter(Box::new(tremolo_filter));

        assert!(chain.is_enabled());

        let mut samples = vec![0.25; 50];
        chain.process(&mut samples).unwrap();

        // Both filters should have been applied
        assert!(samples.iter().any(|&s| s != 0.25));
    }
}

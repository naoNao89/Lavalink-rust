// Player engine and management tests
// These tests validate the audio player functionality and state management

use lavalink_rust::player::{PlayerManager, TrackEndReason};
use lavalink_rust::protocol::messages::VoiceState;
use lavalink_rust::protocol::Omissible;
use lavalink_rust::protocol::{Filters, PlayerState};
use lavalink_rust::test_utils::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Test player manager creation and basic operations
#[tokio::test]
async fn test_player_manager_creation() {
    let player_manager = PlayerManager::new();

    // Should start with no players - we'll test this by trying to get a non-existent player
    let player = player_manager.get_player("non_existent_guild").await;
    assert!(player.is_none());
}

/// Test player creation and lifecycle
#[tokio::test]
async fn test_player_lifecycle() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    // Create a player
    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;
    assert_eq!(player.read().await.guild_id, guild_id);

    // Verify player exists in manager
    let retrieved_player = player_manager.get_player(&guild_id).await;
    assert!(retrieved_player.is_some());

    // Remove player
    let removed_player = player_manager.remove_player(&guild_id).await;
    assert!(removed_player.is_some());

    // Verify player is removed
    let player_after = player_manager.get_player(&guild_id).await;
    assert!(player_after.is_none());
}

/// Test player state management
#[tokio::test]
async fn test_player_state_management() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;

    // Test initial state
    {
        let player_guard = player.read().await;
        assert_eq!(player_guard.volume, 100);
        assert!(!player_guard.paused);
        assert!(player_guard.current_track.is_none());
    }

    // Test volume change
    {
        let mut player_guard = player.write().await;
        player_guard
            .set_volume(150)
            .await
            .expect("Failed to set volume");
        assert_eq!(player_guard.volume, 150);
    }

    // Test pause state
    {
        let mut player_guard = player.write().await;
        player_guard
            .set_paused(true)
            .await
            .expect("Failed to pause");
        assert!(player_guard.paused);
    }
}

/// Test track playback operations
#[tokio::test]
async fn test_track_playback() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;
    let track = create_mock_track();

    // Play track
    {
        let mut player_guard = player.write().await;
        let result = player_guard.play_track(track.clone(), None, None).await;
        assert!(result.is_ok());

        // Verify track is set
        assert!(player_guard.current_track.is_some());
        if let Some(current) = &player_guard.current_track {
            assert_eq!(current.info.identifier, track.info.identifier);
        }
    }

    // Stop track
    {
        let mut player_guard = player.write().await;
        let result = player_guard.stop_track().await;
        assert!(result.is_ok());

        // Verify track is cleared
        assert!(player_guard.current_track.is_none());
    }
}

/// Test voice state management
#[tokio::test]
async fn test_voice_state_management() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;

    let voice_state = VoiceState {
        token: "test_token".to_string(),
        endpoint: "test_endpoint".to_string(),
        session_id: "test_session_id".to_string(),
        connected: true,
        ping: 50,
    };

    // Set voice state
    {
        let mut player_guard = player.write().await;
        player_guard
            .set_voice_state(voice_state.clone())
            .await
            .expect("Failed to set voice state");

        // Verify voice state is set
        assert!(player_guard.voice_state().is_some());
        if let Some(vs) = player_guard.voice_state() {
            assert_eq!(vs.token, voice_state.token);
            assert_eq!(vs.endpoint, voice_state.endpoint);
            assert_eq!(vs.session_id, voice_state.session_id);
        }
    }
}

/// Test filter application
#[tokio::test]
async fn test_filter_application() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;

    let filters = Filters {
        volume: Omissible::Present(1.5),
        equalizer: Omissible::Present(vec![
            lavalink_rust::protocol::filters::Band { band: 0, gain: 0.2 },
            lavalink_rust::protocol::filters::Band {
                band: 1,
                gain: 0.15,
            },
        ]),
        karaoke: Omissible::Present(None),
        timescale: Omissible::Present(None),
        tremolo: Omissible::Present(None),
        vibrato: Omissible::Present(None),
        rotation: Omissible::Present(None),
        distortion: Omissible::Present(None),
        channel_mix: Omissible::Present(None),
        low_pass: Omissible::Present(None),
        plugin_filters: HashMap::new(),
    };

    // Apply filters
    {
        let mut player_guard = player.write().await;
        let result = player_guard.set_filters(filters.clone()).await;
        assert!(result.is_ok());

        // Verify filters are applied
        match &player_guard.filters.volume {
            Omissible::Present(vol) => assert_eq!(*vol, 1.5),
            _ => panic!("Volume filter not applied"),
        }
    }
}

/// Test seeking functionality
#[tokio::test]
async fn test_seeking() {
    let player_manager = Arc::new(PlayerManager::new());
    let guild_id = "123456789".to_string();
    let session_id = "session123".to_string();

    let player = player_manager
        .get_or_create_player(guild_id.clone(), session_id.clone())
        .await;
    let track = create_mock_track();

    // Play track first
    {
        let mut player_guard = player.write().await;
        player_guard
            .play_track(track, None, None)
            .await
            .expect("Failed to play track");
    }

    // Test seeking
    {
        let mut player_guard = player.write().await;
        let seek_position = 30000; // 30 seconds
        let result = player_guard.seek(seek_position).await;
        assert!(result.is_ok());

        // Position should be updated (approximately)
        assert!(player_guard.position >= seek_position - 1000); // Allow 1s tolerance
    }
}

/// Test player state serialization
#[tokio::test]
async fn test_player_state_serialization() {
    let player_state = create_mock_player_state();

    // Test serialization
    let serialized =
        serde_json::to_string(&player_state).expect("Failed to serialize player state");
    let deserialized: PlayerState =
        serde_json::from_str(&serialized).expect("Failed to deserialize player state");

    assert_eq!(player_state.position, deserialized.position);
    assert_eq!(player_state.connected, deserialized.connected);
    assert_eq!(player_state.ping, deserialized.ping);
}

/// Test concurrent player operations
#[tokio::test]
async fn test_concurrent_player_operations() {
    let player_manager = Arc::new(PlayerManager::new());
    let mut handles = Vec::new();

    // Create multiple players concurrently
    for i in 0..5 {
        let manager = player_manager.clone();
        let guild_id = format!("guild_{}", i);

        let handle = tokio::spawn(async move {
            let session_id = format!("session_{}", i);
            let player = manager
                .get_or_create_player(guild_id.clone(), session_id)
                .await;

            // Perform some operations
            {
                let mut player_guard = player.write().await;
                player_guard
                    .set_volume(50 + i * 10)
                    .await
                    .expect("Failed to set volume");
                player_guard
                    .set_paused(i % 2 == 0)
                    .await
                    .expect("Failed to set pause state");
            }

            guild_id
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let results = futures::future::join_all(handles).await;

    // Verify all players were created by checking each one individually
    for i in 0..5 {
        let guild_id = format!("guild_{}", i);
        let player = player_manager.get_player(&guild_id).await;
        assert!(player.is_some(), "Player for guild_{} should exist", i);
    }

    // Verify each player has correct state
    for (i, result) in results.into_iter().enumerate() {
        let guild_id = result.expect("Task should complete");
        let player = player_manager.get_player(&guild_id).await.expect("Player should exist");

        let player_guard = player.read().await;
        assert_eq!(player_guard.volume, (50 + i * 10) as u8);
        assert_eq!(player_guard.paused, i % 2 == 0);
    }
}

/// Test track end reason handling
#[tokio::test]
async fn test_track_end_reasons() {
    // Test serialization of different end reasons
    let reasons = vec![
        TrackEndReason::Finished,
        TrackEndReason::LoadFailed,
        TrackEndReason::Stopped,
        TrackEndReason::Replaced,
        TrackEndReason::Cleanup,
    ];

    for reason in reasons {
        let serialized =
            serde_json::to_string(&reason).expect("Failed to serialize track end reason");
        let deserialized: TrackEndReason =
            serde_json::from_str(&serialized).expect("Failed to deserialize track end reason");
        assert_eq!(reason, deserialized);
    }
}

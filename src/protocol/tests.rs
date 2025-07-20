// Protocol serialization and compatibility tests
// These tests ensure compatibility with the Java Lavalink implementation

use super::*;
use crate::test_utils::*;

#[cfg(test)]
mod info_tests {
    use super::*;

    #[test]
    fn test_info_serialization() {
        let json = r#"
        {
          "version": {
            "semver": "4.0.0",
            "major": 4,
            "minor": 0,
            "patch": 0,
            "preRelease": null,
            "build": null
          },
          "buildTime": 1664223916812,
          "git": {
            "branch": "master",
            "commit": "85c5ab5",
            "commitTime": 1664223916812
          },
          "jvm": "N/A - Rust",
          "lavaplayer": "N/A - Native Rust",
          "sourceManagers": [
            "youtube",
            "soundcloud"
          ],
          "filters": [
            "equalizer",
            "karaoke",
            "timescale",
            "channelMix"
          ],
          "plugins": []
        }
        "#;

        let info: Info = test_json_roundtrip(json).expect("Failed to deserialize Info");

        assert_eq!(info.version.semver, "4.0.0");
        assert_eq!(info.version.major, 4);
        assert_eq!(info.version.minor, 0);
        assert_eq!(info.version.patch, 0);
        assert_eq!(info.build_time, 1_664_223_916_812);
        assert_eq!(info.git.branch, "master");
        assert_eq!(info.git.commit, "85c5ab5");
        assert_eq!(info.git.commit_time, 1_664_223_916_812);
        assert_eq!(info.jvm, "N/A - Rust");
        assert_eq!(info.lavaplayer, "N/A - Native Rust");
        assert!(info.source_managers.contains(&"youtube".to_string()));
        assert!(info.source_managers.contains(&"soundcloud".to_string()));
        assert!(info.filters.contains(&"equalizer".to_string()));
        assert!(info.filters.contains(&"karaoke".to_string()));
    }

    #[test]
    fn test_version_serialization() {
        let json = r#"
        {
          "semver": "4.0.0-rc.1",
          "major": 4,
          "minor": 0,
          "patch": 0,
          "preRelease": "rc.1",
          "build": "123"
        }
        "#;

        let version: Version = test_json_roundtrip(json).expect("Failed to deserialize Version");

        assert_eq!(version.semver, "4.0.0-rc.1");
        assert_eq!(version.major, 4);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release, Some("rc.1".to_string()));
        assert_eq!(version.build, Some("123".to_string()));
    }

    #[test]
    fn test_git_info_serialization() {
        let json = r#"
        {
          "branch": "feature/rust-migration",
          "commit": "abc123def456",
          "commitTime": 1700000000000
        }
        "#;

        let git_info: Git = test_json_roundtrip(json).expect("Failed to deserialize Git");

        assert_eq!(git_info.branch, "feature/rust-migration");
        assert_eq!(git_info.commit, "abc123def456");
        assert_eq!(git_info.commit_time, 1_700_000_000_000);
    }
}

#[cfg(test)]
mod track_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_track_serialization() {
        let json = r#"
        {
          "encoded": "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==",
          "info": {
            "identifier": "dQw4w9WgXcQ",
            "isSeekable": true,
            "author": "RickAstleyVEVO",
            "length": 212000,
            "isStream": false,
            "position": 0,
            "title": "Rick Astley - Never Gonna Give You Up",
            "uri": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "sourceName": "youtube",
            "artworkUrl": null,
            "isrc": null
          },
          "pluginInfo": {},
          "userData": {}
        }
        "#;

        let track: Track = test_json_roundtrip(json).expect("Failed to deserialize Track");

        assert_eq!(track.info.identifier, "dQw4w9WgXcQ");
        assert!(track.info.is_seekable);
        assert_eq!(track.info.author, "RickAstleyVEVO");
        assert_eq!(track.info.length, 212_000);
        assert!(!track.info.is_stream);
        assert_eq!(track.info.position, 0);
        assert_eq!(track.info.title, "Rick Astley - Never Gonna Give You Up");
        assert_eq!(
            track.info.uri,
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string())
        );
        assert_eq!(track.info.source_name, "youtube");
        assert_eq!(track.info.artwork_url, None);
        assert_eq!(track.info.isrc, None);
    }

    #[test]
    fn test_track_decode() {
        let encoded = "QAAAjQIAJVJpY2sgQXN0bGV5IC0gTmV2ZXIgR29ubmEgR2l2ZSBZb3UgVXAADlJpY2tBc3RsZXlWRVZPAAAAAAADPCAAC2RRdzR3OVdnWGNRAAEAK2h0dHBzOi8vd3d3LnlvdXR1YmUuY29tL3dhdGNoP3Y9ZFF3NHc5V2dYY1EAB3lvdXR1YmUAAAAAAAAAAA==";

        // This test validates that we can decode base64 encoded tracks
        // For now, we'll test the structure rather than actual decoding
        let track = Track {
            encoded: encoded.to_string(),
            info: TrackInfo {
                identifier: "dQw4w9WgXcQ".to_string(),
                is_seekable: true,
                author: "RickAstleyVEVO".to_string(),
                length: 212_000,
                is_stream: false,
                position: 0,
                title: "Rick Astley - Never Gonna Give You Up".to_string(),
                uri: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
                artwork_url: None,
                isrc: None,
                source_name: "youtube".to_string(),
            },
            plugin_info: HashMap::new(),
            user_data: HashMap::new(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&track).expect("Failed to serialize track");
        let deserialized: Track =
            serde_json::from_str(&serialized).expect("Failed to deserialize track");

        assert_eq!(track.encoded, deserialized.encoded);
        assert_eq!(track.info.identifier, deserialized.info.identifier);
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;
    use crate::protocol::messages::*;

    #[test]
    fn test_ready_message_serialization() {
        let json = r#"
        {
          "op": "ready",
          "resumed": false,
          "sessionId": "test-session-123"
        }
        "#;

        let message: Message =
            test_json_roundtrip(json).expect("Failed to deserialize Ready message");

        match message {
            Message::Ready {
                resumed,
                session_id,
            } => {
                assert!(!resumed);
                assert_eq!(session_id, "test-session-123");
            }
            _ => panic!("Expected Ready message"),
        }
    }

    #[test]
    fn test_player_update_message_serialization() {
        let json = r#"
        {
          "op": "playerUpdate",
          "guildId": "123456789",
          "state": {
            "time": 1000,
            "position": 1000,
            "connected": true,
            "ping": 10
          }
        }
        "#;

        let message: Message =
            test_json_roundtrip(json).expect("Failed to deserialize PlayerUpdate message");

        match message {
            Message::PlayerUpdate { guild_id, state } => {
                assert_eq!(guild_id, "123456789");
                // Note: time is a DateTime, so we just check it's present
                assert!(state.time.timestamp_millis() > 0);
                assert_eq!(state.position, 1000);
                assert!(state.connected);
                assert_eq!(state.ping, 10);
            }
            _ => panic!("Expected PlayerUpdate message"),
        }
    }

    #[test]
    fn test_track_start_event_serialization() {
        let json = r#"
        {
          "op": "event",
          "type": "TrackStartEvent",
          "guildId": "123456789",
          "track": {
            "encoded": "test-encoded-track",
            "info": {
              "identifier": "test-id",
              "isSeekable": true,
              "author": "Test Author",
              "length": 180000,
              "isStream": false,
              "position": 0,
              "title": "Test Track",
              "uri": "https://example.com/track",
              "sourceName": "test",
              "artworkUrl": null,
              "isrc": null
            },
            "pluginInfo": {},
            "userData": {}
          }
        }
        "#;

        let message: Message =
            test_json_roundtrip(json).expect("Failed to deserialize TrackStartEvent");

        match message {
            Message::Event(boxed_event) => {
                if let Event::TrackStart { guild_id, track } = boxed_event.as_ref() {
                    assert_eq!(guild_id, "123456789");
                    assert_eq!(track.info.identifier, "test-id");
                    assert_eq!(track.info.title, "Test Track");
                } else {
                    panic!("Expected TrackStart event");
                }
            }
            _ => panic!("Expected TrackStartEvent"),
        }
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_response_serialization() {
        let json = r#"
        {
          "timestamp": 1667857581613,
          "status": 404,
          "error": "Not Found",
          "trace": "...",
          "message": "Session not found",
          "path": "/v4/sessions/test-session/players/123456789"
        }
        "#;

        let error: ErrorResponse =
            test_json_roundtrip(json).expect("Failed to deserialize ErrorResponse");

        assert_eq!(error.timestamp, 1_667_857_581_613);
        assert_eq!(error.status, 404);
        assert_eq!(error.error, "Not Found");
        assert_eq!(error.trace, Some("...".to_string()));
        assert_eq!(error.message, Some("Session not found".to_string()));
        assert_eq!(error.path, "/v4/sessions/test-session/players/123456789");
    }

    #[test]
    fn test_exception_serialization() {
        let json = r#"
        {
          "message": "Track loading failed",
          "severity": "common",
          "cause": "Network timeout"
        }
        "#;

        let exception: Exception =
            test_json_roundtrip(json).expect("Failed to deserialize Exception");

        assert_eq!(exception.message, Some("Track loading failed".to_string()));
        assert_eq!(exception.severity, Severity::Common);
        assert_eq!(exception.cause, "Network timeout");
    }
}

#[cfg(test)]
mod omissible_tests {
    use super::*;

    #[test]
    fn test_omissible_present() {
        let json = r#"{"value": "test"}"#;

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestStruct {
            value: Omissible<String>,
        }

        let test_struct: TestStruct = serde_json::from_str(json).expect("Failed to deserialize");

        match test_struct.value {
            Omissible::Present(val) => assert_eq!(val, "test"),
            _ => panic!("Expected Present value"),
        }
    }

    #[test]
    fn test_omissible_null() {
        let json = r#"{"value": null}"#;

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestStruct {
            value: Omissible<String>,
        }

        let test_struct: TestStruct = serde_json::from_str(json).expect("Failed to deserialize");

        match test_struct.value {
            Omissible::Null => {}
            _ => panic!("Expected Null value"),
        }
    }

    #[test]
    fn test_omissible_omitted() {
        let json = r"{}";

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestStruct {
            #[serde(default)]
            value: Omissible<String>,
        }

        let test_struct: TestStruct = serde_json::from_str(json).expect("Failed to deserialize");

        match test_struct.value {
            Omissible::Omitted => {}
            _ => panic!("Expected Omitted value"),
        }
    }
}

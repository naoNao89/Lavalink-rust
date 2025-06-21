use super::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Player state information
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub time: Timestamp,
    pub position: u64,
    pub connected: bool,
    pub ping: i32,
}

impl Serialize for PlayerState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PlayerState", 4)?;
        state.serialize_field("time", &self.time.timestamp_millis())?;
        state.serialize_field("position", &self.position)?;
        state.serialize_field("connected", &self.connected)?;
        state.serialize_field("ping", &self.ping)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PlayerState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Time,
            Position,
            Connected,
            Ping,
        }

        struct PlayerStateVisitor;

        impl<'de> Visitor<'de> for PlayerStateVisitor {
            type Value = PlayerState;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PlayerState")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PlayerState, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut time = None;
                let mut position = None;
                let mut connected = None;
                let mut ping = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Time => {
                            if time.is_some() {
                                return Err(de::Error::duplicate_field("time"));
                            }
                            let timestamp_millis: i64 = map.next_value()?;
                            time = Some(
                                chrono::DateTime::from_timestamp_millis(timestamp_millis)
                                    .ok_or_else(|| de::Error::custom("invalid timestamp"))?,
                            );
                        }
                        Field::Position => {
                            if position.is_some() {
                                return Err(de::Error::duplicate_field("position"));
                            }
                            position = Some(map.next_value()?);
                        }
                        Field::Connected => {
                            if connected.is_some() {
                                return Err(de::Error::duplicate_field("connected"));
                            }
                            connected = Some(map.next_value()?);
                        }
                        Field::Ping => {
                            if ping.is_some() {
                                return Err(de::Error::duplicate_field("ping"));
                            }
                            ping = Some(map.next_value()?);
                        }
                    }
                }

                let time = time.ok_or_else(|| de::Error::missing_field("time"))?;
                let position = position.ok_or_else(|| de::Error::missing_field("position"))?;
                let connected = connected.ok_or_else(|| de::Error::missing_field("connected"))?;
                let ping = ping.ok_or_else(|| de::Error::missing_field("ping"))?;

                Ok(PlayerState {
                    time,
                    position,
                    connected,
                    ping,
                })
            }
        }

        const FIELDS: &[&str] = &["time", "position", "connected", "ping"];
        deserializer.deserialize_struct("PlayerState", FIELDS, PlayerStateVisitor)
    }
}

impl PlayerState {
    /// Create a disconnected player state
    pub fn disconnected() -> Self {
        Self {
            time: chrono::Utc::now(),
            position: 0,
            connected: false,
            ping: -1,
        }
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::disconnected()
    }
}

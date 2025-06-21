use super::Omissible;
use serde::{Deserialize, Serialize};

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub resuming: bool,
    pub timeout: u64,
}

/// Session update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub resuming: Omissible<bool>,
    #[serde(skip_serializing_if = "Omissible::is_omitted", default)]
    pub timeout: Omissible<u64>,
}

impl Session {
    /// Create a non-resuming session
    pub fn non_resuming() -> Self {
        Self {
            resuming: false,
            timeout: 0,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::non_resuming()
    }
}

impl SessionUpdate {
    /// Create a new session update
    pub fn new() -> Self {
        Self {
            resuming: Omissible::Omitted,
            timeout: Omissible::Omitted,
        }
    }
}

impl Default for SessionUpdate {
    fn default() -> Self {
        Self::new()
    }
}

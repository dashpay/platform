use crate::tokens::status::TokenStatus;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialOrd, PartialEq, Eq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum TokenEmergencyAction {
    #[default]
    Pause = 0,
    Resume = 1,
}

impl TokenEmergencyAction {
    pub fn paused(&self) -> bool {
        matches!(self, TokenEmergencyAction::Pause)
    }
    pub fn resulting_status(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<TokenStatus, ProtocolError> {
        match self {
            TokenEmergencyAction::Pause => TokenStatus::new(true, platform_version),
            TokenEmergencyAction::Resume => TokenStatus::new(false, platform_version),
        }
    }
}

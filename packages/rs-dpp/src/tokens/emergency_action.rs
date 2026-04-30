use crate::tokens::status::TokenStatus;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialOrd, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum TokenEmergencyAction {
    #[default]
    Pause = 0,
    Resume = 1,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenEmergencyAction {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenEmergencyAction {}

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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_tokenemergencyaction {
    use super::*;

    #[test]
    fn json_round_trip_tokenemergencyaction() {
        use crate::serialization::JsonConvertible;
        let original = TokenEmergencyAction::default();
        let json = original.to_json().expect("to_json");
        let recovered = TokenEmergencyAction::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_tokenemergencyaction() {
        use crate::serialization::ValueConvertible;
        let original = TokenEmergencyAction::default();
        let value = original.to_object().expect("to_object");
        let recovered = TokenEmergencyAction::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

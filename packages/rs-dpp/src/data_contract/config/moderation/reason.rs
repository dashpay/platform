use crate::consensus::basic::contract_moderation::ContractModerationReasonTooLongError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};

/// Why a moderator banned or suspended an identity. Every ban and every suspension carries
/// one, and it is stored with the entry, so whoever reads the list reads the reason.
///
/// Nothing checks what a moderator writes: the text is free, and so is the code.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationReason {
    /// Reserved for the ban codes a contract may declare in a later protocol version. No
    /// contract declares any today, so this is expected to be `None`. A value is accepted and
    /// stored as written, and is not checked against anything.
    #[serde(default)]
    pub code: Option<u16>,
    /// Free text, at most `SystemLimits::max_contract_moderation_reason_length` bytes of
    /// UTF-8. May be empty.
    pub text: String,
}

impl ContractModerationReason {
    /// A reason without a code.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            code: None,
            text: text.into(),
        }
    }

    /// Checks the length of the text. The code is not checked.
    pub fn validate(&self, platform_version: &PlatformVersion) -> SimpleConsensusValidationResult {
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length;
        if self.text.len() > max_length as usize {
            return SimpleConsensusValidationResult::new_with_error(
                ContractModerationReasonTooLongError::new(self.text.len() as u64, max_length)
                    .into(),
            );
        }
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerationReason {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    #[test]
    fn should_round_trip_through_json_with_and_without_a_code() {
        let reason = ContractModerationReason {
            code: Some(7),
            text: "spam".to_string(),
        };
        let json = serde_json::to_value(&reason).expect("to json");
        assert_eq!(json, serde_json::json!({"code": 7, "text": "spam"}));
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(json).expect("from json"),
            reason
        );

        // The code may be left out
        assert_eq!(
            serde_json::from_value::<ContractModerationReason>(serde_json::json!({"text": ""}))
                .expect("from json"),
            ContractModerationReason::from_text("")
        );
    }

    #[test]
    fn should_refuse_an_unknown_field() {
        serde_json::from_value::<ContractModerationReason>(
            serde_json::json!({"text": "spam", "note": "x"}),
        )
        .expect_err("unknown field");
    }

    #[test]
    fn should_accept_any_code_and_a_text_up_to_the_limit() {
        let platform_version = PlatformVersion::latest();
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize;

        let reason = ContractModerationReason {
            code: Some(u16::MAX),
            text: "é".repeat(max_length / 2),
        };
        assert_eq!(reason.text.len(), max_length);
        assert!(reason.validate(platform_version).is_valid());
        assert!(ContractModerationReason::default()
            .validate(platform_version)
            .is_valid());
    }

    #[test]
    fn should_refuse_a_text_over_the_limit_counted_in_bytes() {
        let platform_version = PlatformVersion::latest();
        let max_length = platform_version
            .system_limits
            .max_contract_moderation_reason_length;

        // One character under the limit in characters, over it in bytes
        let reason = ContractModerationReason::from_text("é".repeat(max_length as usize / 2 + 1));
        let result = reason.validate(platform_version);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractModerationReasonTooLongError(error))]
                if error.length() == max_length as u64 + 2 && error.max_length() == max_length
        ));
    }
}

use bincode::{Decode, Encode};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    TryFromPrimitive,
    IntoPrimitive,
    Encode,
    Decode,
    Default,
)]
pub enum StateTransitionType {
    #[default]
    DataContractCreate = 0,
    Batch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
    IdentityCreditTransfer = 7,
    MasternodeVote = 8,
    IdentityCreditTransferToAddresses = 9,
    IdentityCreateFromAddresses = 10,
    IdentityTopUpFromAddresses = 11,
    AddressFundsTransfer = 12,
    AddressFundingFromAssetLock = 13,
    AddressCreditWithdrawal = 14,
    Shield = 15,
    ShieldedTransfer = 16,
    Unshield = 17,
    ShieldFromAssetLock = 18,
    ShieldedWithdrawal = 19,
    IdentityCreateFromShieldedPool = 20,
}

impl std::fmt::Display for StateTransitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_default_is_data_contract_create() {
        let default = StateTransitionType::default();
        assert_eq!(default, StateTransitionType::DataContractCreate);
    }

    #[test]
    fn test_display_all_variants() {
        let cases: Vec<(StateTransitionType, &str)> = vec![
            (
                StateTransitionType::DataContractCreate,
                "DataContractCreate",
            ),
            (StateTransitionType::Batch, "Batch"),
            (StateTransitionType::IdentityCreate, "IdentityCreate"),
            (StateTransitionType::IdentityTopUp, "IdentityTopUp"),
            (
                StateTransitionType::DataContractUpdate,
                "DataContractUpdate",
            ),
            (StateTransitionType::IdentityUpdate, "IdentityUpdate"),
            (
                StateTransitionType::IdentityCreditWithdrawal,
                "IdentityCreditWithdrawal",
            ),
            (
                StateTransitionType::IdentityCreditTransfer,
                "IdentityCreditTransfer",
            ),
            (StateTransitionType::MasternodeVote, "MasternodeVote"),
            (
                StateTransitionType::IdentityCreditTransferToAddresses,
                "IdentityCreditTransferToAddresses",
            ),
            (
                StateTransitionType::IdentityCreateFromAddresses,
                "IdentityCreateFromAddresses",
            ),
            (
                StateTransitionType::IdentityTopUpFromAddresses,
                "IdentityTopUpFromAddresses",
            ),
            (
                StateTransitionType::AddressFundsTransfer,
                "AddressFundsTransfer",
            ),
            (
                StateTransitionType::AddressFundingFromAssetLock,
                "AddressFundingFromAssetLock",
            ),
            (
                StateTransitionType::AddressCreditWithdrawal,
                "AddressCreditWithdrawal",
            ),
            (StateTransitionType::Shield, "Shield"),
            (StateTransitionType::ShieldedTransfer, "ShieldedTransfer"),
            (StateTransitionType::Unshield, "Unshield"),
            (
                StateTransitionType::ShieldFromAssetLock,
                "ShieldFromAssetLock",
            ),
            (
                StateTransitionType::ShieldedWithdrawal,
                "ShieldedWithdrawal",
            ),
            (
                StateTransitionType::IdentityCreateFromShieldedPool,
                "IdentityCreateFromShieldedPool",
            ),
        ];
        for (variant, expected) in cases {
            assert_eq!(
                format!("{}", variant),
                expected,
                "Display mismatch for {:?}",
                variant
            );
        }
    }

    #[test]
    fn test_try_from_u8_all_valid() {
        let pairs: Vec<(u8, StateTransitionType)> = vec![
            (0, StateTransitionType::DataContractCreate),
            (1, StateTransitionType::Batch),
            (2, StateTransitionType::IdentityCreate),
            (3, StateTransitionType::IdentityTopUp),
            (4, StateTransitionType::DataContractUpdate),
            (5, StateTransitionType::IdentityUpdate),
            (6, StateTransitionType::IdentityCreditWithdrawal),
            (7, StateTransitionType::IdentityCreditTransfer),
            (8, StateTransitionType::MasternodeVote),
            (9, StateTransitionType::IdentityCreditTransferToAddresses),
            (10, StateTransitionType::IdentityCreateFromAddresses),
            (11, StateTransitionType::IdentityTopUpFromAddresses),
            (12, StateTransitionType::AddressFundsTransfer),
            (13, StateTransitionType::AddressFundingFromAssetLock),
            (14, StateTransitionType::AddressCreditWithdrawal),
            (15, StateTransitionType::Shield),
            (16, StateTransitionType::ShieldedTransfer),
            (17, StateTransitionType::Unshield),
            (18, StateTransitionType::ShieldFromAssetLock),
            (19, StateTransitionType::ShieldedWithdrawal),
            (20, StateTransitionType::IdentityCreateFromShieldedPool),
        ];
        for (val, expected) in pairs {
            let result = StateTransitionType::try_from(val).unwrap();
            assert_eq!(result, expected, "TryFrom<u8> failed for {}", val);
        }
    }

    #[test]
    fn test_try_from_u8_invalid() {
        assert!(StateTransitionType::try_from(21u8).is_err());
        assert!(StateTransitionType::try_from(255u8).is_err());
    }

    #[test]
    fn test_into_u8_roundtrip() {
        let all_variants = vec![
            StateTransitionType::DataContractCreate,
            StateTransitionType::Batch,
            StateTransitionType::IdentityCreate,
            StateTransitionType::IdentityTopUp,
            StateTransitionType::DataContractUpdate,
            StateTransitionType::IdentityUpdate,
            StateTransitionType::IdentityCreditWithdrawal,
            StateTransitionType::IdentityCreditTransfer,
            StateTransitionType::MasternodeVote,
            StateTransitionType::IdentityCreditTransferToAddresses,
            StateTransitionType::IdentityCreateFromAddresses,
            StateTransitionType::IdentityTopUpFromAddresses,
            StateTransitionType::AddressFundsTransfer,
            StateTransitionType::AddressFundingFromAssetLock,
            StateTransitionType::AddressCreditWithdrawal,
            StateTransitionType::Shield,
            StateTransitionType::ShieldedTransfer,
            StateTransitionType::Unshield,
            StateTransitionType::ShieldFromAssetLock,
            StateTransitionType::ShieldedWithdrawal,
            StateTransitionType::IdentityCreateFromShieldedPool,
        ];
        for variant in all_variants {
            let val: u8 = variant.into();
            let roundtripped = StateTransitionType::try_from(val).unwrap();
            assert_eq!(variant, roundtripped, "roundtrip failed for {:?}", variant);
        }
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_clone_and_copy() {
        let original = StateTransitionType::MasternodeVote;
        let cloned = original.clone();
        let copied = original;
        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    #[test]
    fn test_equality() {
        assert_eq!(
            StateTransitionType::DataContractCreate,
            StateTransitionType::DataContractCreate
        );
        assert_ne!(
            StateTransitionType::DataContractCreate,
            StateTransitionType::Batch
        );
    }
}

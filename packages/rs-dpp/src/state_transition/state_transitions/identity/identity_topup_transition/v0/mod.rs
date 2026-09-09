mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;

#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;

use crate::prelude::{Identifier, UserFeeIncrease};

use crate::ProtocolError;

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityTopUpTransitionV0 {
    // Own ST fields
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::{
        StateTransitionHasUserFeeIncrease, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use platform_value::BinaryData;

    fn make_topup_v0() -> IdentityTopUpTransitionV0 {
        IdentityTopUpTransitionV0 {
            asset_lock_proof: AssetLockProof::default(),
            identity_id: Identifier::random(),
            user_fee_increase: 2,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn test_default() {
        let t = IdentityTopUpTransitionV0::default();
        assert_eq!(t.user_fee_increase, 0);
        assert_eq!(t.identity_id, Identifier::default());
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_topup_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityTopUp
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
    }

    #[test]
    fn test_unique_identifiers() {
        let t = make_topup_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // With a default AssetLockProof, create_identifier fails, so the
        // implementation returns a default empty string as fallback
    }

    #[test]
    fn test_owner_id() {
        let t = make_topup_v0();
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_topup_v0();
        assert_eq!(t.user_fee_increase(), 2);
        t.set_user_fee_increase(10);
        assert_eq!(t.user_fee_increase(), 10);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_topup_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5, 6]);
        assert_eq!(t.signature().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn test_into_state_transition() {
        use crate::state_transition::StateTransition;
        let t = make_topup_v0();
        let st: StateTransition = t.into();
        match st {
            StateTransition::IdentityTopUp(_) => {}
            _ => panic!("expected IdentityTopUp"),
        }
    }

    #[test]
    fn test_asset_lock_proved() {
        use crate::identity::state_transition::AssetLockProved;
        let mut t = make_topup_v0();
        let proof = t.asset_lock_proof().clone();
        assert_eq!(&proof, t.asset_lock_proof());
        t.set_asset_lock_proof(AssetLockProof::default())
            .expect("should set proof");
    }
}

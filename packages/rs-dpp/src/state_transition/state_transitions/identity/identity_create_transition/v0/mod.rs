#[cfg(feature = "json-conversion")]
mod json_conversion;
mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use std::convert::TryFrom;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::prelude::{Identifier, UserFeeIncrease};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::state_transition::AssetLockProved;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::version::PlatformVersion;
use crate::ProtocolError;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateTransitionV0Inner")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside of bincode_derive
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityCreateTransitionV0 {
    // When signing, we don't sign the signatures for keys
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub asset_lock_proof: AssetLockProof,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[cfg_attr(feature = "serde-conversion", serde(skip))]
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[cfg_attr(
    feature = "serde-conversion",
    derive(Deserialize),
    serde(rename_all = "camelCase")
)]
struct IdentityCreateTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    user_fee_increase: UserFeeIncrease,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionV0Inner> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionV0Inner {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
        } = value;
        let identity_id = asset_lock_proof.create_identifier()?;
        Ok(Self {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
            identity_id,
        })
    }
}

impl IdentityCreateTransitionV0 {
    pub fn try_from_identity_v0(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<Self, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransitionV0::default();

        let public_keys = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.into())
            .collect::<Vec<IdentityPublicKeyInCreation>>();
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition.set_asset_lock_proof(asset_lock_proof)?;

        Ok(identity_create_transition)
    }

    pub fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_identity_create_transition
        {
            0 => Self::try_from_identity_v0(identity, asset_lock_proof),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateTransitionV0::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionHasUserFeeIncrease, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use platform_value::BinaryData;

    fn make_create_v0() -> IdentityCreateTransitionV0 {
        IdentityCreateTransitionV0 {
            public_keys: vec![],
            asset_lock_proof: AssetLockProof::default(),
            user_fee_increase: 0,
            signature: [0u8; 65].to_vec().into(),
            identity_id: Identifier::random(),
        }
    }

    #[test]
    fn test_default() {
        let t = IdentityCreateTransitionV0::default();
        assert_eq!(t.user_fee_increase, 0);
        assert!(t.public_keys.is_empty());
        assert!(t.signature.is_empty());
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_create_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
    }

    #[test]
    fn test_unique_identifiers() {
        let t = make_create_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_owner_id() {
        let t = make_create_v0();
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_create_v0();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(7);
        assert_eq!(t.user_fee_increase(), 7);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_create_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_into_state_transition() {
        use crate::state_transition::StateTransition;
        let t = make_create_v0();
        let st: StateTransition = t.into();
        match st {
            StateTransition::IdentityCreate(_) => {}
            _ => panic!("expected IdentityCreate"),
        }
    }

    #[test]
    fn test_accessors() {
        let mut t = make_create_v0();
        assert!(t.public_keys().is_empty());
        assert_eq!(t.identity_id(), t.identity_id);

        // Test set_public_keys and add_public_keys
        t.set_public_keys(vec![]);
        assert!(t.public_keys().is_empty());
    }

    #[test]
    fn test_to_object_produces_value() {
        use crate::state_transition::StateTransitionValueConvert;
        let t = make_create_v0();
        let obj = t.to_object(false).expect("to_object should work");
        assert!(obj.is_map());
    }

    #[test]
    fn test_value_conversion_skip_signature() {
        use crate::state_transition::StateTransitionValueConvert;
        let t = make_create_v0();
        let obj = t.to_object(true).expect("to_object should work");
        let map = obj.into_btree_string_map().expect("should be a map");
        assert!(!map.contains_key("signature"));
    }

    #[test]
    fn test_to_cleaned_object() {
        use crate::state_transition::StateTransitionValueConvert;
        let t = make_create_v0();
        let obj = t.to_cleaned_object(false).expect("should work");
        assert!(obj.is_map());
    }
}

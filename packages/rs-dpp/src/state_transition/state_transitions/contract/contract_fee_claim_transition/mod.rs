//! Pays out one of the two fee pots of a data contract (protocol version 14).
//!
//! A document type may charge a fixed fee in credits for actions on its documents (the
//! `actionFees` keyword). The `owner` parts collect in the contract's owner pot and the
//! `moderators` parts in its moderators pot. This transition names a pot and pays it out: the
//! owner pot to the contract owner, who alone may claim it, and the moderators pot, which any
//! member of the contract's moderation team may claim, in equal shares to the whole team. What
//! an equal split leaves over stays in the pot.
//!
//! A pot is paid out at most once per epoch; the two pots are independent. Signed with a
//! CRITICAL authentication key, and paid for by the claimant like any other transition.

pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use fields::*;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub use crate::data_contract::document_type::action_fees::ContractFeePot;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.contract_fee_claim_state_transition"
)]
pub enum ContractFeeClaimTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ContractFeeClaimTransitionV0),
}

impl ContractFeeClaimTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_fee_claim_state_transition
            .default_current_version
        {
            0 => Ok(ContractFeeClaimTransition::V0(
                ContractFeeClaimTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContractFeeClaimTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for ContractFeeClaimTransition {}

impl StateTransitionFieldTypes for ContractFeeClaimTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![OWNER_ID, DATA_CONTRACT_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make() -> ContractFeeClaimTransition {
        ContractFeeClaimTransition::V0(ContractFeeClaimTransitionV0 {
            owner_id: Identifier::random(),
            data_contract_id: Identifier::random(),
            identity_contract_nonce: 10,
            pot: ContractFeePot::Moderators,
            user_fee_increase: 2,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn should_create_the_default_version() {
        let t = ContractFeeClaimTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            ContractFeeClaimTransition::V0(_) => {}
        }
    }

    #[test]
    fn should_round_trip_through_bytes_for_every_pot() {
        for pot in [ContractFeePot::Owner, ContractFeePot::Moderators] {
            let mut t = make();
            t.set_pot(pot);
            let bytes = t.serialize_to_bytes().expect("should serialize");
            let restored = ContractFeeClaimTransition::deserialize_from_bytes_untrusted(&bytes)
                .expect("should deserialize");
            assert_eq!(t, restored);
            assert_eq!(restored.pot(), pot);
        }
    }

    #[test]
    fn should_report_its_type_and_identifiers() {
        let t = make();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ContractFeeClaim
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.data_contract_id()]);
        assert_eq!(t.unique_identifiers().len(), 1);
        let ContractFeeClaimTransition::V0(v0) = &t;
        assert_eq!(t.owner_id(), v0.owner_id);
        assert_eq!(t.pot(), ContractFeePot::Moderators);
    }

    #[test]
    fn should_expose_the_user_fee_increase_and_signature() {
        let mut t = make();
        assert_eq!(t.user_fee_increase(), 2);
        t.set_user_fee_increase(50);
        assert_eq!(t.user_fee_increase(), 50);
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2]));
        assert_eq!(t.signature().as_slice(), &[1, 2]);
        t.set_signature_bytes(vec![3, 4]);
        assert_eq!(t.signature().as_slice(), &[3, 4]);
    }

    #[test]
    fn should_expose_the_accessors() {
        let mut t = make();
        assert_eq!(t.identity_contract_nonce(), 10);
        t.set_identity_contract_nonce(20);
        assert_eq!(t.identity_contract_nonce(), 20);
        let contract_id = Identifier::random();
        t.set_data_contract_id(contract_id);
        assert_eq!(t.data_contract_id(), contract_id);
        let owner_id = Identifier::random();
        t.set_owner_id(owner_id);
        assert_eq!(t.owner_id(), owner_id);
    }

    #[test]
    fn should_list_its_field_types() {
        assert_eq!(
            ContractFeeClaimTransition::signature_property_paths().len(),
            2
        );
        assert_eq!(
            ContractFeeClaimTransition::identifiers_property_paths().len(),
            2
        );
        assert_eq!(ContractFeeClaimTransition::binary_property_paths().len(), 1);
    }

    #[test]
    fn should_require_the_credit_transfer_minimum_fee() {
        let t = make();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert_eq!(
            fee,
            LATEST_PLATFORM_VERSION
                .fee_version
                .state_transition_min_fees
                .credit_transfer
        );
        assert!(t
            .validate_estimated_fee(fee, LATEST_PLATFORM_VERSION)
            .expect("validation should work")
            .is_valid());
        assert!(!t
            .validate_estimated_fee(fee - 1, LATEST_PLATFORM_VERSION)
            .expect("validation should work")
            .is_valid());
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;

    use platform_value::{platform_value, BinaryData, Identifier};
    use serde_json::json;

    pub(crate) fn fixture() -> ContractFeeClaimTransition {
        ContractFeeClaimTransition::V0(ContractFeeClaimTransitionV0 {
            owner_id: Identifier::new([0x55; 32]),
            data_contract_id: Identifier::new([0x66; 32]),
            identity_contract_nonce: 17,
            pot: ContractFeePot::Moderators,
            user_fee_increase: 4,
            signature_public_key_id: 6,
            signature: BinaryData::new(vec![0xd4; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "ownerId": Identifier::new([0x55; 32]),
                "dataContractId": Identifier::new([0x66; 32]),
                "identityContractNonce": 17,
                "pot": "moderators",
                "userFeeIncrease": 4,
                "signaturePublicKeyId": 6,
                "signature": "1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NQ=",
            })
        );
        let recovered = ContractFeeClaimTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "ownerId": Identifier::new([0x55; 32]),
                "dataContractId": Identifier::new([0x66; 32]),
                "identityContractNonce": 17u64,
                "pot": "moderators",
                "userFeeIncrease": 4u16,
                "signaturePublicKeyId": 6u32,
                "signature": BinaryData::new(vec![0xd4; 65]),
            })
        );
        let recovered = ContractFeeClaimTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

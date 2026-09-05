mod identity_signed;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::address_funds::PlatformAddress;
use crate::identity::KeyID;
use std::collections::BTreeMap;

use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};

use crate::fee::Credits;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
#[derive(Default)]
pub struct IdentityCreditTransferToAddressesTransitionV0 {
    // Own ST fields
    pub identity_id: Identifier,
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_output_map_required_amount")
    )]
    pub recipient_addresses: BTreeMap<PlatformAddress, Credits>,
    pub nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use crate::address_funds::PlatformAddress;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::signer::Signer;
    use crate::identity::v0::IdentityV0;
    use crate::identity::{Identity, IdentityPublicKey};
    use crate::state_transition::identity_credit_transfer_to_addresses_transition::methods::IdentityCreditTransferToAddressesTransitionMethodsV0;
    use crate::ProtocolError;
    use async_trait::async_trait;
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use platform_value::Identifier;
    use rand::Rng;
    use std::fmt::Debug;

    fn test_identity_credit_transfer_to_addresses_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
        transition: T,
    ) where
        <T as PlatformSerializable>::Error: std::fmt::Debug,
    {
        let serialized = T::serialize_to_bytes(&transition).expect("expected to serialize");
        let deserialized =
            T::deserialize_from_bytes(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_identity_credit_transfer_to_addresses_transition1() {
        use crate::address_funds::PlatformAddress;
        use std::collections::BTreeMap;

        let mut rng = rand::thread_rng();
        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([1u8; 20]), rng.gen::<u64>());

        let transition = IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: Identifier::random(),
            recipient_addresses,
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_identity_credit_transfer_to_addresses_transition(transition);
    }

    fn make_transfer_to_addresses_v0() -> IdentityCreditTransferToAddressesTransitionV0 {
        use crate::address_funds::PlatformAddress;
        use std::collections::BTreeMap;
        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([5u8; 20]), 1_000_000);
        recipient_addresses.insert(PlatformAddress::P2pkh([6u8; 20]), 2_000_000);
        IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: Identifier::random(),
            recipient_addresses,
            nonce: 7,
            user_fee_increase: 3,
            signature_public_key_id: 2,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn state_transition_like_basics() {
        use crate::state_transition::{
            StateTransitionHasUserFeeIncrease, StateTransitionLike, StateTransitionOwned,
            StateTransitionType,
        };
        let mut t = make_transfer_to_addresses_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreditTransferToAddresses
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
        assert_eq!(t.owner_id(), t.identity_id);
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // unique_identifier contains nonce in hex
        assert!(ids[0].contains("7"));
        assert_eq!(t.user_fee_increase(), 3);
        t.set_user_fee_increase(77);
        assert_eq!(t.user_fee_increase(), 77);
    }

    #[test]
    fn identity_signed_requirements() {
        use crate::identity::{Purpose, SecurityLevel};
        use crate::state_transition::StateTransitionIdentitySigned;
        let mut t = make_transfer_to_addresses_v0();
        assert_eq!(t.signature_public_key_id(), 2);
        t.set_signature_public_key_id(42);
        assert_eq!(t.signature_public_key_id(), 42);
        assert_eq!(
            t.security_level_requirement(Purpose::TRANSFER),
            vec![SecurityLevel::CRITICAL]
        );
        assert_eq!(t.purpose_requirement(), vec![Purpose::TRANSFER]);
    }

    #[test]
    fn single_signed_behaviour() {
        use crate::state_transition::StateTransitionSingleSigned;
        use platform_value::BinaryData;
        let mut t = make_transfer_to_addresses_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn into_state_transition_wraps_correctly() {
        use crate::state_transition::StateTransition;
        let t = make_transfer_to_addresses_v0();
        let st: StateTransition = t.into();
        assert!(matches!(
            st,
            StateTransition::IdentityCreditTransferToAddresses(_)
        ));
    }

    #[derive(Debug)]
    struct UnreachableIdentitySigner;

    #[async_trait]
    impl Signer<IdentityPublicKey> for UnreachableIdentitySigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            panic!("sign should not run when protocol gating rejects the constructor")
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<crate::address_funds::AddressWitness, ProtocolError> {
            panic!(
                "sign_create_witness should not run when protocol gating rejects the constructor"
            )
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn try_from_identity_returns_not_active_before_structure_validation() {
        let mut low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();
        low_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_to_addresses_state_transition
            .basic_structure = None;
        let identity: Identity = IdentityV0 {
            id: Identifier::random(),
            public_keys: BTreeMap::<u32, IdentityPublicKey>::new(),
            balance: 0,
            revision: 0,
        }
        .into();
        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([1u8; 20]), 1);

        let result = IdentityCreditTransferToAddressesTransitionV0::try_from_identity(
            &identity,
            recipient_addresses,
            0,
            &UnreachableIdentitySigner,
            None,
            1,
            &low_version,
            None,
        )
        .await;

        assert!(matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    *boxed,
                    ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
                )
        ));
    }
}

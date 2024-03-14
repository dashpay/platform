use crate::serialization::PlatformDeserializable;
use crate::state_transition::StateTransition;
use crate::ProtocolError;

impl StateTransition {
    pub fn deserialize_many(raw_state_transitions: &[Vec<u8>]) -> Result<Vec<Self>, ProtocolError> {
        raw_state_transitions
            .iter()
            .map(|raw_state_transition| Self::deserialize_from_bytes(raw_state_transition))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::core_script::CoreScript;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::Identity;
    use crate::prelude::AssetLockProof;
    use crate::serialization::PlatformMessageSignable;
    use crate::serialization::Signable;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use crate::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV0,
    };
    use crate::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;
    use crate::state_transition::documents_batch_transition::{
        DocumentsBatchTransition, DocumentsBatchTransitionV0,
    };
    use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use crate::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
    use crate::state_transition::StateTransition;
    use crate::tests::fixtures::{
        get_data_contract_fixture, get_document_transitions_fixture,
        get_extended_documents_fixture_with_owner_id_from_contract,
        raw_instant_asset_lock_proof_fixture,
    };
    use crate::version::PlatformVersion;
    use crate::withdrawal::Pooling;
    use crate::{NativeBlsModule, ProtocolError};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use platform_version::TryIntoPlatformVersioned;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    #[cfg(feature = "random-identities")]
    fn identity_create_transition_ser_de() {
        let platform_version = LATEST_PLATFORM_VERSION;
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let asset_lock_proof = raw_instant_asset_lock_proof_fixture(None);

        let identity_create_transition = IdentityCreateTransition::V0(
            IdentityCreateTransitionV0::try_from_identity(
                &identity,
                AssetLockProof::Instant(asset_lock_proof),
                platform_version,
            )
            .expect("expected to make an identity create transition"),
        );

        let state_transition: StateTransition = identity_create_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn identity_topup_transition_ser_de() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let asset_lock_proof = raw_instant_asset_lock_proof_fixture(None);

        let identity_topup_transition = IdentityTopUpTransitionV0 {
            asset_lock_proof: AssetLockProof::Instant(asset_lock_proof),
            identity_id: identity.id(),
            user_fee_increase: 0,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = identity_topup_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn identity_update_transition_add_keys_ser_de() {
        let mut rng = StdRng::seed_from_u64(5);
        let (identity, mut keys): (Identity, BTreeMap<_, _>) =
            Identity::random_identity_with_main_keys_with_private_key(
                5,
                &mut rng,
                LATEST_PLATFORM_VERSION,
            )
            .expect("expected to get identity");
        let bls = NativeBlsModule;
        let add_public_keys_in_creation = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.into())
            .collect();
        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys: vec![],
            user_fee_increase: 0,
        };

        let key_signable_bytes = identity_update_transition
            .signable_bytes()
            .expect("expected to get signable bytes");

        identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(identity.public_keys().clone().into_values())
            .try_for_each(|(public_key_with_witness, public_key)| {
                if public_key.key_type().is_unique_key_type() {
                    let private_key = keys
                        .get(&public_key)
                        .expect("expected to have the private key");
                    let signature = key_signable_bytes
                        .as_slice()
                        .sign_by_private_key(private_key, public_key.key_type(), &bls)?
                        .into();
                    public_key_with_witness.set_signature(signature);
                }

                Ok::<(), ProtocolError>(())
            })
            .expect("expected to update keys");

        let (public_key, private_key) = keys.pop_first().unwrap();

        let mut state_transition: StateTransition = identity_update_transition.into();

        state_transition
            .sign_by_private_key(private_key.as_slice(), public_key.key_type(), &bls)
            .expect("expected to sign IdentityUpdateTransition");
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "state-transition-signing")]
    fn identity_update_transition_disable_keys_ser_de() {
        let mut rng = StdRng::seed_from_u64(5);
        let (identity, mut keys): (Identity, BTreeMap<_, _>) =
            Identity::random_identity_with_main_keys_with_private_key(
                5,
                &mut rng,
                LATEST_PLATFORM_VERSION,
            )
            .expect("expected to get identity");
        let bls = NativeBlsModule;
        let add_public_keys_in_creation = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.into())
            .collect();
        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys: vec![3, 4, 5],
            user_fee_increase: 0,
        };

        let key_signable_bytes = identity_update_transition
            .signable_bytes()
            .expect("expected to get signable bytes");

        identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(identity.public_keys().clone().into_values())
            .try_for_each(|(public_key_with_witness, public_key)| {
                if public_key.key_type().is_unique_key_type() {
                    let private_key = keys
                        .get(&public_key)
                        .expect("expected to have the private key");
                    let signature = key_signable_bytes
                        .as_slice()
                        .sign_by_private_key(private_key, public_key.key_type(), &bls)?
                        .into();
                    public_key_with_witness.set_signature(signature);
                }

                Ok::<(), ProtocolError>(())
            })
            .expect("expected to update keys");

        let (public_key, private_key) = keys.pop_first().unwrap();

        let mut state_transition: StateTransition = identity_update_transition.into();

        state_transition
            .sign_by_private_key(private_key.as_slice(), public_key.key_type(), &bls)
            .expect("expected to sign IdentityUpdateTransition");
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn identity_credit_withdrawal_transition_ser_de() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let identity_credit_withdrawal_transition = IdentityCreditWithdrawalTransitionV0 {
            identity_id: identity.id(),
            amount: 5000000,
            core_fee_per_byte: 34,
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = identity_credit_withdrawal_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn data_contract_create_ser_de() {
        let platform_version = LATEST_PLATFORM_VERSION;
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let created_data_contract = get_data_contract_fixture(
            Some(identity.id()),
            0,
            LATEST_PLATFORM_VERSION.protocol_version,
        );
        let data_contract_create_transition: DataContractCreateTransition = created_data_contract
            .try_into_platform_versioned(platform_version)
            .expect("expected to transform into a DataContractCreateTransition");
        let state_transition: StateTransition = data_contract_create_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn data_contract_update_ser_de() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let created_data_contract =
            get_data_contract_fixture(Some(identity.id()), 0, platform_version.protocol_version);
        let data_contract_update_transition =
            DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
                identity_contract_nonce: 1,
                data_contract: created_data_contract
                    .data_contract_owned()
                    .try_into_platform_versioned(platform_version)
                    .expect("expected a data contract"),
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: [1u8; 65].to_vec().into(),
            });
        let state_transition: StateTransition = data_contract_update_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn document_batch_transition_10_created_documents_ser_de() {
        let platform_version = PlatformVersion::latest();

        let mut nonces = BTreeMap::new();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let documents = get_extended_documents_fixture_with_owner_id_from_contract(
            &data_contract,
            platform_version.protocol_version,
        )
        .unwrap();
        let documents = documents
            .iter()
            .map(|extended_document| {
                let document = extended_document.document().clone();
                let data_contract = extended_document.data_contract();
                (
                    document,
                    data_contract
                        .document_type_for_name(extended_document.document_type_name())
                        .unwrap(),
                    *extended_document.entropy(),
                )
            })
            .collect::<Vec<_>>();
        let transitions = get_document_transitions_fixture(
            [(DocumentTransitionActionType::Create, documents)],
            &mut nonces,
        );
        let documents_batch_transition: DocumentsBatchTransition = DocumentsBatchTransitionV0 {
            owner_id: data_contract.owner_id(),
            transitions,
            ..Default::default()
        }
        .into();
        let state_transition: StateTransition = documents_batch_transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let recovered_state_transition = StateTransition::deserialize_from_bytes(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }
}

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
    use hex::ToHex;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use platform_value::string_encoding::Encoding;
    use crate::bls::native_bls::NativeBlsModule;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::identity::state_transition::AssetLockProved;
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
    use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
    use crate::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV1,
    };
    use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
    use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use crate::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
    use crate::state_transition::StateTransition;
    use crate::tests::fixtures::{
        get_data_contract_fixture, get_batched_transitions_fixture,
        get_extended_documents_fixture_with_owner_id_from_contract,
        raw_instant_asset_lock_proof_fixture,
    };
    use crate::version::PlatformVersion;
    use crate::withdrawal::Pooling;
    use crate::ProtocolError;
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use platform_version::TryIntoPlatformVersioned;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    /// Given mainnet transaction 6CDCC15AC4EC68DBB414EE0DA692DFE363A996A0F285423BEFC3A29F87948A0D,
    /// when deserialized, it should be identity create transition.
    fn should_build_identity_create_transition_from_mainnet_asset_lock_transaction() {
        const EXPECTED_STATE_TRANSITION_HASH: &str =
            "6CDCC15AC4EC68DBB414EE0DA692DFE363A996A0F285423BEFC3A29F87948A0D";
        const RAW_TRANSACTION_BASE64: &str = "AwADAAAAAAAAACEDeLqSkwVyfHvYThgegiZUvPu0+dU4kyd3PJKigGLC1spBH+wrzjjA/ZGZdQmUzpQyOiC3GyP2eBp8ga9cNlnIOkptMzAtfXPA2daH3xTqt25JQ+fZ6UKB3ypzTK3fOXaAATgAAQAAAgAAIQPoVeBC6iyS0jFV0Dly5WV0SEl6uDciQqqi4EATeUJutEEfAd6+/HbUM4FLS6+lNc6AH8vaD9lViiYny4GPsl/AlBxdr0WjJxxU/B0cNVH8kRMo+W6a+1iSN+NZS7MTyzmTHwACAAEDAAAhA6S0TKbm1a/xyrYMG+Y2odspJ1roL1TcoK9h552yE1VCQSA+KpHiQ8lDBseXI/1ZCMxEvu0qopdjDojaQ4FzaZMgUGfPBeXSfMbQGksLMNseKRBLob/g0DHJWqZAxSDOuAwZAfwAIQxGIDIHY9cjWxS0tJupeJuKMZwzFKmLxkU3NmqFTcFscilVAABBH9R3vwbfA3q5XJG4m4z87OAA1uG8wup915wGGKAxdEObXPSqIvPBWrHlGTf/Uymanc2cDH1uKdsniJyoORwauPBIqlz61/Kf9HDnubX4GoHRYdnb4WzE+Tdh+L39a2dN2A==";
        const EXPECTED_IDENTITY_ADDRESS: &str = "5tf2QotaJw8kRNpQEa8TXtRQ6FLxwUrY4Mtee2JF2nco";
        const EXPECTED_CORE_TRANSACTION_HASH: &str =
            "5529726cc14d856a363745c68ba914339c318a9b78a99bb4b4145b23d7630732";
        let raw_transaction = STANDARD
            .decode(RAW_TRANSACTION_BASE64)
            .expect("base64 transaction should decode");
        let state_transition = StateTransition::deserialize_from_bytes(&raw_transaction)
            .expect("State transition deserializes correctly");

        assert_eq!(
            &state_transition
                .transaction_id()
                .expect("expected transaction id")
                .encode_hex_upper::<String>(),
            EXPECTED_STATE_TRANSITION_HASH
        );

        let StateTransition::IdentityCreate(identity_create_transition) = state_transition else {
            panic!("expected identity create transition");
        };

        // This mainnet transaction uses a ChainAssetLockProof (not InstantAssetLockProof)
        // ChainAssetLockProof doesn't embed the full transaction, just the out_point reference
        let asset_lock_proof = identity_create_transition.asset_lock_proof();
        let AssetLockProof::Chain(chain_proof) = asset_lock_proof else {
            panic!("expected chain asset lock proof for this mainnet transaction");
        };

        // Verify the out_point references the expected transaction
        assert_eq!(
            &chain_proof.out_point.txid.to_string().to_lowercase(),
            EXPECTED_CORE_TRANSACTION_HASH
        );

        let identity_address = identity_create_transition
            .identity_id()
            .to_string(Encoding::Base58);

        assert_eq!(identity_address, EXPECTED_IDENTITY_ADDRESS);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn identity_create_transition_ser_de() {
        let platform_version = LATEST_PLATFORM_VERSION;
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let asset_lock_proof = raw_instant_asset_lock_proof_fixture(None, None);

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
        let asset_lock_proof = raw_instant_asset_lock_proof_fixture(None, None);

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
                    None,
                )
            })
            .collect::<Vec<_>>();
        let transitions = get_batched_transitions_fixture(
            [(DocumentTransitionActionType::Create, documents)],
            &mut nonces,
        );
        let documents_batch_transition: BatchTransition = BatchTransitionV1 {
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

    #[test]
    fn deserialize_empty_bytes_should_fail() {
        let result = StateTransition::deserialize_from_bytes(&[]);
        assert!(
            result.is_err(),
            "deserialization of empty bytes should fail"
        );
    }

    #[test]
    fn deserialize_single_byte_should_fail() {
        let result = StateTransition::deserialize_from_bytes(&[0xFF]);
        assert!(
            result.is_err(),
            "deserialization of a single 0xFF byte should fail"
        );
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn deserialize_truncated_bytes_should_fail() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let transition = IdentityCreditWithdrawalTransitionV0 {
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
        let state_transition: StateTransition = transition.into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        // Truncate to half
        let half = &bytes[..bytes.len() / 2];
        assert!(
            StateTransition::deserialize_from_bytes(half).is_err(),
            "deserialization of truncated-to-half bytes should fail"
        );

        // Truncate by removing last byte
        let minus_one = &bytes[..bytes.len() - 1];
        assert!(
            StateTransition::deserialize_from_bytes(minus_one).is_err(),
            "deserialization of bytes missing last byte should fail"
        );

        // Keep only first byte
        let first_only = &bytes[..1];
        assert!(
            StateTransition::deserialize_from_bytes(first_only).is_err(),
            "deserialization of only the first byte should fail"
        );
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn deserialize_corrupted_bytes_should_not_panic() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let transition = IdentityCreditWithdrawalTransitionV0 {
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
        let state_transition: StateTransition = transition.into();
        let mut bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        // Flip bits in the middle of the payload
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;

        // Should either fail or return a different value - must not panic
        let result = StateTransition::deserialize_from_bytes(&bytes);
        if let Ok(recovered) = result {
            assert_ne!(
                state_transition, recovered,
                "corrupted bytes should not deserialize to the original value"
            );
        }
    }

    #[test]
    fn deserialize_many_empty_list() {
        let result = StateTransition::deserialize_many(&[]);
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn deserialize_many_with_invalid_entry() {
        let result = StateTransition::deserialize_many(&[vec![0xFF]]);
        assert!(
            result.is_err(),
            "deserialize_many with invalid entry should fail"
        );
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn deserialize_many_with_valid_entries() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");

        let make_transition = |amount: u64, nonce: u64| -> StateTransition {
            let t = IdentityCreditWithdrawalTransitionV0 {
                identity_id: identity.id(),
                amount,
                core_fee_per_byte: 34,
                pooling: Pooling::Standard,
                output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: [1u8; 65].to_vec().into(),
            };
            t.into()
        };

        let st1 = make_transition(1000000, 1);
        let st2 = make_transition(2000000, 2);
        let st3 = make_transition(3000000, 3);

        let raw: Vec<Vec<u8>> = vec![
            st1.serialize_to_bytes().unwrap(),
            st2.serialize_to_bytes().unwrap(),
            st3.serialize_to_bytes().unwrap(),
        ];

        let recovered = StateTransition::deserialize_many(&raw).expect("should deserialize all");
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0], st1);
        assert_eq!(recovered[1], st2);
        assert_eq!(recovered[2], st3);
    }
}

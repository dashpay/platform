use crate::serialization::PlatformDeserializableUntrusted;
use crate::state_transition::*;
use crate::ProtocolError;

impl StateTransition {
    pub fn deserialize_many_untrusted(
        raw_state_transitions: &[Vec<u8>],
    ) -> Result<Vec<Self>, ProtocolError> {
        raw_state_transitions
            .iter()
            .map(|raw_state_transition| {
                Self::deserialize_from_bytes_untrusted(raw_state_transition)
            })
            .collect()
    }

    /// Decodes one transition of `state_transition_type` serialized on its own, without the
    /// `StateTransition` variant tag in front (what `IdentityUpdateTransition::serialize_to_bytes`
    /// produces, for example). Bytes left over after the transition are refused, and the
    /// `StateTransition` byte budget is applied to the body up front.
    pub fn deserialize_untagged_untrusted_exact(
        state_transition_type: StateTransitionType,
        bytes: &[u8],
    ) -> Result<Self, ProtocolError> {
        // The inner transition types declare no budget of their own, so the `StateTransition`
        // one is applied here: an untagged body is the tagged transition less its one-byte tag.
        if bytes.len() >= STATE_TRANSITION_MAX_ENCODED_BYTES {
            return Err(ProtocolError::MaxEncodedBytesReachedError {
                max_size_kbytes: STATE_TRANSITION_MAX_ENCODED_BYTES,
                size_hit: bytes.len(),
            });
        }
        let state_transition: Self = match state_transition_type {
            StateTransitionType::DataContractCreate => {
                DataContractCreateTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::DataContractUpdate => {
                DataContractUpdateTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::Batch => {
                BatchTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityCreate => {
                IdentityCreateTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityTopUp => {
                IdentityTopUpTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityCreditWithdrawal => {
                IdentityCreditWithdrawalTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::IdentityUpdate => {
                IdentityUpdateTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityCreditTransfer => {
                IdentityCreditTransferTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::MasternodeVote => {
                MasternodeVoteTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityCreditTransferToAddresses => {
                IdentityCreditTransferToAddressesTransition::deserialize_from_bytes_untrusted_exact(
                    bytes,
                )?
                .into()
            }
            StateTransitionType::IdentityCreateFromAddresses => {
                IdentityCreateFromAddressesTransition::deserialize_from_bytes_untrusted_exact(
                    bytes,
                )?
                .into()
            }
            StateTransitionType::IdentityTopUpFromAddresses => {
                IdentityTopUpFromAddressesTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::AddressFundsTransfer => {
                AddressFundsTransferTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::AddressFundingFromAssetLock => {
                AddressFundingFromAssetLockTransition::deserialize_from_bytes_untrusted_exact(
                    bytes,
                )?
                .into()
            }
            StateTransitionType::AddressCreditWithdrawal => {
                AddressCreditWithdrawalTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::Shield => {
                ShieldTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::ShieldedTransfer => {
                ShieldedTransferTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::Unshield => {
                UnshieldTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::ShieldFromAssetLock => {
                ShieldFromAssetLockTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::ShieldedWithdrawal => {
                ShieldedWithdrawalTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityCreateFromShieldedPool => {
                IdentityCreateFromShieldedPoolTransition::deserialize_from_bytes_untrusted_exact(
                    bytes,
                )?
                .into()
            }
            StateTransitionType::ShieldFromIdentity => {
                ShieldFromIdentityTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
            StateTransitionType::IdentityTopUpFromShieldedPool => {
                IdentityTopUpFromShieldedPoolTransition::deserialize_from_bytes_untrusted_exact(
                    bytes,
                )?
                .into()
            }
            StateTransitionType::IdentityKeyLimitsUpdate => {
                IdentityKeyLimitsUpdateTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::ContractUserModeration => {
                ContractUserModerationTransition::deserialize_from_bytes_untrusted_exact(bytes)?
                    .into()
            }
            StateTransitionType::ContractFeeClaim => {
                ContractFeeClaimTransition::deserialize_from_bytes_untrusted_exact(bytes)?.into()
            }
        };
        // Every arm converts into `StateTransition`, so a mismatched arm would still compile.
        if state_transition.state_transition_type() != state_transition_type {
            return Err(ProtocolError::CorruptedCodeExecution(format!(
                "untagged {state_transition_type} bytes decoded as {}",
                state_transition.state_transition_type()
            )));
        }
        Ok(state_transition)
    }
}

#[cfg(test)]
mod tests {
    use hex::ToHex;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use platform_value::string_encoding::Encoding;
    use platform_value::{Identifier, Value};
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
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use crate::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV0,
    };
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::{
        DocumentCreateTransition, DocumentCreateTransitionV0,
    };
    use crate::state_transition::batch_transition::batched_transition::document_transition::{
        DocumentTransition, DocumentTransitionV0Methods,
    };
    use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
    use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
    use crate::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV1,
    };
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
    use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use crate::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
    use crate::state_transition::StateTransitionType;
    use crate::state_transition::STATE_TRANSITION_MAX_ENCODED_BYTES;
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
        let state_transition = StateTransition::deserialize_from_bytes_untrusted(&raw_transaction)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
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
        let recovered_state_transition = StateTransition::deserialize_from_bytes_untrusted(&bytes)
            .expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    /// Stack size for tests that build `Value`s nested to the decoder depth ceiling.
    ///
    /// Only decoding is iterative: the derived `Encode`, `PartialEq` and drop glue recurse once
    /// per nesting level, and in debug builds those frames cost roughly 7 KiB per level, so a
    /// value ~256 levels deep exhausts libtest's default 2 MiB per-test thread. nextest runs each
    /// test on the 8 MiB main thread, which is why CI does not see the overflow.
    const DEEP_VALUE_TEST_STACK_SIZE: usize = 16 * 1024 * 1024;

    fn on_deep_value_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(DEEP_VALUE_TEST_STACK_SIZE)
            .spawn(test)
            .expect("the deep value test thread should spawn")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    }

    #[test]
    fn document_batch_rejects_excessive_value_depth_during_decode() {
        on_deep_value_stack(|| {
            let nested = (0..300).fold(Value::Null, |value, _| Value::Array(vec![value]));
            let document_transition = DocumentTransition::Create(DocumentCreateTransition::V0(
                DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                        id: Identifier::default(),
                        identity_contract_nonce: 1,
                        document_type_name: "test".to_string(),
                        data_contract_id: Identifier::default(),
                    }),
                    entropy: [0; 32],
                    data: BTreeMap::from([("nested".to_string(), nested)]),
                    prefunded_voting_balance: None,
                },
            ));
            assert_eq!(
                document_transition.first_data_depth_exceeding(256),
                Some(257)
            );

            let state_transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                transitions: vec![BatchedTransition::Document(document_transition)],
                ..Default::default()
            }));
            let bytes = state_transition
                .serialize_to_bytes()
                .expect("the state transition should encode below the byte limit");
            assert!(
                bytes.len() as u64
                    <= PlatformVersion::latest()
                        .system_limits
                        .max_state_transition_size
            );

            // The intentionally invalid transition is no longer needed after encoding. Avoid walking
            // its recursive data during drop so this regression test only exercises decoder behavior.
            std::mem::forget(state_transition);

            let error = StateTransition::deserialize_from_bytes_untrusted_in_version(
                &bytes,
                PlatformVersion::latest(),
            )
            .expect_err("excessive nesting must be rejected during decode");
            assert!(error
                .to_string()
                .contains("value nesting depth 257 exceeds maximum 256"));
        });
    }

    #[test]
    fn document_batch_value_depth_limits_align_between_decode_and_validation() {
        on_deep_value_stack(|| {
            // Every decodable document value must also satisfy the consensus depth rule, so depth
            // violations always fail the same way: as an undecodable transition. A value at the
            // decoder ceiling must therefore round-trip and pass the validation-side depth check.
            let max_depth = PlatformVersion::latest()
                .system_limits
                .max_document_value_depth
                .expect("latest protocol should enforce document value depth")
                as usize;
            let nested = (1..max_depth).fold(Value::Array(vec![Value::Null]), |value, _| {
                Value::Array(vec![value])
            });
            let document_transition = DocumentTransition::Create(DocumentCreateTransition::V0(
                DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                        id: Identifier::default(),
                        identity_contract_nonce: 1,
                        document_type_name: "test".to_string(),
                        data_contract_id: Identifier::default(),
                    }),
                    entropy: [0; 32],
                    data: BTreeMap::from([("nested".to_string(), nested)]),
                    prefunded_voting_balance: None,
                },
            ));
            assert_eq!(
                document_transition.first_data_depth_exceeding(max_depth),
                None
            );
            assert_eq!(
                document_transition.first_data_depth_exceeding(max_depth - 1),
                Some(max_depth)
            );

            let state_transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                transitions: vec![BatchedTransition::Document(document_transition)],
                ..Default::default()
            }));
            let bytes = state_transition
                .serialize_to_bytes()
                .expect("the state transition should encode below the byte limit");

            let recovered = StateTransition::deserialize_from_bytes_untrusted_in_version(
                &bytes,
                PlatformVersion::latest(),
            )
            .expect("a value at the decoder ceiling must decode");
            assert_eq!(state_transition, recovered);
        });
    }

    #[test]
    fn deserialize_empty_bytes_should_fail() {
        let result = StateTransition::deserialize_from_bytes_untrusted(&[]);
        assert!(
            result.is_err(),
            "deserialization of empty bytes should fail"
        );
    }

    #[test]
    fn deserialize_single_byte_should_fail() {
        let result = StateTransition::deserialize_from_bytes_untrusted(&[0xFF]);
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
            StateTransition::deserialize_from_bytes_untrusted(half).is_err(),
            "deserialization of truncated-to-half bytes should fail"
        );

        // Truncate by removing last byte
        let minus_one = &bytes[..bytes.len() - 1];
        assert!(
            StateTransition::deserialize_from_bytes_untrusted(minus_one).is_err(),
            "deserialization of bytes missing last byte should fail"
        );

        // Keep only first byte
        let first_only = &bytes[..1];
        assert!(
            StateTransition::deserialize_from_bytes_untrusted(first_only).is_err(),
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
        let result = StateTransition::deserialize_from_bytes_untrusted(&bytes);
        if let Ok(recovered) = result {
            assert_ne!(
                state_transition, recovered,
                "corrupted bytes should not deserialize to the original value"
            );
        }
    }

    /// Crafts a minimal payload that looks like a StateTransition variant
    /// (IdentityCreditWithdrawal = discriminant 5) followed by a version byte
    /// and then a Vec<u8> field with a varint-encoded length that claims to
    /// contain `fake_len` bytes. The total payload is small but the decoded
    /// length would trigger a huge allocation without the limit guard.
    fn craft_oversized_vec_payload(fake_len: u64) -> Vec<u8> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        // Build the payload: variant discriminant + version + bogus vec length + filler
        let mut buf = Vec::new();
        // StateTransition enum discriminant for IdentityCreditWithdrawal (index 5)
        buf.extend_from_slice(&bincode::encode_to_vec(5u32, config).unwrap());
        // Version byte (0 = V0)
        buf.push(0);
        // identity_id: 32 bytes (Identifier)
        buf.extend_from_slice(&[0u8; 32]);
        // amount: u64
        buf.extend_from_slice(&bincode::encode_to_vec(1000u64, config).unwrap());
        // core_fee_per_byte: u32
        buf.extend_from_slice(&bincode::encode_to_vec(1u32, config).unwrap());
        // pooling: enum variant 0
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        // output_script (CoreScript = BinaryData = Vec<u8>): encode the malicious length
        buf.extend_from_slice(&bincode::encode_to_vec(fake_len, config).unwrap());
        // Don't provide the actual bytes — the limit check should fire before allocation
        buf
    }

    #[test]
    fn deserialize_crafted_huge_vec_length_does_not_oom() {
        // Craft a small payload (~80 bytes) with a Vec<u8> field claiming 8 GB.
        // Without the limit fix this would attempt `vec![0u8; 8_000_000_000]` and abort.
        let payload = craft_oversized_vec_payload(8_000_000_000);
        let result = StateTransition::deserialize_from_bytes_untrusted(&payload);
        // Must return an error, not OOM-abort the process
        assert!(
            result.is_err(),
            "crafted payload with 8GB vec length must be rejected, not cause OOM"
        );
    }

    #[test]
    fn deserialize_crafted_vec_exceeding_limit_is_rejected() {
        // Craft a payload with a Vec<u8> claiming 200,000 bytes — exceeds the
        // 100,000 byte budget configured on StateTransition.
        // The limit causes bincode to reject the read (either as Io/LimitExceeded
        // or UnexpectedEnd depending on the exact code path). Either way, the
        // deserialization must fail safely without OOM.
        let payload = craft_oversized_vec_payload(200_000);
        let result = StateTransition::deserialize_from_bytes_untrusted(&payload);
        assert!(
            result.is_err(),
            "Vec length exceeding byte budget must be rejected"
        );
    }

    #[test]
    fn deserialize_no_limit_does_not_enforce_byte_budget() {
        // Same crafted payload with 200,000-byte Vec — the no_limit variant
        // should NOT reject it for byte budget reasons (it will still fail
        // because the data doesn't actually contain 200,000 bytes).
        let payload = craft_oversized_vec_payload(200_000);
        let result = StateTransition::deserialize_from_bytes_untrusted_no_limit(&payload);
        assert!(result.is_err());
        // any other error is fine — the data is garbage
        if let ProtocolError::MaxEncodedBytesReachedError { .. } = result.unwrap_err() {
            panic!("deserialize_from_bytes_untrusted_no_limit should NOT enforce byte budget");
        }
    }

    #[test]
    fn deserialize_many_empty_list() {
        let result = StateTransition::deserialize_many_untrusted(&[]);
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn deserialize_many_with_invalid_entry() {
        let result = StateTransition::deserialize_many_untrusted(&[vec![0xFF]]);
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

        let recovered =
            StateTransition::deserialize_many_untrusted(&raw).expect("should deserialize all");
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0], st1);
        assert_eq!(recovered[1], st2);
        assert_eq!(recovered[2], st3);
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn exact_decode_refuses_trailing_bytes() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let st: StateTransition = IdentityCreditWithdrawalTransitionV0 {
            identity_id: identity.id(),
            amount: 1000000,
            core_fee_per_byte: 34,
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: [1u8; 65].to_vec().into(),
        }
        .into();
        let bytes = st.serialize_to_bytes().unwrap();

        assert_eq!(
            StateTransition::deserialize_from_bytes_untrusted_exact(&bytes).unwrap(),
            st
        );

        let mut padded = bytes.clone();
        padded.push(0);
        // The derived decoder ignores the suffix; the exact one refuses it.
        assert_eq!(
            StateTransition::deserialize_from_bytes_untrusted(&padded).unwrap(),
            st
        );
        assert!(matches!(
            StateTransition::deserialize_from_bytes_untrusted_exact(&padded),
            Err(ProtocolError::PlatformDeserializationError(message))
                if message.contains("1 bytes left over")
        ));
    }

    /// A tagged transition followed by another decodes loosely as the first alone; only the
    /// exact decoder reports the suffix.
    #[test]
    fn exact_decode_refuses_a_transition_followed_by_another() {
        let update: StateTransition = IdentityUpdateTransitionV0 {
            identity_id: Identifier::from([0x21; 32]),
            revision: 1,
            nonce: 1,
            disable_public_keys: vec![1],
            ..Default::default()
        }
        .into();
        let withdrawal: StateTransition = IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from([0x21; 32]),
            amount: 1000000,
            core_fee_per_byte: 34,
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: 2,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let mut both = update.serialize_to_bytes().unwrap();
        both.extend_from_slice(&withdrawal.serialize_to_bytes().unwrap());

        assert_eq!(
            StateTransition::deserialize_from_bytes_untrusted(&both).unwrap(),
            update
        );
        assert!(matches!(
            StateTransition::deserialize_from_bytes_untrusted_exact(&both),
            Err(ProtocolError::PlatformDeserializationError(message)) if message.contains("left over")
        ));
    }

    #[test]
    #[cfg(feature = "random-identities")]
    fn untagged_decode_matches_the_tagged_transition() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let update = IdentityUpdateTransitionV0 {
            signature: [7u8; 65].to_vec().into(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: identity
                .public_keys()
                .values()
                .map(|public_key| public_key.into())
                .collect(),
            disable_public_keys: vec![],
            user_fee_increase: 0,
        };
        let inner_bytes = IdentityUpdateTransition::from(update.clone())
            .serialize_to_bytes()
            .unwrap();
        let st: StateTransition = update.into();

        let decoded = StateTransition::deserialize_untagged_untrusted_exact(
            StateTransitionType::IdentityUpdate,
            &inner_bytes,
        )
        .expect("untagged identity update decodes");
        assert_eq!(decoded, st);
        assert_eq!(
            &decoded.serialize_to_bytes().unwrap()[1..],
            &inner_bytes[..]
        );

        assert!(StateTransition::deserialize_untagged_untrusted_exact(
            StateTransitionType::Batch,
            &inner_bytes,
        )
        .is_err());

        let mut padded = inner_bytes.clone();
        padded.push(0);
        assert!(StateTransition::deserialize_untagged_untrusted_exact(
            StateTransitionType::IdentityUpdate,
            &padded,
        )
        .is_err());
    }

    /// The untagged decoder refuses a body at the `StateTransition` byte budget, naming it.
    #[test]
    fn untagged_decode_honours_the_state_transition_budget() {
        let st: StateTransition = IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from([0x21; 32]),
            amount: 1000000,
            core_fee_per_byte: 34,
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes(vec![0; STATE_TRANSITION_MAX_ENCODED_BYTES]),
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let tagged = bincode::encode_to_vec(&st, bincode::config::standard().with_big_endian())
            .expect("encodes");

        match StateTransition::deserialize_untagged_untrusted_exact(
            StateTransitionType::IdentityCreditWithdrawal,
            &tagged[1..],
        ) {
            Err(ProtocolError::MaxEncodedBytesReachedError {
                max_size_kbytes, ..
            }) => assert_eq!(max_size_kbytes, STATE_TRANSITION_MAX_ENCODED_BYTES),
            other => panic!("expected the budget error, got {other:?}"),
        }
    }
}

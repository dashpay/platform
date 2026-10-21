mod advanced_structure;
mod basic_structure;
pub(crate) mod identity_and_signatures;
mod state;

use crate::error::Error;
use dpp::dashcore::Network;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::basic_structure::v0::IdentityCreateStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_create::state::v1::IdentityCreateStateTransitionStateValidationV1;
use crate::platform_types::platform::PlatformRef;

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create::advanced_structure::v0::IdentityCreateStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::identity_create::advanced_structure::v1::IdentityCreateStateTransitionAdvancedStructureValidationV1;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

/// A trait for transforming into an action for the identity create transition
pub trait StateTransitionActionTransformerForIdentityCreateTransitionV0 {
    /// Transforming into the action
    fn transform_into_action_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionActionTransformerForIdentityCreateTransitionV0 for IdentityCreateTransition {
    fn transform_into_action_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                signable_bytes,
                validation_mode,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreateTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive to add as validation methods to the execution context
                self.validate_basic_structure_v0(platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

/// A trait for advanced structure validation after transforming into an action
pub trait StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0 {
    /// Validation of the advanced structure
    fn validate_advanced_structure_from_state_for_identity_create_transition(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0
    for IdentityCreateTransition
{
    fn validate_advanced_structure_from_state_for_identity_create_transition(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .advanced_structure
        {
            Some(0) => self.validate_advanced_structure_from_state_v0(
                action,
                signable_bytes,
                execution_context,
                platform_version,
            ),
            Some(1) => self.validate_advanced_structure_from_state_v1(
                action,
                signable_bytes,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0, 1],
            })),
        }
    }
}

/// A trait for state validation for the identity create transition
pub trait StateTransitionStateValidationForIdentityCreateTransitionV0 {
    /// Validate state
    fn validate_state_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateTransitionAction,
        platform: &PlatformRef<C>,
        block_info: &dpp::block::block_info::BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationForIdentityCreateTransitionV0 for IdentityCreateTransition {
    fn validate_state_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateTransitionAction,
        platform: &PlatformRef<C>,
        block_info: &dpp::block::block_info::BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, action, execution_context, tx, platform_version),
            1 => self.validate_state_v1(
                platform,
                block_info,
                action,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::{Network, PrivateKey};
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::native_bls::NativeBlsModule;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::ops::Div;

    #[tokio::test]
    async fn test_identity_create_validation_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(1)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(999),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_identity_public_key(key.clone(), private_key);

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1871240);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, 99913915760);
    }

    #[tokio::test]
    async fn test_identity_create_validation_latest_protocol_version() {
        run_test_identity_create_validation_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            // PROTOCOL_VERSION_14: 4,960 credits less, see the protocol version 13 twin
            1914580,
            99913872420,
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: 4,960 credits more processing than at the latest version. v14
    /// adds the documents expirations tree under `Misc` (key `E`), beside the total system
    /// credits item this transition rewrites, and the extra key reshapes the `Misc` Merk
    /// the write rehashes. v14's `ContractGroups` root tree (key 124) sits under the
    /// `Versions` node no fee-bearing transition rewrites and changes nothing here. Pinned so
    /// v13 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_identity_create_validation_protocol_version_13() {
        run_test_identity_create_validation_at_protocol_version(13, 1919540, 99913867460).await;
    }

    /// Helper for the paired fee tests above: the same scenario at the requested protocol
    /// version, asserting the processing fee and the resulting identity balance.
    async fn run_test_identity_create_validation_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
        expected_identity_balance: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(999),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_identity_public_key(key.clone(), private_key);

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, expected_identity_balance);
    }

    /// Neither the InstantSend lock nor the transition's signature covers an instant proof's
    /// asset-lock payload length prefix, and every node has always read the payload whatever
    /// length the prefix announces. Anyone relaying a pending IdentityCreate can change the
    /// prefix, so the changed transition must execute on every node exactly as the original:
    /// the same identity, at the same fee, with the same balance.
    #[tokio::test]
    async fn should_create_the_same_identity_from_a_proof_with_another_payload_length_prefix() {
        let original =
            create_identity_with_proof_payload_length_prefix(|length| vec![length]).await;
        let prefixes: [(&str, fn(u8) -> Vec<u8>); 4] = [
            ("one more", |length| vec![length + 1]),
            ("one less", |length| vec![length - 1]),
            ("three bytes wide", |_| vec![0xfd, 0x00, 0x01]),
            ("five bytes wide", |_| vec![0xfe, 0x00, 0x00, 0x01, 0x00]),
        ];
        for (case, prefix) in prefixes {
            assert_eq!(
                create_identity_with_proof_payload_length_prefix(prefix).await,
                original,
                "prefix {case}"
            );
        }
    }

    /// Executes, at the latest protocol version, an IdentityCreate whose serialized instant
    /// proof has its 1-byte asset-lock payload length prefix replaced by `prefix(length)`, and
    /// returns the processing fee and the new identity's balance.
    async fn create_identity_with_proof_payload_length_prefix(
        prefix: fn(u8) -> Vec<u8>,
    ) -> (dpp::fee::Credits, dpp::fee::Credits) {
        use dpp::dashcore::consensus::serialize;
        use dpp::prelude::AssetLockProof;

        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(999),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_identity_public_key(key.clone(), private_key);

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let AssetLockProof::Instant(instant_proof) = &asset_lock_proof else {
            panic!("expected an instant asset lock proof");
        };
        let proof_transaction = serialize(instant_proof.transaction());
        // The asset-lock payload ends the transaction, right after its 1-byte length.
        let payload_size = instant_proof
            .transaction()
            .special_transaction_payload
            .as_ref()
            .expect("expected an asset lock payload")
            .size();
        let prefix_at = proof_transaction.len() - payload_size - 1;
        assert_eq!(usize::from(proof_transaction[prefix_at]), payload_size);

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let mut identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction_at = identity_create_serialized_transition
            .windows(proof_transaction.len())
            .position(|bytes| bytes == proof_transaction.as_slice())
            .expect("expected the proof transaction in the serialized transition");
        let new_prefix = prefix(proof_transaction[prefix_at]);
        // The proof's transaction bytes follow their bincode length, a single byte below 251.
        let length = &mut identity_create_serialized_transition[transaction_at - 1];
        assert_eq!(usize::from(*length), proof_transaction.len());
        *length = u8::try_from(proof_transaction.len() - 1 + new_prefix.len())
            .ok()
            .filter(|length| *length < 251)
            .expect("a 1-byte bincode length");
        identity_create_serialized_transition.splice(
            transaction_at + prefix_at..=transaction_at + prefix_at,
            new_prefix.iter().copied(),
        );

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(
            processing_result.valid_count(),
            1,
            "prefix {new_prefix:02x?}"
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        (
            processing_result.aggregated_fees().processing_fee,
            identity_balance,
        )
    }

    #[tokio::test]
    async fn should_create_identity_with_bound_authentication_key() {
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        for (protocol, document_type_bound) in [(13, false), (13, true), (14, false), (14, true)] {
            let platform_version = PlatformVersion::get(protocol).unwrap();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_initial_protocol_version(protocol)
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let mut signer = SimpleSigner::default();

            let mut rng = StdRng::seed_from_u64(567);

            let (master_key, master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(58),
                    platform_version,
                )
                .expect("expected to get key pair");

            signer.add_identity_public_key(master_key.clone(), master_private_key);

            let (mut key, private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                    1,
                    Some(999),
                    platform_version,
                )
                .expect("expected to get key pair");

            use dpp::data_contract::accessors::v0::DataContractV0Getters;
            use dpp::identity::contract_bounds::ContractBounds;
            let dashpay = platform
                .drive
                .cache
                .system_data_contracts
                .load_dashpay(platform_version)
                .unwrap();
            let bounds = if document_type_bound {
                ContractBounds::SingleContractDocumentType {
                    id: dashpay.id(),
                    document_type_name: "profile".into(),
                }
            } else {
                ContractBounds::SingleContract { id: dashpay.id() }
            };
            let IdentityPublicKey::V0(ref mut key_v0) = key else {
                panic!("expected a version 0 key")
            };
            key_v0.contract_bounds = Some(bounds.clone());
            signer.add_identity_public_key(key.clone(), private_key);

            let (_, pk) = ECDSA_SECP256K1
                .random_public_and_private_key_data(&mut rng, platform_version)
                .unwrap();

            let asset_lock_proof = instant_asset_lock_proof_fixture(
                Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
                None,
            );

            let identifier = asset_lock_proof
                .create_identifier()
                .expect("expected an identifier");

            let identity: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([(0, master_key.clone()), (1, key.clone())]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                    &identity,
                    asset_lock_proof,
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .await
                .expect("expected an identity create transition");

            let identity_create_serialized_transition = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let before = platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            if protocol == 13 {
                // Protocol 13 keeps the historical Drive rejection of bound authentication keys.
                use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
                assert_matches!(processing_result.execution_results().as_slice(), [StateTransitionExecutionResult::InternalError(error)] if error.contains("identity key bounds error: purpose not available for key bounds"));
                assert_eq!(
                    platform
                        .drive
                        .grove
                        .root_hash(None, &platform_version.drive.grove_version)
                        .unwrap()
                        .unwrap(),
                    before
                );
                continue;
            }
            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            let identity_balance = platform
                .drive
                .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected there to be an identity balance for this identity");

            assert!(identity_balance > 0);
            use drive::drive::identity::key::fetch::IdentityKeysRequest;
            let fetched = platform
                .drive
                .fetch_identity_keys_as_partial_identity(
                    IdentityKeysRequest::new_specific_key_query(&identity.id().to_buffer(), 1),
                    None,
                    platform_version,
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                fetched
                    .loaded_public_keys
                    .get(&1)
                    .unwrap()
                    .contract_bounds(),
                Some(&bounds)
            );
        }
    }

    #[tokio::test]
    async fn should_create_identity_with_an_authentication_key_bound_to_a_contract_group() {
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
        use dpp::consensus::codes::ErrorWithCode;
        use dpp::contract_group::{ContractGroupInfo, ContractGroupRegistration};
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::prelude::Identifier;
        for (protocol, group_exists) in [(13, false), (14, false), (14, true)] {
            let platform_version = PlatformVersion::get(protocol).unwrap();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_initial_protocol_version(protocol)
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let mut signer = SimpleSigner::default();

            let mut rng = StdRng::seed_from_u64(567);

            let (master_key, master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(58),
                    platform_version,
                )
                .expect("expected to get key pair");

            signer.add_identity_public_key(master_key.clone(), master_private_key);

            let (mut key, private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                    1,
                    Some(999),
                    platform_version,
                )
                .expect("expected to get key pair");

            use dpp::identity::contract_bounds::ContractBounds;
            let contract_group_id = Identifier::from([0x47; 32]);
            if group_exists {
                let info: ContractGroupInfo = (
                    Identifier::from([0x46; 32]),
                    ContractGroupRegistration {
                        admins: Default::default(),
                        name: None,
                        description: None,
                    },
                )
                    .into();
                platform
                    .drive
                    .insert_contract_group(
                        contract_group_id,
                        &info,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to register the group");
            }
            let bounds = ContractBounds::ContractGroup {
                id: contract_group_id,
            };
            let IdentityPublicKey::V0(ref mut key_v0) = key else {
                panic!("expected a version 0 key")
            };
            key_v0.contract_bounds = Some(bounds.clone());
            signer.add_identity_public_key(key.clone(), private_key);

            let (_, pk) = ECDSA_SECP256K1
                .random_public_and_private_key_data(&mut rng, platform_version)
                .unwrap();

            let asset_lock_proof = instant_asset_lock_proof_fixture(
                Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
                None,
            );

            let identifier = asset_lock_proof
                .create_identifier()
                .expect("expected an identifier");

            let identity: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([(0, master_key.clone()), (1, key.clone())]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                    &identity,
                    asset_lock_proof,
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .await
                .expect("expected an identity create transition");

            let identity_create_serialized_transition = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let before = platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            if protocol == 13 {
                // A transition carrying a group-bound key is not active before protocol version
                // 14: refused by the `active_version_range` check at decode, before any
                // consensus validation, with nothing charged and nothing written.
                assert_matches!(processing_result.execution_results().as_slice(), [StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(error)))] if error.state_transition_type().starts_with("IdentityCreate"));
                assert_eq!(
                    platform
                        .drive
                        .grove
                        .root_hash(None, &platform_version.drive.grove_version)
                        .unwrap()
                        .unwrap(),
                    before
                );
                continue;
            }
            if !group_exists {
                // The group is read in state validation: a paid failure on the asset lock.
                assert_matches!(processing_result.execution_results().as_slice(), [StateTransitionExecutionResult::PaidConsensusError { error, .. }] if error.code() == 41001);
                continue;
            }
            assert_eq!(processing_result.valid_count(), 1);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            let identity_balance = platform
                .drive
                .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
                .expect("expected to get identity balance")
                .expect("expected there to be an identity balance for this identity");

            assert!(identity_balance > 0);
            use drive::drive::identity::key::fetch::IdentityKeysRequest;
            let fetched = platform
                .drive
                .fetch_identity_keys_as_partial_identity(
                    IdentityKeysRequest::new_specific_key_query(&identity.id().to_buffer(), 1),
                    None,
                    platform_version,
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                fetched
                    .loaded_public_keys
                    .get(&1)
                    .unwrap()
                    .contract_bounds(),
                Some(&bounds)
            );
        }
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_reuse_after_issue_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(1)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let mut identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 80700 processing

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Okay now let us try to reuse the asset lock

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2146900);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, 99909310400); // The identity balance is smaller than if there hadn't been any issue
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_reuse_after_issue_latest_protocol_version() {
        run_test_identity_create_asset_lock_reuse_after_issue_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            // PROTOCOL_VERSION_14: 4,960 credits less, see the protocol version 13 twin
            2190240,
            99909267060,
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: 4,960 credits more processing than at the latest version. v14
    /// adds the documents expirations tree under `Misc` (key `E`), beside the total system
    /// credits item this transition rewrites, and the extra key reshapes the `Misc` Merk
    /// the write rehashes. v14's `ContractGroups` root tree (key 124) sits under the
    /// `Versions` node no fee-bearing transition rewrites and changes nothing here. Pinned so
    /// v13 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_identity_create_asset_lock_reuse_after_issue_protocol_version_13() {
        run_test_identity_create_asset_lock_reuse_after_issue_at_protocol_version(
            13,
            2195200,
            99909262100,
        )
        .await;
    }

    /// Helper for the paired fee tests above: the same scenario at the requested protocol
    /// version, asserting the processing fee and the resulting identity balance.
    async fn run_test_identity_create_asset_lock_reuse_after_issue_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
        expected_identity_balance: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let mut identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 80700 processing

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Okay now let us try to reuse the asset lock

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, expected_identity_balance); // The identity balance is smaller than if there hadn't been any issue
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_reuse_after_max_issues() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        for i in 0..16 {
            let (new_master_key, new_master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(58 + i),
                    platform_version,
                )
                .expect("expected to get key pair");

            signer.add_identity_public_key(new_master_key.clone(), new_master_private_key);

            let identity: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([
                    (0, new_master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .await
                .expect("expected an identity create transition");

            let identity_create_serialized_transition = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.invalid_paid_count(), 1);

            assert_eq!(processing_result.invalid_unpaid_count(), 0);

            assert_eq!(processing_result.valid_count(), 0);

            assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 80700 processing

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");
        }

        // Okay now let us try to reuse the asset lock, there should be no balance

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, new_public_key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 1);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_use_all_funds() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let base_cost = min_fees.identity_create_base_cost;
        let keys_extra_cost = base_cost
            .saturating_add(min_fees.identity_key_in_creation_cost.saturating_mul(2))
            .div(1000);

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            Some(220000 + keys_extra_cost),
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        // this should work for 2 times only
        for i in 0..2 {
            let (new_master_key, new_master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key(
                    0,
                    Some(58 + i),
                    platform_version,
                )
                .expect("expected to get key pair");

            signer.add_identity_public_key(new_master_key.clone(), new_master_private_key);

            let identity: Identity = IdentityV0 {
                id: identifier,
                public_keys: BTreeMap::from([
                    (0, new_master_key.clone()),
                    (1, critical_public_key_that_is_already_in_system.clone()),
                ]),
                balance: 1000000000,
                revision: 0,
            }
            .into();

            let identity_create_transition: StateTransition =
                IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .await
                .expect("expected an identity create transition");

            let identity_create_serialized_transition = identity_create_transition
                .serialize_to_bytes()
                .expect("serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![identity_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_eq!(processing_result.invalid_paid_count(), 1);

            assert_eq!(processing_result.invalid_unpaid_count(), 0);

            assert_eq!(processing_result.valid_count(), 0);

            assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 13800 processing

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");
        }

        // Okay now let us try to reuse the asset lock, there should be no balance

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, new_public_key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 1);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_replay_attack_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(1)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let mut identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 80700 processing

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // let's try to replay the bad transaction

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 1);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 0);

        // Okay now let us try to reuse the asset lock

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2146900);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, 99909310400); // The identity balance is smaller than if there hadn't been any issue
    }

    #[tokio::test]
    async fn test_identity_create_asset_lock_replay_attack_latest_protocol_version() {
        run_test_identity_create_asset_lock_replay_attack_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            // PROTOCOL_VERSION_14: 4,960 credits less, see the protocol version 13 twin
            2190240,
            99909267060,
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: 4,960 credits more processing than at the latest version. v14
    /// adds the documents expirations tree under `Misc` (key `E`), beside the total system
    /// credits item this transition rewrites, and the extra key reshapes the `Misc` Merk
    /// the write rehashes. v14's `ContractGroups` root tree (key 124) sits under the
    /// `Versions` node no fee-bearing transition rewrites and changes nothing here. Pinned so
    /// v13 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_identity_create_asset_lock_replay_attack_protocol_version_13() {
        run_test_identity_create_asset_lock_replay_attack_at_protocol_version(
            13,
            2195200,
            99909262100,
        )
        .await;
    }

    /// Helper for the paired fee tests above: the same scenario at the requested protocol
    /// version, asserting the processing fee and the resulting identity balance.
    async fn run_test_identity_create_asset_lock_replay_attack_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
        expected_identity_balance: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_state = platform.state.load();

        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(567);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key(
                0,
                Some(58),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key_that_is_already_in_system, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        // Let's start by adding this critical key to another identity

        let (another_master_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(53),
            platform_version,
        )
        .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, another_master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 100000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_identity_public_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key,
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_byte_array(&pk, Network::Testnet).unwrap()),
            None,
        );

        let identifier = asset_lock_proof
            .create_identifier()
            .expect("expected an identifier");

        let mut identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key_that_is_already_in_system.clone()),
            ]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 10080700); // 10000000 penalty + 80700 processing

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // let's try to replay the bad transaction

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 1);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 0);

        // Okay now let us try to reuse the asset lock

        let (new_public_key, new_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(13),
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(new_public_key.clone(), new_private_key);

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
            .await
            .expect("expected an identity create transition");

        let identity_create_serialized_transition = identity_create_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(identity.id().into_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, expected_identity_balance); // The identity balance is smaller than if there hadn't been any issue
    }

    /// The asset lock key signs an identity create's signable bytes, which leave out the
    /// identity id and every key's proof of possession. From protocol version 14 a block
    /// refuses, unpaid, a transition whose only fault is in one of those fields, and the asset
    /// lock it carries is left unused. check_tx refuses such a transition at every protocol
    /// version.
    mod fields_outside_the_asset_lock_signature {
        use super::*;
        use crate::execution::check_tx::CheckTxLevel;
        use crate::platform_types::platform::PlatformRef;
        use crate::platform_types::platform_state::PlatformStateV0Methods;
        use crate::platform_types::state_transitions_processing_result::{
            StateTransitionExecutionResult, StateTransitionsProcessingResult,
        };
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::TempPlatform;
        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
        use dpp::asset_lock::StoredAssetLockInfo;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::signature::SignatureError;
        use dpp::consensus::ConsensusError;
        use dpp::dashcore::consensus::serialize;
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::{BlockHash, OutPoint, Transaction};
        use dpp::dashcore_rpc::json::GetRawTransactionResult;
        use dpp::fee::Credits;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::{
            AssetLockProof, InstantAssetLockProof,
        };
        use dpp::identity::KeyType::ECDSA_HASH160;
        use dpp::identity::{Purpose, SecurityLevel};
        use dpp::platform_value::Bytes36;
        use dpp::serialization::Signable;
        use dpp::state_transition::public_key_in_creation::accessors::{
            IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
        };
        use dpp::tests::fixtures::{
            instant_asset_lock_is_lock_fixture, instant_asset_lock_proof_transaction_fixture,
        };
        use dpp::version::ProtocolVersion;
        use platform_version::version::LATEST_VERSION;

        /// The protocol versions the check_tx tests run at: 1 (the first), 13 (the last whose
        /// identity create advanced structure charges these faults) and the latest (which
        /// refuses them unpaid). This is a sample, not every version: protocol version 11 raises
        /// an identity create's minimum asset lock balance and 14 also changes its state
        /// validation, and the transitions these tests build meet both versions of each.
        const CHECK_TX_PROTOCOL_VERSIONS: [ProtocolVersion; 3] = [1, 13, LATEST_VERSION];

        #[derive(Clone, Copy, Debug)]
        enum Fault {
            /// The identity id is not the one the asset lock's outpoint derives
            IdentityIdNotFromAssetLock,
            /// A key's proof of possession does not verify
            InvalidKeyProofOfPossession,
            /// A hash key, which must not reveal its public key, carries a signature
            SignatureOnAHashKey,
        }

        const FAULTS: [Fault; 3] = [
            Fault::IdentityIdNotFromAssetLock,
            Fault::InvalidKeyProofOfPossession,
            Fault::SignatureOnAHashKey,
        ];

        impl Fault {
            fn is_reported_by(self, error: &ConsensusError) -> bool {
                match self {
                    Fault::IdentityIdNotFromAssetLock => matches!(
                        error,
                        ConsensusError::BasicError(BasicError::InvalidIdentifierError(_))
                    ),
                    Fault::InvalidKeyProofOfPossession => matches!(
                        error,
                        ConsensusError::SignatureError(SignatureError::BasicECDSAError(_))
                    ),
                    Fault::SignatureOnAHashKey => matches!(
                        error,
                        ConsensusError::SignatureError(
                            SignatureError::SignatureShouldNotBePresentError(_)
                        )
                    ),
                }
            }

            fn penalty(self, platform_version: &PlatformVersion) -> Credits {
                let penalties = &platform_version
                    .drive_abci
                    .validation_and_processing
                    .penalties;
                match self {
                    Fault::IdentityIdNotFromAssetLock => penalties.identity_id_not_correct,
                    Fault::InvalidKeyProofOfPossession | Fault::SignatureOnAHashKey => {
                        penalties.validation_of_added_keys_proof_of_possession_failure
                    }
                }
            }

            /// The transition with this fault. Its signable bytes are unchanged, so the asset
            /// lock signature still verifies.
            fn applied_to(self, transition: &StateTransition) -> StateTransition {
                let mut faulty = transition.clone();
                let StateTransition::IdentityCreate(IdentityCreateTransition::V0(v0)) = &mut faulty
                else {
                    panic!("expected an identity create v0 transition");
                };
                match self {
                    Fault::IdentityIdNotFromAssetLock => {
                        let mut identity_id = v0.identity_id.to_buffer();
                        identity_id[0] ^= 0x01;
                        v0.identity_id = Identifier::new(identity_id);
                    }
                    Fault::InvalidKeyProofOfPossession => {
                        let key = v0.public_keys.first_mut().expect("expected a key");
                        let mut signature = key.signature().to_vec();
                        let last_byte = signature.last_mut().expect("expected a signature");
                        *last_byte ^= 0x01;
                        key.set_signature(signature.into());
                    }
                    Fault::SignatureOnAHashKey => {
                        let key = v0
                            .public_keys
                            .iter_mut()
                            .find(|key| key.key_type() == ECDSA_HASH160)
                            .expect("expected a hash key");
                        key.set_signature(vec![1u8; 65].into());
                    }
                }
                assert_eq!(
                    faulty.signable_bytes().expect("expected signable bytes"),
                    transition
                        .signable_bytes()
                        .expect("expected signable bytes"),
                );
                faulty
            }
        }

        /// The keys an identity create registers.
        #[derive(Clone, Copy)]
        enum Keys {
            /// A master key, a critical key and a high level hash key
            Valid,
            /// The same keys with a critical key in place of the master key: a key structure
            /// failure, as an identity needs a master key
            WithoutAMasterKey,
            /// The valid keys, except that the hash key's data has this many bytes instead of
            /// the 20 bytes of a hash
            WithAHashKeyOfSize(usize),
        }

        /// How an identity create proves its asset lock.
        #[derive(Clone, Copy)]
        enum AssetLockProofKind {
            /// An instant lock of the asset lock transaction
            Instant,
            /// The asset lock transaction's outpoint, chain locked at the genesis core height;
            /// Core is asked for the transaction
            Chain,
        }

        struct Registration {
            identity_id: Identifier,
            asset_lock_proof: AssetLockProof,
            asset_lock_transaction: Transaction,
            transition: StateTransition,
        }

        fn new_platform(protocol_version: ProtocolVersion) -> TempPlatform<MockCoreRPCLike> {
            TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .with_config(PlatformConfig {
                    testing_configs: PlatformTestConfig {
                        disable_instant_lock_signature_verification: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .build_with_mock_rpc()
                .set_initial_state_structure()
        }

        /// An identity create funded by an instant asset lock, with every signature valid.
        async fn registration(keys: Keys, platform_version: &PlatformVersion) -> Registration {
            registration_proved_by(keys, AssetLockProofKind::Instant, platform_version).await
        }

        /// An identity create whose asset lock is proved by `proof_kind`, with every signature
        /// valid.
        async fn registration_proved_by(
            keys: Keys,
            proof_kind: AssetLockProofKind,
            platform_version: &PlatformVersion,
        ) -> Registration {
            let mut signer = SimpleSigner::default();
            let mut rng = StdRng::seed_from_u64(567);

            let (first_key, first_private_key) = match keys {
                Keys::Valid | Keys::WithAHashKeyOfSize(_) => {
                    IdentityPublicKey::random_ecdsa_master_authentication_key(
                        0,
                        Some(58),
                        platform_version,
                    )
                }
                Keys::WithoutAMasterKey => {
                    IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                        0,
                        Some(58),
                        platform_version,
                    )
                }
            }
            .expect("expected to get key pair");
            signer.add_identity_public_key(first_key.clone(), first_private_key);

            let (second_key, second_private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                    1,
                    Some(999),
                    platform_version,
                )
                .expect("expected to get key pair");
            signer.add_identity_public_key(second_key.clone(), second_private_key);

            let (mut hash_key, _) = IdentityPublicKey::random_key_with_known_attributes(
                2,
                &mut rng,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                ECDSA_HASH160,
                None,
                platform_version,
            )
            .expect("expected to get key pair");
            if let Keys::WithAHashKeyOfSize(size) = keys {
                hash_key.set_data(vec![0x5a; size].into());
            }

            let (_, pk) = ECDSA_SECP256K1
                .random_public_and_private_key_data(&mut rng, platform_version)
                .expect("expected to get key pair");

            let asset_lock_transaction = instant_asset_lock_proof_transaction_fixture(
                Some(
                    PrivateKey::from_byte_array(&pk, Network::Testnet)
                        .expect("expected a private key"),
                ),
                None,
            );
            let asset_lock_proof = match proof_kind {
                AssetLockProofKind::Instant => AssetLockProof::Instant(InstantAssetLockProof::new(
                    instant_asset_lock_is_lock_fixture(asset_lock_transaction.txid()),
                    asset_lock_transaction.clone(),
                    0,
                )),
                AssetLockProofKind::Chain => AssetLockProof::Chain(ChainAssetLockProof::new(
                    0,
                    OutPoint::new(asset_lock_transaction.txid(), 0).into(),
                )),
            };

            let identity: Identity = IdentityV0 {
                id: asset_lock_proof
                    .create_identifier()
                    .expect("expected an identifier"),
                public_keys: BTreeMap::from([(0, first_key), (1, second_key), (2, hash_key)]),
                balance: 0,
                revision: 0,
            }
            .into();

            let transition =
                IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
                .await
                .expect("expected an identity create transition");

            Registration {
                identity_id: identity.id(),
                asset_lock_proof,
                asset_lock_transaction,
                transition,
            }
        }

        fn check_tx(
            platform: &TempPlatform<MockCoreRPCLike>,
            transition: &StateTransition,
            check_tx_level: CheckTxLevel,
        ) -> Vec<ConsensusError> {
            let raw_transition = transition
                .serialize_to_bytes()
                .expect("expected to serialize");
            let state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };
            platform
                .check_tx(
                    &raw_transition,
                    check_tx_level,
                    &platform_ref,
                    state
                        .current_platform_version()
                        .expect("expected a platform version"),
                )
                .expect("expected to check tx")
                .errors
        }

        /// Processes the transitions as one block and commits it.
        fn process_block(
            platform: &TempPlatform<MockCoreRPCLike>,
            transitions: &[&StateTransition],
            platform_version: &PlatformVersion,
        ) -> StateTransitionsProcessingResult {
            let raw_transitions: Vec<Vec<u8>> = transitions
                .iter()
                .map(|transition| {
                    transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")
                })
                .collect();
            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &raw_transitions,
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");
            result
        }

        fn asset_lock_info(
            platform: &TempPlatform<MockCoreRPCLike>,
            registration: &Registration,
            platform_version: &PlatformVersion,
        ) -> StoredAssetLockInfo {
            let out_point = registration
                .asset_lock_proof
                .out_point()
                .expect("expected an outpoint");
            platform
                .drive
                .fetch_asset_lock_outpoint_info(
                    &Bytes36::new(out_point.into()),
                    None,
                    &platform_version.drive,
                )
                .expect("expected to fetch the asset lock info")
        }

        fn identity_balance(
            platform: &TempPlatform<MockCoreRPCLike>,
            registration: &Registration,
            platform_version: &PlatformVersion,
        ) -> Option<Credits> {
            platform
                .drive
                .fetch_identity_balance(
                    registration.identity_id.into_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch the identity balance")
        }

        /// check_tx refuses, at every protocol version, an identity create whose identity id is
        /// not the one its asset lock derives, while a transition with the same signable bytes
        /// and the derived identity id is still admitted. The identity id is checked before the
        /// asset lock proof, so the refusal names the identity id even once the asset lock is
        /// spent.
        #[tokio::test]
        async fn should_refuse_at_check_tx_an_identity_create_whose_identity_id_does_not_match_its_asset_lock(
        ) {
            let fault = Fault::IdentityIdNotFromAssetLock;
            for protocol_version in CHECK_TX_PROTOCOL_VERSIONS {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let platform = new_platform(protocol_version);
                let registration = registration(Keys::Valid, platform_version).await;
                let faulty = fault.applied_to(&registration.transition);

                let errors = check_tx(&platform, &faulty, CheckTxLevel::FirstTimeCheck);
                assert_matches!(
                    errors.as_slice(),
                    [error] if fault.is_reported_by(error),
                    "at protocol version {protocol_version}: {errors:?}"
                );
                assert_eq!(
                    check_tx(
                        &platform,
                        &registration.transition,
                        CheckTxLevel::FirstTimeCheck
                    ),
                    vec![],
                    "at protocol version {protocol_version}"
                );

                let created =
                    process_block(&platform, &[&registration.transition], platform_version);
                assert_eq!(
                    created.valid_count(),
                    1,
                    "{:?}",
                    created.execution_results()
                );
                let errors = check_tx(&platform, &faulty, CheckTxLevel::FirstTimeCheck);
                assert_matches!(
                    errors.as_slice(),
                    [error] if fault.is_reported_by(error),
                    "at protocol version {protocol_version} once the asset lock is spent: {errors:?}"
                );
            }
        }

        /// check_tx runs an identity create's advanced structure validation at every protocol
        /// version, so a transition with the fault never enters the mempool, while a transition
        /// with the same signable bytes and no fault is still admitted.
        async fn assert_refused_at_check_tx(fault: Fault) {
            for protocol_version in CHECK_TX_PROTOCOL_VERSIONS {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let platform = new_platform(protocol_version);
                let registration = registration(Keys::Valid, platform_version).await;

                let errors = check_tx(
                    &platform,
                    &fault.applied_to(&registration.transition),
                    CheckTxLevel::FirstTimeCheck,
                );
                assert_matches!(
                    errors.as_slice(),
                    [error] if fault.is_reported_by(error),
                    "at protocol version {protocol_version}: {errors:?}"
                );
                assert_eq!(
                    check_tx(
                        &platform,
                        &registration.transition,
                        CheckTxLevel::FirstTimeCheck
                    ),
                    vec![],
                    "at protocol version {protocol_version}"
                );
            }
        }

        #[tokio::test]
        async fn should_refuse_at_check_tx_an_identity_create_with_an_invalid_key_proof_of_possession(
        ) {
            assert_refused_at_check_tx(Fault::InvalidKeyProofOfPossession).await;
        }

        #[tokio::test]
        async fn should_refuse_at_check_tx_an_identity_create_with_a_signature_on_a_hash_key() {
            assert_refused_at_check_tx(Fault::SignatureOnAHashKey).await;
        }

        /// A key structure failure is charged by a block, but check_tx refuses it too, at every
        /// protocol version, since it runs the same advanced structure validation.
        #[tokio::test]
        async fn should_refuse_at_check_tx_an_identity_create_with_an_invalid_key_structure() {
            for protocol_version in CHECK_TX_PROTOCOL_VERSIONS {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let platform = new_platform(protocol_version);
                let registration = registration(Keys::WithoutAMasterKey, platform_version).await;

                let errors = check_tx(
                    &platform,
                    &registration.transition,
                    CheckTxLevel::FirstTimeCheck,
                );
                assert_matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::MissingMasterPublicKeyError(_)
                    )],
                    "at protocol version {protocol_version}: {errors:?}"
                );
            }
        }

        /// check_tx validates an identity create's advanced structure on the action it built
        /// and keeps that action, so the asset lock is verified once: Core is asked for a chain
        /// asset lock's transaction once.
        #[tokio::test]
        async fn should_verify_the_asset_lock_once_at_check_tx() {
            for protocol_version in CHECK_TX_PROTOCOL_VERSIONS {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let mut platform = new_platform(protocol_version);
                let registration = registration_proved_by(
                    Keys::Valid,
                    AssetLockProofKind::Chain,
                    platform_version,
                )
                .await;

                let transaction = registration.asset_lock_transaction.clone();
                platform
                    .core_rpc
                    .expect_get_optional_transaction_extended_info()
                    .times(1)
                    .returning(move |_| {
                        Ok(Some(GetRawTransactionResult {
                            in_active_chain: true,
                            hex: serialize(&transaction),
                            txid: transaction.txid(),
                            size: 0,
                            version: transaction.version as u32,
                            tx_type: 8,
                            locktime: transaction.lock_time,
                            vin: vec![],
                            vout: vec![],
                            extra_payload_size: None,
                            extra_payload: None,
                            blockhash: Some(BlockHash::all_zeros()),
                            confirmations: Some(1),
                            time: Some(0),
                            blocktime: Some(0),
                            height: Some(0),
                            instantlock: false,
                            instantlock_internal: false,
                            chainlock: true,
                        }))
                    });

                assert_eq!(
                    check_tx(
                        &platform,
                        &registration.transition,
                        CheckTxLevel::FirstTimeCheck
                    ),
                    vec![],
                    "at protocol version {protocol_version}"
                );
                platform.core_rpc.checkpoint();
            }
        }

        /// The asset lock key did not sign the field at fault, so a block refuses the
        /// transition without charging the asset lock or recording its signable bytes as
        /// used. A transition with the same signable bytes and no fault is still valid
        /// afterwards, at recheck and in a later block or later in the same block, and creates
        /// the identity with its full balance.
        async fn assert_refused_unpaid_leaving_the_asset_lock_untouched(fault: Fault) {
            let platform_version = PlatformVersion::latest();
            let registration = registration(Keys::Valid, platform_version).await;
            let faulty = fault.applied_to(&registration.transition);

            let platform = new_platform(platform_version.protocol_version);
            let created = process_block(&platform, &[&registration.transition], platform_version);
            assert_eq!(
                created.valid_count(),
                1,
                "{:?}",
                created.execution_results()
            );
            let expected_balance = identity_balance(&platform, &registration, platform_version)
                .expect("expected the identity to exist");

            let platform = new_platform(platform_version.protocol_version);
            let refused = process_block(&platform, &[&faulty], platform_version);
            assert_matches!(
                refused.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(error)]
                    if fault.is_reported_by(error),
                "{:?}",
                refused.execution_results()
            );
            assert_eq!(
                asset_lock_info(&platform, &registration, platform_version),
                StoredAssetLockInfo::NotPresent
            );
            assert_eq!(
                identity_balance(&platform, &registration, platform_version),
                None
            );
            assert_eq!(
                check_tx(&platform, &registration.transition, CheckTxLevel::Recheck),
                vec![]
            );
            let created = process_block(&platform, &[&registration.transition], platform_version);
            assert_eq!(
                created.valid_count(),
                1,
                "{:?}",
                created.execution_results()
            );
            assert_eq!(
                identity_balance(&platform, &registration, platform_version),
                Some(expected_balance)
            );

            let platform = new_platform(platform_version.protocol_version);
            let block = process_block(
                &platform,
                &[&faulty, &registration.transition],
                platform_version,
            );
            assert_matches!(
                block.execution_results().as_slice(),
                [
                    StateTransitionExecutionResult::UnpaidConsensusError(error),
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                ] if fault.is_reported_by(error),
                "{:?}",
                block.execution_results()
            );
            assert_eq!(
                identity_balance(&platform, &registration, platform_version),
                Some(expected_balance)
            );
        }

        #[tokio::test]
        async fn should_refuse_unpaid_an_identity_create_whose_identity_id_does_not_match_its_asset_lock(
        ) {
            assert_refused_unpaid_leaving_the_asset_lock_untouched(
                Fault::IdentityIdNotFromAssetLock,
            )
            .await;
        }

        #[tokio::test]
        async fn should_refuse_unpaid_an_identity_create_with_an_invalid_key_proof_of_possession() {
            assert_refused_unpaid_leaving_the_asset_lock_untouched(
                Fault::InvalidKeyProofOfPossession,
            )
            .await;
        }

        #[tokio::test]
        async fn should_refuse_unpaid_an_identity_create_with_a_signature_on_a_hash_key() {
            assert_refused_unpaid_leaving_the_asset_lock_untouched(Fault::SignatureOnAHashKey)
                .await;
        }

        /// The asset lock key signs the keys an identity create registers, so a key structure
        /// failure is still charged to the asset lock.
        #[tokio::test]
        async fn should_charge_the_asset_lock_for_an_identity_create_with_an_invalid_key_structure()
        {
            let platform_version = PlatformVersion::latest();
            let platform = new_platform(platform_version.protocol_version);
            let registration = registration(Keys::WithoutAMasterKey, platform_version).await;

            let charged = process_block(&platform, &[&registration.transition], platform_version);
            assert_matches!(
                charged.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::BasicError(BasicError::MissingMasterPublicKeyError(_)),
                    ..
                }],
                "{:?}",
                charged.execution_results()
            );
            let charged_credits = charged.aggregated_fees().processing_fee;
            assert!(
                charged_credits
                    > platform_version
                        .drive_abci
                        .validation_and_processing
                        .penalties
                        .validation_of_added_keys_structure_failure,
                "charged {charged_credits}"
            );
            let StoredAssetLockInfo::PartiallyConsumed(asset_lock_value) =
                asset_lock_info(&platform, &registration, platform_version)
            else {
                panic!("expected the asset lock to be partially consumed");
            };
            assert_eq!(
                asset_lock_value.remaining_credit_value(),
                asset_lock_value.initial_credit_value() - charged_credits
            );
        }

        /// A hash key's data is the hash itself, so from protocol version 14 data that is not 20
        /// bytes fails the key structure check: check_tx refuses it with a consensus error, and
        /// a block charges it to the asset lock like any other key structure failure, as the
        /// asset lock key signs the keys an identity create registers.
        #[tokio::test]
        async fn should_charge_the_asset_lock_for_an_identity_create_with_a_hash_key_of_the_wrong_size(
        ) {
            let platform_version = PlatformVersion::latest();
            for size in [0, 19, 21] {
                let platform = new_platform(platform_version.protocol_version);
                let registration =
                    registration(Keys::WithAHashKeyOfSize(size), platform_version).await;

                let errors = check_tx(
                    &platform,
                    &registration.transition,
                    CheckTxLevel::FirstTimeCheck,
                );
                assert_matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityPublicKeyDataError(error)
                    )] if error.public_key_id() == 2,
                    "{size} bytes: {errors:?}"
                );

                let charged =
                    process_block(&platform, &[&registration.transition], platform_version);
                assert_matches!(
                    charged.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::BasicError(
                            BasicError::InvalidIdentityPublicKeyDataError(_)
                        ),
                        ..
                    }],
                    "{size} bytes: {:?}",
                    charged.execution_results()
                );
                let charged_credits = charged.aggregated_fees().processing_fee;
                assert!(
                    charged_credits
                        > platform_version
                            .drive_abci
                            .validation_and_processing
                            .penalties
                            .validation_of_added_keys_structure_failure,
                    "{size} bytes: charged {charged_credits}"
                );
                let StoredAssetLockInfo::PartiallyConsumed(asset_lock_value) =
                    asset_lock_info(&platform, &registration, platform_version)
                else {
                    panic!("{size} bytes: expected the asset lock to be partially consumed");
                };
                assert_eq!(
                    asset_lock_value.remaining_credit_value(),
                    asset_lock_value.initial_credit_value() - charged_credits,
                    "{size} bytes"
                );
                assert_eq!(
                    identity_balance(&platform, &registration, platform_version),
                    None,
                    "{size} bytes"
                );
            }
        }

        /// Protocol version 13 does not check the size of a hash key's data, so such a key fails
        /// once its hash is taken, as an internal error that keeps the transition out of every
        /// block; that history replays unchanged.
        #[tokio::test]
        async fn should_fail_an_identity_create_with_a_hash_key_of_the_wrong_size_internally_at_protocol_version_13(
        ) {
            let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
            let platform = new_platform(platform_version.protocol_version);
            let registration = registration(Keys::WithAHashKeyOfSize(19), platform_version).await;

            let failed = process_block(&platform, &[&registration.transition], platform_version);
            assert_matches!(
                failed.execution_results().as_slice(),
                [StateTransitionExecutionResult::InternalError(_)],
                "{:?}",
                failed.execution_results()
            );
            assert_eq!(
                asset_lock_info(&platform, &registration, platform_version),
                StoredAssetLockInfo::NotPresent
            );
        }

        /// Before protocol version 14 these faults are charged to the asset lock and the
        /// transition's signable bytes are recorded as used, as advanced structure v0 does;
        /// those protocol versions keep this so their history replays unchanged.
        #[tokio::test]
        async fn should_charge_the_asset_lock_for_an_identity_id_or_key_proof_fault_before_protocol_version_14(
        ) {
            for protocol_version in [1, 13] {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                for fault in FAULTS {
                    let platform = new_platform(protocol_version);
                    let registration = registration(Keys::Valid, platform_version).await;

                    let charged = process_block(
                        &platform,
                        &[&fault.applied_to(&registration.transition)],
                        platform_version,
                    );
                    assert_matches!(
                        charged.execution_results().as_slice(),
                        [StateTransitionExecutionResult::PaidConsensusError { error, .. }]
                            if fault.is_reported_by(error),
                        "{fault:?} at protocol version {protocol_version}: {:?}",
                        charged.execution_results()
                    );
                    let charged_credits = charged.aggregated_fees().processing_fee;
                    assert!(
                        charged_credits > fault.penalty(platform_version),
                        "{fault:?} at protocol version {protocol_version}: charged {charged_credits}"
                    );
                    let StoredAssetLockInfo::PartiallyConsumed(asset_lock_value) =
                        asset_lock_info(&platform, &registration, platform_version)
                    else {
                        panic!("{fault:?} at protocol version {protocol_version}: expected the asset lock to be partially consumed");
                    };
                    assert_eq!(
                        asset_lock_value.remaining_credit_value(),
                        asset_lock_value.initial_credit_value() - charged_credits,
                        "{fault:?} at protocol version {protocol_version}"
                    );

                    let replayed =
                        process_block(&platform, &[&registration.transition], platform_version);
                    assert_matches!(
                        replayed.execution_results().as_slice(),
                        [StateTransitionExecutionResult::UnpaidConsensusError(
                            ConsensusError::BasicError(
                                BasicError::IdentityAssetLockStateTransitionReplayError(_)
                            )
                        )],
                        "{fault:?} at protocol version {protocol_version}"
                    );
                }
            }
        }
    }
}

mod advanced_structure;
mod basic_structure;
pub(crate) mod identity_and_signatures;
mod state;

use crate::error::Error;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::basic_structure::v0::IdentityCreateStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;
use crate::platform_types::platform::PlatformRef;

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create::advanced_structure::v0::IdentityCreateStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
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
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
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
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationForIdentityCreateTransitionV0 for IdentityCreateTransition {
    fn validate_state_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateTransitionAction,
        platform: &PlatformRef<C>,
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
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
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

    #[test]
    fn test_identity_create_validation() {
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

        signer.add_key(master_key.clone(), master_private_key.clone());

        let (key, private_key) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(999),
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_key(key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
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
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1823240);

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

        assert_eq!(identity_balance, 99916906760);
    }

    #[test]
    fn test_identity_create_asset_lock_reuse_after_issue() {
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

        signer.add_key(master_key.clone(), master_private_key.clone());

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

        signer.add_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key.clone(),
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
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
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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

        signer.add_key(new_public_key.clone(), new_private_key.clone());

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2098900);

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

        assert_eq!(identity_balance, 99912301400); // The identity balance is smaller than if there hadn't been any issue
    }

    #[test]
    fn test_identity_create_asset_lock_reuse_after_max_issues() {
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

        signer.add_key(master_key.clone(), master_private_key.clone());

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

        signer.add_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key.clone(),
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
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

            signer.add_key(new_master_key.clone(), new_master_private_key.clone());

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
                IdentityCreateTransition::try_from_identity_with_signer(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
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

        signer.add_key(new_public_key.clone(), new_private_key.clone());

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, new_public_key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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

    #[test]
    fn test_identity_create_asset_lock_use_all_funds() {
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

        signer.add_key(master_key.clone(), master_private_key.clone());

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

        signer.add_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key.clone(),
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
            Some(220000),
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

            signer.add_key(new_master_key.clone(), new_master_private_key.clone());

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
                IdentityCreateTransition::try_from_identity_with_signer(
                    &identity,
                    asset_lock_proof.clone(),
                    pk.as_slice(),
                    &signer,
                    &NativeBlsModule,
                    0,
                    platform_version,
                )
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

        signer.add_key(new_public_key.clone(), new_private_key.clone());

        let identity: Identity = IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::from([(0, master_key.clone()), (1, new_public_key.clone())]),
            balance: 1000000000,
            revision: 0,
        }
        .into();

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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

    #[test]
    fn test_identity_create_asset_lock_replay_attack() {
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

        signer.add_key(master_key.clone(), master_private_key.clone());

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

        signer.add_key(
            critical_public_key_that_is_already_in_system.clone(),
            private_key.clone(),
        );

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
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
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof.clone(),
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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

        signer.add_key(new_public_key.clone(), new_private_key.clone());

        // let's set the new key to the identity (replacing the one that was causing the issue
        identity.set_public_keys(BTreeMap::from([
            (0, master_key.clone()),
            (1, new_public_key.clone()),
        ]));

        let identity_create_transition: StateTransition =
            IdentityCreateTransition::try_from_identity_with_signer(
                &identity,
                asset_lock_proof,
                pk.as_slice(),
                &signer,
                &NativeBlsModule,
                0,
                platform_version,
            )
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
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2098900);

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

        assert_eq!(identity_balance, 99912301400); // The identity balance is smaller than if there hadn't been any issue
    }
}

pub(crate) mod identity_retrieval;
mod structure;
mod transform_into_action;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_top_up::structure::v0::IdentityTopUpStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_top_up::transform_into_action::v0::IdentityTopUpStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;

use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

/// A trait to transform into a top up action
pub trait StateTransitionIdentityTopUpTransitionActionTransformer {
    /// Transform into a top up action
    fn transform_into_action_for_identity_top_up_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionIdentityTopUpTransitionActionTransformer for IdentityTopUpTransition {
    fn transform_into_action_for_identity_top_up_transition<C: CoreRPCLike>(
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
            .identity_top_up_state_transition
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
                method: "identity top up transition: transform_top_up_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityTopUpTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_top_up_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive here, so need to ask users to pay for anything
                self.validate_basic_structure_v0(platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity top up transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity top up transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
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
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    #[test]
    fn test_identity_top_up_validation() {
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

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                1,
                Some(999),
                platform_version,
            )
            .expect("expected to get key pair");

        let identity_already_in_system: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: 50000000000,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity_already_in_system.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        signer.add_key(critical_public_key.clone(), private_key.clone());

        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(&mut rng, platform_version)
            .unwrap();

        let asset_lock_proof = instant_asset_lock_proof_fixture(
            Some(PrivateKey::from_slice(pk.as_slice(), Network::Testnet).unwrap()),
            None,
        );

        let identity_top_up_transition: StateTransition =
            IdentityTopUpTransition::try_from_identity(
                &identity_already_in_system,
                asset_lock_proof,
                pk.as_slice(),
                0,
                platform_version,
                None,
            )
            .expect("expected an identity create transition");

        let identity_top_up_serialized_transition = identity_top_up_transition
            .serialize_to_bytes()
            .expect("serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![identity_top_up_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 540580);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let identity_balance = platform
            .drive
            .fetch_identity_balance(
                identity_already_in_system.id().into_buffer(),
                None,
                platform_version,
            )
            .expect("expected to get identity balance")
            .expect("expected there to be an identity balance for this identity");

        assert_eq!(identity_balance, 149993654420); // about 0.5 Dash starting balance + 1 Dash asset lock top up
    }
}

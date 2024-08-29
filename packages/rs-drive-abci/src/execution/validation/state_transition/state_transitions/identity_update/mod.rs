pub(crate) mod advanced_structure;
mod basic_structure;
mod nonce;
mod state;

use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_update::basic_structure::v0::IdentityUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_update::state::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
};

use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for IdentityUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityUpdateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for IdentityUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        _block_info: &BlockInfo,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::tests::{
        setup_add_key_to_identity, setup_identity_return_master_key,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::signer::Signer;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_identity_update_that_disables_an_authentication_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![1],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_identity_update_that_disables_an_encryption_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (mut identity, mut signer, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();

        let key = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            4,
            2,
            Purpose::ENCRYPTION,
            SecurityLevel::MEDIUM,
            KeyType::ECDSA_SECP256K1,
            Some(ContractBounds::SingleContractDocumentType {
                id: dashpay.id(),
                document_type_name: "contactRequest".to_string(),
            }),
        );

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![key.id()],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&master_key, data.as_slice())
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}

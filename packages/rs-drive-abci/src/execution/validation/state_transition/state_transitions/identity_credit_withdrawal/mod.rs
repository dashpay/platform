mod balance;
mod nonce;
pub(crate) mod signature_purpose_matches_requirements;
mod state;
mod structure;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::drive::subscriptions::DriveSubscriptionFilter;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_credit_withdrawal::state::v0::IdentityCreditWithdrawalStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_credit_withdrawal::structure::v0::IdentityCreditWithdrawalStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_credit_withdrawal::structure::v1::IdentityCreditWithdrawalStateTransitionStructureValidationV1;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for IdentityCreditWithdrawalTransition {
    fn transform_into_action<'a, C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        _requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .basic_structure
        {
            Some(0) => {
                // Returns not supported
                self.validate_basic_structure_v0(platform_version)
            }
            Some(1) => self.validate_basic_structure_v1(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: validate_basic_structure"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity credit withdrawal transition: validate_basic_structure"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .state
        {
            0 => self.validate_state_v0(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: validate_state".to_string(),
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
        setup_identity_with_withdrawal_key_and_system_credits, setup_masternode_owner_identity,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::KeyType::{ECDSA_HASH160, ECDSA_SECP256K1};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
        IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
    };
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::withdrawal::Pooling;
    use platform_version::version::v1::PROTOCOL_VERSION_1;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn test_identity_credit_withdrawal_is_disabled_on_release() {
        let platform_version = PlatformVersion::first();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut rng = StdRng::seed_from_u64(567);

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(PROTOCOL_VERSION_1)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let (identity, signer, _, withdrawal_key) =
            setup_identity_with_withdrawal_key_and_system_credits(
                &mut platform,
                rng.gen(),
                ECDSA_SECP256K1,
                dash_to_credits!(0.5),
            );

        let platform_state = platform.state.load();

        let withdrawal_amount = dash_to_credits!(0.1);

        let credit_withdrawal_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            &identity,
            Some(CoreScript::random_p2pkh(&mut rng)),
            withdrawal_amount,
            Pooling::Never,
            1,
            0,
            signer,
            Some(&withdrawal_key),
            PreferredKeyPurposeForSigningWithdrawal::Any,
            2,
            platform_version,
            Some(1),
        )
        .expect("expected a credit withdrawal transition");

        let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![credit_withdrawal_transition_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::UnsupportedFeatureError(_))
            )]
        );
    }

    #[test]
    fn test_identity_credit_withdrawal_with_withdrawal_address_creates_withdrawal_document() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut rng = StdRng::seed_from_u64(567);

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let (identity, signer, _, withdrawal_key) =
            setup_identity_with_withdrawal_key_and_system_credits(
                &mut platform,
                rng.gen(),
                ECDSA_SECP256K1,
                dash_to_credits!(0.5),
            );

        let platform_state = platform.state.load();

        let withdrawal_amount = dash_to_credits!(0.1);

        let credit_withdrawal_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            &identity,
            Some(CoreScript::random_p2pkh(&mut rng)),
            withdrawal_amount,
            Pooling::Never,
            1,
            0,
            signer,
            Some(&withdrawal_key),
            PreferredKeyPurposeForSigningWithdrawal::Any,
            2,
            platform_version,
            None,
        )
        .expect("expected a credit withdrawal transition");

        let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![credit_withdrawal_transition_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(1_200_001_000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );
    }

    #[test]
    fn test_identity_credit_withdrawal_without_withdrawal_address_creates_withdrawal_document_when_signing_with_withdrawal_key(
    ) {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut rng = StdRng::seed_from_u64(567);

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let (identity, signer, _, withdrawal_key) =
            setup_identity_with_withdrawal_key_and_system_credits(
                &mut platform,
                rng.gen(),
                ECDSA_HASH160,
                dash_to_credits!(0.5),
            );

        let platform_state = platform.state.load();

        let withdrawal_amount = dash_to_credits!(0.1);

        let credit_withdrawal_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            &identity,
            None,
            withdrawal_amount,
            Pooling::Never,
            1,
            0,
            signer,
            Some(&withdrawal_key),
            PreferredKeyPurposeForSigningWithdrawal::TransferOnly,
            2,
            platform_version,
            None,
        )
        .expect("expected a credit withdrawal transition");

        let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![credit_withdrawal_transition_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(1_200_001_000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );
    }

    #[test]
    fn test_masternode_credit_withdrawal_without_withdrawal_address_creates_withdrawal_document_when_signing_with_withdrawal_key(
    ) {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut rng = StdRng::seed_from_u64(529);

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let (identity, signer, _, _) = setup_masternode_owner_identity(
            &mut platform,
            rng.gen(),
            dash_to_credits!(0.5),
            platform_version,
        );

        let platform_state = platform.state.load();

        let withdrawal_amount = dash_to_credits!(0.1);

        let credit_withdrawal_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            &identity,
            None,
            withdrawal_amount,
            Pooling::Never,
            1,
            0,
            signer,
            None,
            PreferredKeyPurposeForSigningWithdrawal::TransferOnly,
            2,
            platform_version,
            None,
        )
        .expect("expected a credit withdrawal transition");

        let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![credit_withdrawal_transition_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(1_200_001_000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );
    }

    #[test]
    fn test_masternode_credit_withdrawal_without_withdrawal_address_creates_withdrawal_document_when_signing_with_owner_key(
    ) {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut rng = StdRng::seed_from_u64(529);

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let (identity, signer, _, _) = setup_masternode_owner_identity(
            &mut platform,
            rng.gen(),
            dash_to_credits!(0.5),
            platform_version,
        );

        let platform_state = platform.state.load();

        let withdrawal_amount = dash_to_credits!(0.1);

        let credit_withdrawal_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            &identity,
            None,
            withdrawal_amount,
            Pooling::Never,
            1,
            0,
            signer,
            None,
            PreferredKeyPurposeForSigningWithdrawal::OwnerOnly,
            2,
            platform_version,
            None,
        )
        .expect("expected a credit withdrawal transition");

        let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![credit_withdrawal_transition_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(1_200_001_000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );
    }

    mod errors {
        use super::*;
        use dpp::consensus::state::state_error::StateError;
        #[test]
        fn test_credit_withdrawal_without_withdrawal_address_with_a_non_payable_transfer_key() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut rng = StdRng::seed_from_u64(567);

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let (identity, signer, _, withdrawal_key) =
                setup_identity_with_withdrawal_key_and_system_credits(
                    &mut platform,
                    rng.gen(),
                    ECDSA_SECP256K1,
                    dash_to_credits!(0.5),
                );

            let platform_state = platform.state.load();

            let withdrawal_amount = dash_to_credits!(0.1);

            let credit_withdrawal_transition =
                IdentityCreditWithdrawalTransition::try_from_identity(
                    &identity,
                    None,
                    withdrawal_amount,
                    Pooling::Never,
                    1,
                    0,
                    signer,
                    Some(&withdrawal_key),
                    PreferredKeyPurposeForSigningWithdrawal::TransferOnly,
                    2,
                    platform_version,
                    None,
                )
                .expect("expected a credit withdrawal transition");

            let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![credit_withdrawal_transition_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default_with_time(1_200_001_000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(
                        StateError::NoTransferKeyForCoreWithdrawalAvailableError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_masternode_credit_withdrawal_with_withdrawal_address_creates_when_signing_with_owner_key_should_fail(
        ) {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut rng = StdRng::seed_from_u64(529);

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

            let (identity, signer, _, _) = setup_masternode_owner_identity(
                &mut platform,
                rng.gen(),
                dash_to_credits!(0.5),
                platform_version,
            );

            let platform_state = platform.state.load();

            let withdrawal_amount = dash_to_credits!(0.1);

            let credit_withdrawal_transition =
                IdentityCreditWithdrawalTransition::try_from_identity(
                    &identity,
                    Some(CoreScript::random_p2pkh(&mut rng)),
                    withdrawal_amount,
                    Pooling::Never,
                    1,
                    0,
                    signer,
                    None,
                    PreferredKeyPurposeForSigningWithdrawal::OwnerOnly,
                    2,
                    platform_version,
                    None,
                )
                .expect("expected a credit withdrawal transition");

            let credit_withdrawal_transition_serialized_transition = credit_withdrawal_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![credit_withdrawal_transition_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default_with_time(1_200_001_000),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError(_)
                    )
                )]
            );
        }
    }
}

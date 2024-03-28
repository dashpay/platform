use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::consensus::signature::{BasicECDSAError, SignatureError};

use dpp::consensus::state::identity::invalid_asset_lock_proof_value::InvalidAssetLockProofValueError;
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::dashcore::signer;
use dpp::dashcore::signer::double_sha;
use dpp::identity::KeyType;

use dpp::identity::state_transition::AssetLockProved;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::Signable;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;

use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};

use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use dpp::version::DefaultForPlatformVersion;
use drive::grovedb::TransactionArg;

use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;
use crate::execution::validation::state_transition::identity_create::transform_into_partially_used_asset_lock_action::TransformIntoPartiallyUsedAssetLockAction;
use crate::execution::validation::state_transition::ValidationMode;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionStateValidationV0
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        action: StateTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionStateValidationV0 for IdentityCreateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        action: StateTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut state_transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        let identity_id = self.identity_id();
        let balance =
            drive.fetch_identity_balance(identity_id.to_buffer(), transaction, platform_version)?;

        // Balance is here to check if the identity does already exist
        if balance.is_some() {
            // Since the id comes from the state transition this should never be reachable
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id.to_owned()).into(),
            ));
        }

        // Now we should check the state of added keys to make sure there aren't any that already exist
        let unique_public_key_validation_result =
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys(),
                drive,
                &mut state_transition_execution_context,
                transaction,
                platform_version,
            )?;

        if unique_public_key_validation_result.is_valid() {
            // We just pass the action that was given to us
            Ok(ConsensusValidationResult::new_with_data(action))
        } else {
            // It's not valid, we need to give back the action that partially uses the asset lock

            let used_credits = 1000; //todo: figure this out
            self.transform_into_partially_used_asset_lock_action(
                unique_public_key_validation_result.errors,
                used_credits,
                &platform.into(),
                transaction,
                platform_version,
            )
        }
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Todo: we might want a lowered required balance
        let required_balance = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_create;

        // Validate asset lock proof state
        let asset_lock_proof_validation = if validation_mode != ValidationMode::NoValidation {
            self.asset_lock_proof().validate(
                platform,
                required_balance,
                transaction,
                platform_version,
            )?
        } else {
            ConsensusValidationResult::new()
        };

        if !asset_lock_proof_validation.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                asset_lock_proof_validation.errors,
            ));
        }

        let tx_out_validation = fetch_asset_lock_transaction_output_sync(
            platform.core_rpc,
            self.asset_lock_proof(),
            platform_version,
        )?;

        if !tx_out_validation.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
        let min_value = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_create;
        if tx_out.value < min_value {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidAssetLockProofValueError::new(tx_out.value, min_value).into(),
            ));
        }

        // Verify one time signature

        let public_key_hash = tx_out
            .script_pubkey
            .p2pkh_public_key_hash_bytes()
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "output must be a valid p2pkh already",
                ))
            })?;

        execution_context.add_operation(ValidationOperation::DoubleSha256);
        execution_context.add_operation(ValidationOperation::SignatureVerification(
            SignatureVerificationOperation::new(KeyType::ECDSA_HASH160),
        ));

        if let Err(e) = signer::verify_hash_signature(
            &double_sha(signable_bytes),
            self.signature().as_slice(),
            public_key_hash,
        ) {
            return Ok(ConsensusValidationResult::new_with_error(
                SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
            ));
        }

        let asset_lock_value_to_be_consumed = if asset_lock_proof_validation.has_data() {
            asset_lock_proof_validation.into_data()?
        } else {
            let initial_balance_amount = tx_out.value * CREDITS_PER_DUFF;
            AssetLockValue::new(
                initial_balance_amount,
                initial_balance_amount,
                platform_version,
            )?
        };

        match IdentityCreateTransitionAction::try_from_borrowed(
            self,
            asset_lock_value_to_be_consumed,
        ) {
            Ok(action) => Ok(ConsensusValidationResult::new_with_data(action.into())),
            Err(error) => Ok(ConsensusValidationResult::new_with_error(error)),
        }
    }
}

use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::balances::credits::CREDITS_PER_DUFF;

use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::consensus::state::identity::invalid_asset_lock_proof_value::InvalidAssetLockProofValueError;
use dpp::dashcore::signer;
use dpp::dashcore::signer::double_sha;
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::KeyType;

use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::Signable;

use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};

use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use drive::grovedb::TransactionArg;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;
use crate::execution::validation::state_transition::ValidationMode;

pub(in crate::execution::validation::state_transition::state_transitions::identity_top_up) trait IdentityTopUpStateTransitionStateValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityTopUpStateTransitionStateValidationV0 for IdentityTopUpTransition {
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
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
            .required_asset_lock_duff_balance_for_processing_start_for_identity_top_up;
        if tx_out.value < min_value {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidAssetLockProofValueError::new(tx_out.value, min_value).into(),
            ));
        }

        // Verify one time signature

        let signable_bytes = StateTransition::IdentityTopUp(self.clone()).signable_bytes()?;

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

        match IdentityTopUpTransitionAction::try_from_borrowed(
            self,
            asset_lock_value_to_be_consumed,
        ) {
            Ok(action) => Ok(ConsensusValidationResult::new_with_data(action.into())),
            Err(error) => Ok(ConsensusValidationResult::new_with_error(error)),
        }
    }
}

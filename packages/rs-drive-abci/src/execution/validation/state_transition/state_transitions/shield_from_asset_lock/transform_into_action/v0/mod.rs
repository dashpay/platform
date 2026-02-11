use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointNotEnoughBalanceError;
use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{signer, ScriptBuf, Txid};
use dpp::fee::Credits;
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::KeyType;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::{StateTransitionEstimatedFeeValidation, StateTransitionSingleSigned};
use drive::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockActionV0;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;

pub(in crate::execution::validation::state_transition::state_transitions::shield_from_asset_lock) trait ShieldFromAssetLockStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldFromAssetLockStateTransitionTransformIntoActionValidationV0
    for ShieldFromAssetLockTransition
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        // Extract note commitments and encrypted notes from serialized actions
        let note_commitments: Vec<[u8; 32]> = match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            ShieldFromAssetLockTransition::V0(v0) => v0
                .actions
                .iter()
                .map(|a| a.encrypted_note.clone())
                .collect(),
        };

        // Step 1: The value_balance must be negative for shielding (funds flowing into the pool)
        let value_balance = match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance,
        };
        if value_balance >= 0 {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(
                    "shield_from_asset_lock value_balance must be negative".to_string(),
                ))
                .into(),
            ));
        }

        // Step 2: Safely negate to get the shield amount, guarding against overflow (i64::MIN)
        let shield_amount: Credits = match value_balance.checked_neg() {
            Some(positive) => positive as u64,
            None => {
                return Ok(ConsensusValidationResult::new_with_error(
                    StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(
                        "shield_from_asset_lock value_balance overflow (i64::MIN)".to_string(),
                    ))
                    .into(),
                ));
            }
        };

        // Step 3: Calculate minimum required fee from platform_version
        let required_balance = self.calculate_min_required_fee(platform_version)?;

        let signable_bytes_len = signable_bytes.len();

        let mut signable_bytes_hasher = SignableBytesHasher::Bytes(signable_bytes);

        // Step 4: Validate asset lock proof
        let asset_lock_proof_validation = if validation_mode != ValidationMode::NoValidation {
            AssetLockProved::asset_lock_proof(self).validate(
                platform,
                &mut signable_bytes_hasher,
                required_balance,
                validation_mode,
                tx,
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

        // Step 5: Fetch/validate asset lock transaction output
        let mut needs_signature_verification = true;

        let asset_lock_value_to_be_consumed = if asset_lock_proof_validation.has_data() {
            let asset_lock_value = asset_lock_proof_validation.into_data()?;
            // There is no need to recheck signatures on recheck tx
            if validation_mode == ValidationMode::RecheckTx {
                needs_signature_verification = false;
            }
            asset_lock_value
        } else {
            let tx_out_validation = fetch_asset_lock_transaction_output_sync(
                platform.core_rpc,
                AssetLockProved::asset_lock_proof(self),
                platform_version,
            )?;

            if !tx_out_validation.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    tx_out_validation.errors,
                ));
            }

            let tx_out = tx_out_validation.into_data()?;

            let tx_out_credit_value = tx_out.value.saturating_mul(CREDITS_PER_DUFF);

            // Verify locked amount >= shield_amount + min_fee
            let required_total = shield_amount.saturating_add(required_balance);
            if tx_out_credit_value < required_total {
                let asset_lock_proof = AssetLockProved::asset_lock_proof(self);
                return Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                        asset_lock_proof
                            .out_point()
                            .map(|outpoint| outpoint.txid)
                            .unwrap_or(Txid::all_zeros()),
                        asset_lock_proof.output_index() as usize,
                        tx_out_credit_value,
                        tx_out_credit_value,
                        required_total,
                    )
                    .into(),
                ));
            }

            if validation_mode == ValidationMode::RecheckTx {
                needs_signature_verification = false;
            }

            let initial_balance_amount = tx_out.value * CREDITS_PER_DUFF;
            AssetLockValue::new(
                initial_balance_amount,
                tx_out.script_pubkey.0,
                initial_balance_amount,
                vec![],
                platform_version,
            )?
        };

        // Step 6: Verify ECDSA signature over signable_bytes (P2PKH from asset lock output)
        if needs_signature_verification {
            let tx_out_script_pubkey =
                ScriptBuf(asset_lock_value_to_be_consumed.tx_out_script().clone());

            let public_key_hash = tx_out_script_pubkey
                .p2pkh_public_key_hash_bytes()
                .ok_or_else(|| {
                    Error::Execution(ExecutionError::CorruptedCachedState(
                        "the script inside the state must be a p2pkh".to_string(),
                    ))
                })?;

            let block_count = signable_bytes_len as u16 / SHA256_BLOCK_SIZE;

            execution_context.add_operation(ValidationOperation::DoubleSha256(block_count));
            execution_context.add_operation(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(KeyType::ECDSA_HASH160),
            ));

            if let Err(e) = signer::verify_hash_signature(
                signable_bytes_hasher.hash_bytes().as_slice(),
                self.signature().as_slice(),
                public_key_hash,
            ) {
                return Ok(ConsensusValidationResult::new_with_error(
                    SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
                ));
            }
        }

        // Step 7: Also check that the remaining asset lock balance covers shield_amount
        let remaining_credit_value = asset_lock_value_to_be_consumed.remaining_credit_value();
        if remaining_credit_value < shield_amount {
            let asset_lock_proof = AssetLockProved::asset_lock_proof(self);
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                    asset_lock_proof
                        .out_point()
                        .map(|outpoint| outpoint.txid)
                        .unwrap_or(Txid::all_zeros()),
                    asset_lock_proof.output_index() as usize,
                    remaining_credit_value,
                    remaining_credit_value,
                    shield_amount,
                )
                .into(),
            ));
        }

        // Step 8: Read current shielded pool total balance from GroveDB
        let mut drive_operations = vec![];
        let pool_path = shielded_credit_pool_path();

        let current_total_balance = platform
            .drive
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&pool_path).into(),
                &[SHIELDED_TOTAL_BALANCE_KEY],
                DirectQueryType::StatefulDirectQuery,
                tx,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or(0);

        // Step 9: Verify Orchard ZK proof via reconstruct_and_verify_bundle()
        // Use EMPTY extra_sighash_data -- no transparent binding needed since
        // the asset lock proof authenticates the source of funds.
        let (actions, flags, anchor, proof, binding_signature) = match self {
            ShieldFromAssetLockTransition::V0(v0) => (
                &v0.actions,
                v0.flags,
                &v0.anchor,
                v0.proof.as_slice(),
                &v0.binding_signature,
            ),
        };

        if let Err(e) = reconstruct_and_verify_bundle(
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            &[], // No transparent fields to bind for shield_from_asset_lock
        ) {
            // Step 10: ZK proof failed -- consume asset lock with penalty (PartiallyUseAssetLockAction)
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .shielded_proof_verification_failure;

            let desired_used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "processing fee overflow in shield_from_asset_lock penalty calculation",
                )))?;

            let asset_lock_outpoint = AssetLockProved::asset_lock_proof(self)
                .out_point()
                .ok_or_else(|| {
                    Error::Execution(ExecutionError::CorruptedCachedState(
                        "asset lock proof must have an outpoint after validation".to_string(),
                    ))
                })?;

            let signable_bytes_hash: Bytes32 = signable_bytes_hasher.into_hashed_bytes();
            let mut previous_transaction_hashes =
                asset_lock_value_to_be_consumed.used_tags_ref().clone();
            previous_transaction_hashes.push(signable_bytes_hash);

            let remaining_after_penalty = remaining_credit_value.saturating_sub(desired_used_credits);
            let used_credits = std::cmp::min(remaining_credit_value, desired_used_credits);

            let user_fee_increase = match self {
                ShieldFromAssetLockTransition::V0(v0) => v0.user_fee_increase,
            };

            let partially_use_action = PartiallyUseAssetLockAction::from(
                PartiallyUseAssetLockActionV0 {
                    asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
                    initial_credit_value: asset_lock_value_to_be_consumed.initial_credit_value(),
                    previous_transaction_hashes,
                    asset_lock_script: asset_lock_value_to_be_consumed.tx_out_script().clone(),
                    remaining_credit_value: remaining_after_penalty,
                    used_credits,
                    user_fee_increase,
                    inputs_with_remaining_balance: None,
                    fee_strategy: None,
                },
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::PartiallyUseAssetLockAction(partially_use_action),
                vec![StateError::InvalidShieldedProofError(e).into()],
            ));
        }

        // Step 11: Build the successful action
        let asset_lock_outpoint = AssetLockProved::asset_lock_proof(self)
            .out_point()
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(
                    "asset lock proof must have an outpoint after validation".to_string(),
                ))
            })?;

        let asset_lock_value_credits = asset_lock_value_to_be_consumed.remaining_credit_value();
        let signable_bytes_hash: [u8; 32] = signable_bytes_hasher.into_hashed_bytes().0;

        let result = ShieldFromAssetLockTransitionAction::try_from_transition(
            self,
            asset_lock_outpoint.into(),
            asset_lock_value_credits,
            signable_bytes_hash,
            shield_amount,
            note_commitments,
            encrypted_notes,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, reconstruct_and_verify_bundle, FLAGS_OUTPUTS_ONLY,
};
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::check_tx_proof_verifier::CheckTxProofVerifier;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointNotEnoughBalanceError;
use dpp::consensus::basic::state_transition::ShieldedImplicitFeeCapExceededError;
use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::consensus::state::state_error::StateError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{signer, ScriptBuf, Txid};
use dpp::fee::Credits;
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::KeyType;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::{StateTransitionEstimatedFeeValidation, StateTransitionSingleSigned};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockActionV0;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shield_from_asset_lock) trait ShieldFromAssetLockStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
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
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        // Step 1: Get the shield amount (value_balance is u64, the amount entering the pool)
        let shield_amount: Credits = match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance,
        };

        // Step 3: Calculate minimum required fee from platform_version.
        // `required_balance` (= asset_lock_base_cost, `albc`) is the L1 asset-lock-proof viability
        // floor, reused unchanged in Step 4's `validate(...)` call below.
        let required_balance = self.calculate_min_required_fee(platform_version)?;

        // Step 3b: Compute the flat fee charged to the fee pools. ShieldFromAssetLock does strictly
        // more work than a transparent shield, so it pays BOTH the shielded operation cost and the
        // L1 asset-lock processing cost, mirroring the established asset-lock fee composition
        // (operation base cost + asset_lock_base_cost):
        //
        //   pool_fee = compute_minimum_shielded_fee(num_actions)  [Halo2 proof + per-action]
        //            + albc                                        [asset-lock processing]
        //
        // Set outside GroveDB; booked at the execution-event layer (PaidFromAssetLockToPool).
        let albc = required_balance;
        let num_actions = match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.actions.len(),
        };
        let shielded_fee = compute_minimum_shielded_fee(num_actions, platform_version)?;
        let pool_fee =
            shielded_fee
                .checked_add(albc)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "shielded fee + asset_lock_base_cost overflow in shield_from_asset_lock",
                )))?;
        // The asset lock must cover `shield_amount + pool_fee`, so the surplus is always >= 0.
        let required_lock_value = shield_amount.checked_add(pool_fee).ok_or(Error::Execution(
            ExecutionError::Overflow("shield_amount + pool_fee overflow in shield_from_asset_lock"),
        ))?;

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

            // Verify locked amount >= shield_amount + pool_fee (so the surplus is >= 0). This is an
            // early reject; Step 7 re-applies the same floor authoritatively on every path.
            let required_total = required_lock_value;
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

            let initial_balance_amount = tx_out.value.saturating_mul(CREDITS_PER_DUFF);
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

            let block_count =
                (signable_bytes_len / SHA256_BLOCK_SIZE as usize).min(u16::MAX as usize) as u16;

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

        // Step 7: The remaining asset-lock balance must cover `shield_amount + pool_fee`.
        // This is the AUTHORITATIVE funding floor: the fresh-fetch, cached `AssetLockValue`, and
        // recheck paths all converge here, so a lock that under-funds the fee can never reach the
        // action (which would mint credits at booking). It also guarantees `surplus >= 0` below.
        let remaining_credit_value = asset_lock_value_to_be_consumed.remaining_credit_value();
        if remaining_credit_value < required_lock_value {
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
                    required_lock_value,
                )
                .into(),
            ));
        }

        // Step 7b: Compute the surplus and reject an over-cap implicit donation BEFORE the expensive
        // Orchard proof verification (Step 9). The surplus is fully derivable here — it depends only
        // on the (now-validated) `remaining_credit_value`, the `shield_amount`, the flat `pool_fee`,
        // and whether a `surplus_output` is set — none of which depend on the ZK proof or the pool
        // read. Rejecting an over-cap transition here avoids ~100ms of Halo2 verification for a
        // transition that can never succeed.
        //
        // Distribute the fully-consumed lock:
        //   shield_amount -> shielded pool
        //   pool_fee      -> fee pools
        //   surplus       -> surplus_output address (when set), else folded into the fee pools.
        // `surplus >= 0` is guaranteed by the Step 7 floor; `checked_sub` is a defensive guard.
        let surplus = remaining_credit_value
            .checked_sub(shield_amount)
            .and_then(|v| v.checked_sub(pool_fee))
            .ok_or(Error::Execution(ExecutionError::Overflow(
                "asset lock value underflow computing shield_from_asset_lock surplus (should be guarded by the Step 7 funding floor)",
            )))?;

        let surplus_output = match self {
            ShieldFromAssetLockTransition::V0(v0) => &v0.surplus_output,
        };

        // When no surplus_output is set, the surplus is donated to the fee pools — but only up to
        // `shielded_implicit_fee_cap`, so a client cannot accidentally forfeit a large remainder.
        if surplus_output.is_none() {
            let implicit_fee_cap = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .shielded_implicit_fee_cap;
            if surplus > implicit_fee_cap {
                return Ok(ConsensusValidationResult::new_with_error(
                    ShieldedImplicitFeeCapExceededError::new(surplus, implicit_fee_cap).into(),
                ));
            }
        }

        // Step 8: Read current shielded pool total balance from GroveDB.
        //
        // ShieldFromAssetLock pays the flat `pool_fee` (computed in Step 3b) at the execution-event
        // layer (PaidFromAssetLockToPool). We do NOT derive a GroveDB fee from these read operations
        // — the flat fee subsumes them.
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(platform.drive, tx, &mut drive_operations, platform_version)?;

        // CheckTx admits the expensive proof only after the asset lock, its
        // signature, funding, fee cap, and current pool state have all passed.
        // Proposal and block processing pass `None` and retain the existing
        // penalty action when proof verification fails.
        let _check_tx_permit = match check_tx_proof_verifier {
            Some(verifier) => Some(verifier.try_acquire(num_actions).ok_or(Error::Execution(
                ExecutionError::CheckTxProofVerificationBusy,
            ))?),
            None => None,
        };

        // Step 9: Verify Orchard ZK proof via reconstruct_and_verify_bundle()
        // Use EMPTY extra_sighash_data -- no transparent binding needed since
        // the asset lock proof authenticates the source of funds.
        let (actions, anchor, proof, binding_signature) = match self {
            ShieldFromAssetLockTransition::V0(v0) => (
                &v0.actions,
                &v0.anchor,
                v0.proof.as_slice(),
                &v0.binding_signature,
            ),
        };

        if let Err(e) = reconstruct_and_verify_bundle(
            actions,
            FLAGS_OUTPUTS_ONLY,
            -(shield_amount as i64),
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

            let remaining_after_penalty =
                remaining_credit_value.saturating_sub(desired_used_credits);
            let used_credits = std::cmp::min(remaining_credit_value, desired_used_credits);

            let partially_use_action =
                PartiallyUseAssetLockAction::from(PartiallyUseAssetLockActionV0 {
                    asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
                    initial_credit_value: asset_lock_value_to_be_consumed.initial_credit_value(),
                    previous_transaction_hashes,
                    asset_lock_script: asset_lock_value_to_be_consumed.tx_out_script().clone(),
                    remaining_credit_value: remaining_after_penalty,
                    used_credits,
                    user_fee_increase: 0,
                    inputs_with_remaining_balance: None,
                    fee_strategy: None,
                });

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

        // The action routes `surplus_amount` to `surplus_output` (when set); otherwise 0 and the
        // surplus folds into the fee pools at the execution event. The surplus and the implicit-fee
        // cap were already computed and enforced in Step 7b (before proof verification).
        let surplus_amount = if surplus_output.is_some() { surplus } else { 0 };

        let result = ShieldFromAssetLockTransitionAction::try_from_transition(
            self,
            asset_lock_outpoint.into(),
            asset_lock_value_credits,
            signable_bytes_hash,
            shield_amount,
            current_total_balance,
            surplus_amount,
        );

        Ok(result.map(|action| action.into()))
    }
}

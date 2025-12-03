use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointNotEnoughBalanceError;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;

use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{signer, ScriptBuf, Txid};
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::KeyType;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::StateTransitionSingleSigned;
use dpp::version::PlatformVersion;
use drive::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use drive::grovedb::TransactionArg;

use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;
use crate::execution::validation::state_transition::ValidationMode;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::address_funding_from_asset_lock) trait AddressFundingFromAssetLockStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl AddressFundingFromAssetLockStateTransitionTransformIntoActionValidationV0
    for AddressFundingFromAssetLockTransition
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let required_balance = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_top_up;

        let signable_bytes_len = signable_bytes.len();

        let mut signable_bytes_hasher = SignableBytesHasher::Bytes(signable_bytes);

        // Validate asset lock proof state
        let asset_lock_proof_validation = if validation_mode != ValidationMode::NoValidation {
            AssetLockProved::asset_lock_proof(self).validate(
                platform,
                &mut signable_bytes_hasher,
                required_balance,
                validation_mode,
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

            // We should always check that the balance is enough as it's very cheap and we could have
            // had a version change that would have changed the minimum duff balance for processing
            // start

            let min_value = platform_version
                .dpp
                .state_transitions
                .identities
                .asset_locks
                .required_asset_lock_duff_balance_for_processing_start_for_address_funding;
            if tx_out.value < min_value {
                let asset_lock_proof = AssetLockProved::asset_lock_proof(self);
                return Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                        asset_lock_proof
                            .out_point()
                            .map(|outpoint| outpoint.txid)
                            .unwrap_or(Txid::all_zeros()),
                        asset_lock_proof.output_index() as usize,
                        tx_out.value,
                        tx_out.value,
                        min_value,
                    )
                    .into(),
                ));
            }

            // Verify one time signature
            // This is not necessary on recheck

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

        if needs_signature_verification {
            let tx_out_script_pubkey =
                ScriptBuf(asset_lock_value_to_be_consumed.tx_out_script().clone());

            // Verify one time signature

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

        // Calculate total available funds (asset lock + inputs from transition)
        let asset_lock_remaining = asset_lock_value_to_be_consumed.remaining_credit_value();
        let inputs_total: Credits = inputs_with_remaining_balance
            .values()
            .map(|(_, amount)| *amount)
            .sum();
        let total_available = asset_lock_remaining.saturating_add(inputs_total);

        // Calculate sum of explicit outputs (Some values only)
        let explicit_outputs_sum: Credits = self.outputs().values().filter_map(|v| *v).sum();

        // Validate that we have enough funds for explicit outputs
        if total_available < explicit_outputs_sum {
            return Ok(ConsensusValidationResult::new_with_error(
                AddressesNotEnoughFundsError::new(
                    inputs_with_remaining_balance.clone(),
                    explicit_outputs_sum,
                )
                .into(),
            ));
        }

        // Determine if remainder output should be removed
        // If total_available == explicit_outputs_sum, there's nothing left for remainder
        let should_remove_remainder = total_available == explicit_outputs_sum;

        match AddressFundingFromAssetLockTransitionAction::try_from_transition(
            self,
            signable_bytes_hasher,
            asset_lock_value_to_be_consumed,
            inputs_with_remaining_balance,
            should_remove_remainder,
        ) {
            Ok(action) => Ok(ConsensusValidationResult::new_with_data(action.into())),
            Err(error) => Ok(ConsensusValidationResult::new_with_error(error)),
        }
    }
}

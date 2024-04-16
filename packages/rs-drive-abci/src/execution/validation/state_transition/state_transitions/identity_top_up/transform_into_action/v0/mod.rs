use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointNotEnoughBalanceError;

use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{signer, ScriptBuf, Txid};
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::KeyType;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::StateTransitionLike;

use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use drive::grovedb::TransactionArg;

use crate::execution::types::execution_operation::{SHA256_BLOCK_SIZE, ValidationOperation};
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
        signable_bytes: Vec<u8>,
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
            .required_asset_lock_duff_balance_for_processing_start_for_identity_top_up;

        let signable_bytes_len = signable_bytes.len();

        let mut signable_bytes_hasher = SignableBytesHasher::Bytes(signable_bytes);

        // Validate asset lock proof state
        let asset_lock_proof_validation = if validation_mode != ValidationMode::NoValidation {
            self.asset_lock_proof().validate(
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
            // There is no needed to recheck signatures on recheck tx
            if validation_mode == ValidationMode::RecheckTx {
                needs_signature_verification = false;
            }
            asset_lock_value
        } else {
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

            // We should always check that the balance is enough as it's very cheap and we could have
            // had a version change that would have changed the minimum duff balance for processing
            // start

            let min_value = platform_version
                .dpp
                .state_transitions
                .identities
                .asset_locks
                .required_asset_lock_duff_balance_for_processing_start_for_identity_top_up;
            if tx_out.value < min_value {
                return Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                        self.asset_lock_proof()
                            .out_point()
                            .map(|outpoint| outpoint.txid)
                            .unwrap_or(Txid::all_zeros()),
                        self.asset_lock_proof().output_index() as usize,
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
                &signable_bytes_hasher.hash_bytes().as_slice(),
                self.signature().as_slice(),
                public_key_hash,
            ) {
                return Ok(ConsensusValidationResult::new_with_error(
                    SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
                ));
            }
        }

        match IdentityTopUpTransitionAction::try_from_borrowed(
            self,
            signable_bytes_hasher,
            asset_lock_value_to_be_consumed,
        ) {
            Ok(action) => Ok(ConsensusValidationResult::new_with_data(action.into())),
            Err(error) => Ok(ConsensusValidationResult::new_with_error(error)),
        }
    }
}

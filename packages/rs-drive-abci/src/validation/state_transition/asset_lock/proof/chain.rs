use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::{CoreRPCLike, CORE_RPC_INVALID_ADDRESS_OR_KEY};
use crate::validation::state_transition::asset_lock::transaction::validate_asset_lock_transaction_structure;
use dashcore_rpc::json::GetTransactionResult;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError,
};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{OutPoint, Transaction, Txid};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::CHAIN_ASSET_LOCK_PROOF_SCHEMA_VALIDATOR;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::{SimpleConsensusValidationResult, ValidationResult};

pub fn validate_structure(
    chain_asset_lock_proof: &ChainAssetLockProof,
) -> Result<SimpleConsensusValidationResult, Error> {
    let result = CHAIN_ASSET_LOCK_PROOF_SCHEMA_VALIDATOR
        .validate(
            &(chain_asset_lock_proof
                .to_cleaned_object()
                .expect("we don't hold unserializable structs")
                .try_into_validating_json()
                .expect("TODO")),
        )
        .expect("TODO: how jsonschema validation will ever fail?");

    Ok(result)
}

pub fn validate_state<C>(
    chain_asset_lock_proof: &ChainAssetLockProof,
    platform_ref: &PlatformRef<C>,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = ConsensusValidationResult::default();

    if platform_ref.block_info.core_height < chain_asset_lock_proof.core_chain_locked_height {
        result.add_error(InvalidAssetLockProofCoreChainHeightError::new(
            chain_asset_lock_proof.core_chain_locked_height,
            platform_ref.block_info.core_height,
        ));

        return Ok(result);
    }

    Ok(result)
}

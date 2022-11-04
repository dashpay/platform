use std::sync::Arc;

use dashcore::consensus::Decodable;
use dashcore::hashes::hex::ToHex;
use dashcore::hashes::Hash;
use dashcore::OutPoint;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError,
};
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, PublicKeyHash,
};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::{DashPlatformProtocolInitError, NonConsensusError};

lazy_static! {
    static ref CHAIN_ASSET_LOCK_PROOF_SCHEMA: Value = serde_json::from_str(include_str!(
        "../../../../schema/identity/stateTransition/assetLockProof/chainAssetLockProof.json"
    ))
    .unwrap();
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBlockHeader {
    current_core_chain_locked_height: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchTransactionResult {
    height: Option<u32>,
    data: Vec<u8>,
}

pub struct ChainAssetLockProofStructureValidator<SR>
where
    SR: StateRepositoryLike,
{
    json_schema_validator: JsonSchemaValidator,
    state_repository: Arc<SR>,
    asset_lock_transaction_validator: Arc<AssetLockTransactionValidator<SR>>,
}

impl<SR> ChainAssetLockProofStructureValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        asset_lock_transaction_validator: Arc<AssetLockTransactionValidator<SR>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(CHAIN_ASSET_LOCK_PROOF_SCHEMA.clone())?;

        Ok(Self {
            json_schema_validator,
            state_repository,
            asset_lock_transaction_validator,
        })
    }

    pub async fn validate(
        &self,
        raw_asset_lock_proof: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ValidationResult<PublicKeyHash>, NonConsensusError> {
        let mut result = ValidationResult::default();

        result.merge(self.json_schema_validator.validate(raw_asset_lock_proof)?);

        if !result.is_valid() {
            return Ok(result);
        }

        let proof: ChainAssetLockProof = serde_json::from_value(raw_asset_lock_proof.clone())
            .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

        let proof_core_chain_locked_height = proof.core_chain_locked_height();

        let latest_platform_block_header: PlatformBlockHeader = self
            .state_repository
            .fetch_latest_platform_block_header()
            .await
            .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

        let current_core_chain_locked_height =
            latest_platform_block_header.current_core_chain_locked_height;

        if current_core_chain_locked_height < proof_core_chain_locked_height {
            result.add_error(InvalidAssetLockProofCoreChainHeightError::new(
                proof_core_chain_locked_height,
                current_core_chain_locked_height,
            ));

            return Ok(result);
        }

        let out_point_buffer = proof.out_point();
        let out_point = OutPoint::consensus_decode(out_point_buffer.as_slice())
            .map_err(|e| NonConsensusError::SerdeParsingError(e.to_string().into()))?;

        let output_index = out_point.vout;
        let transaction_hash = out_point.txid;
        let transaction_hash_string = transaction_hash.to_hex();

        let maybe_transaction_fetch_result: Option<FetchTransactionResult> = self
            .state_repository
            .fetch_transaction(&transaction_hash_string, execution_context)
            .await
            .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

        return if let Some(transaction_result) = maybe_transaction_fetch_result {
            if let Some(tx_height) = transaction_result.height {
                if proof_core_chain_locked_height < tx_height {
                    result.add_error(InvalidAssetLockProofTransactionHeightError::new(
                        proof_core_chain_locked_height,
                        Some(tx_height),
                    ));

                    return Ok(result);
                }
            } else {
                result.add_error(InvalidAssetLockProofTransactionHeightError::new(
                    proof_core_chain_locked_height,
                    None,
                ));

                return Ok(result);
            }

            let validate_asset_lock_transaction_result = self
                .asset_lock_transaction_validator
                .validate(
                    &transaction_result.data,
                    output_index as usize,
                    execution_context,
                )
                .await?;

            let validation_result_data = if validate_asset_lock_transaction_result.is_valid() {
                validate_asset_lock_transaction_result
                    .data()
                    .expect("This can not happen due to the logic above")
                    .clone()
            } else {
                result.merge(validate_asset_lock_transaction_result);
                return Ok(result);
            };

            let public_key_hash = validation_result_data.public_key_hash;

            result.set_data(public_key_hash);

            Ok(result)
        } else {
            result.add_error(IdentityAssetLockTransactionIsNotFoundError::new(
                transaction_hash.as_hash().into_inner(),
            ));

            Ok(result)
        };
    }
}

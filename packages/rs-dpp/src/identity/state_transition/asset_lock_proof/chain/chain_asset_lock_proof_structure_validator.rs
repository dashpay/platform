use std::convert::TryInto;
use std::sync::Arc;

use dashcore::consensus::Decodable;
use dashcore::hashes::hex::ToHex;
use dashcore::hashes::Hash;
use dashcore::OutPoint;
use lazy_static::lazy_static;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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
    static ref CHAIN_ASSET_LOCK_PROOF_SCHEMA: JsonValue = serde_json::from_str(include_str!(
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
        asset_lock_proof_object: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ValidationResult<PublicKeyHash>, NonConsensusError> {
        let mut result = ValidationResult::default();

        result.merge(
            self.json_schema_validator
                .validate(&asset_lock_proof_object.try_to_validating_json()?)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        let proof: ChainAssetLockProof =
            platform_value::from_value(asset_lock_proof_object.clone())
                .map_err(NonConsensusError::ValueError)?;

        let proof_core_chain_locked_height = proof.core_chain_locked_height;

        let current_core_chain_locked_height = self
            .state_repository
            .fetch_latest_platform_core_chain_locked_height()
            .await
            .map_err(|e| NonConsensusError::StateRepositoryFetchError(format!("state repository fetch current core chain locked height for chain asset lock proof verification error: {}",e.to_string())))?
            .unwrap_or(0);

        if current_core_chain_locked_height < proof_core_chain_locked_height {
            result.add_error(InvalidAssetLockProofCoreChainHeightError::new(
                proof_core_chain_locked_height,
                current_core_chain_locked_height,
            ));

            return Ok(result);
        }

        let out_point = OutPoint::consensus_decode(proof.out_point.as_slice())
            .map_err(|e| NonConsensusError::SerdeParsingError(e.to_string().into()))?;

        let output_index = out_point.vout;
        let transaction_hash = out_point.txid;
        let transaction_hash_string = transaction_hash.to_hex();

        let transaction_fetch_result = self
            .state_repository
            .fetch_transaction(&transaction_hash_string, Some(execution_context))
            .await
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "transaction fetching error for chain lock: {}",
                    e.to_string()
                ))
            })?;

        let transaction_result = transaction_fetch_result
            .try_into()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "transaction decoding error: {}",
                    e.to_string()
                ))
            })?;

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

        if let Some(raw_tx) = transaction_result.data {
            let validate_asset_lock_transaction_result = self
                .asset_lock_transaction_validator
                .validate(&raw_tx, output_index as usize, execution_context)
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
            let mut hash = transaction_hash.as_hash().into_inner();
            hash.reverse();
            result.add_error(IdentityAssetLockTransactionIsNotFoundError::new(hash));

            Ok(result)
        }
    }
}

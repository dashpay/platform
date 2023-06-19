use std::sync::Arc;

use dashcore::consensus;
use dashcore::InstantLock;
use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::consensus::basic::identity::{
    IdentityAssetLockProofLockedTransactionMismatchError, InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError,
};
use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionValidator;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{ConsensusValidationResult, JsonSchemaValidator};
use crate::{DashPlatformProtocolInitError, NonConsensusError};

lazy_static! {
    static ref INSTANT_ASSET_LOCK_PROOF_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../schema/identity/stateTransition/assetLockProof/instantAssetLockProof.json"
    ))
    .unwrap();
}

pub type PublicKeyHash = [u8; 20];

pub struct InstantAssetLockProofStructureValidator<SR>
where
    SR: StateRepositoryLike,
{
    json_schema_validator: JsonSchemaValidator,
    state_repository: Arc<SR>,
    asset_lock_transaction_validator: Arc<AssetLockTransactionValidator<SR>>,
}

impl<SR> InstantAssetLockProofStructureValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        asset_lock_transaction_validator: Arc<AssetLockTransactionValidator<SR>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(INSTANT_ASSET_LOCK_PROOF_SCHEMA.clone())?;

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
    ) -> Result<ConsensusValidationResult<PublicKeyHash>, NonConsensusError> {
        let mut result = ConsensusValidationResult::default();
        result.merge(
            self.json_schema_validator
                .validate(&asset_lock_proof_object.try_to_validating_json()?)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        let raw_is_lock = asset_lock_proof_object.get_bytes("instantLock")?;

        let instant_lock = match consensus::deserialize::<InstantLock>(&raw_is_lock) {
            Ok(instant_lock) => instant_lock,
            Err(error) => {
                let err = InvalidInstantAssetLockProofError::new(error.to_string());
                result.add_error(err);
                return Ok(result);
            }
        };

        let is_signature_verified = self
            .state_repository
            .verify_instant_lock(&instant_lock, Some(execution_context))
            .await
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository verify instant send lock error: {}",
                    e
                ))
            })?;

        if !is_signature_verified {
            result.add_error(InvalidInstantAssetLockProofSignatureError::new());
            return Ok(result);
        }

        let tx_json_uint_array = asset_lock_proof_object.get_bytes("transaction")?;

        let output_index = asset_lock_proof_object.get_integer("outputIndex")?;

        let validate_asset_lock_transaction_result = self
            .asset_lock_transaction_validator
            .validate(&tx_json_uint_array, output_index, execution_context)
            .await?;

        let validation_result_data = if validate_asset_lock_transaction_result.is_valid_with_data()
        {
            validate_asset_lock_transaction_result
                .into_data()
                .expect("This can not happen due to the logic above")
        } else {
            result.merge(validate_asset_lock_transaction_result);
            return Ok(result);
        };

        let public_key_hash = validation_result_data.public_key_hash;
        let transaction = &validation_result_data.transaction;

        if instant_lock.txid != transaction.txid() {
            result.add_error(IdentityAssetLockProofLockedTransactionMismatchError::new(
                instant_lock.txid,
                transaction.txid(),
            ));

            return Ok(result);
        }

        result.set_data(public_key_hash);

        Ok(result)
    }
}

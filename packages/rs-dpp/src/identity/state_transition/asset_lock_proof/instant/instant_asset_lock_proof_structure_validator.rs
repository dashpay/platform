use std::sync::Arc;

use dashcore::consensus;
use dashcore::InstantLock;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::consensus::basic::identity::{
    IdentityAssetLockProofLockedTransactionMismatchError, InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError,
};
use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionValidator;
use crate::state_repository::StateRepositoryLike;
use crate::util::json_value::JsonValueExt;
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

lazy_static! {
    static ref INSTANT_ASSET_LOCK_PROOF_SCHEMA: Value = serde_json::from_str(include_str!(
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
        raw_asset_lock_proof: &Value,
    ) -> Result<ValidationResult<PublicKeyHash>, NonConsensusError> {
        let mut result = ValidationResult::default();

        result.merge(self.json_schema_validator.validate(raw_asset_lock_proof)?);

        if !result.is_valid() {
            return Ok(result);
        }

        let raw_is_lock: Vec<u8> = raw_asset_lock_proof
            .as_object()
            .ok_or_else(|| SerdeParsingError::new("Expected raw asset lock proof to be an object"))?
            .get("instantLock")
            .ok_or_else(|| {
                SerdeParsingError::new("Expected raw asset lock to have property 'instantLock'")
            })?
            .as_array()
            .ok_or_else(|| SerdeParsingError::new("Expected 'instantLock' to be an array"))?
            .iter()
            .map(|val| {
                val.as_u64()
                    .ok_or_else(|| SerdeParsingError::new("Expected 'instantLock' to be an array"))
            })
            .collect::<Result<Vec<u64>, SerdeParsingError>>()?
            .into_iter()
            .map(|n| n as u8)
            .collect();

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
            .verify_instant_lock(&instant_lock)
            .await
            .map_err(|err| NonConsensusError::StateRepositoryFetchError(err.to_string()))?;

        if !is_signature_verified {
            result.add_error(InvalidInstantAssetLockProofSignatureError::new());
            return Ok(result);
        }

        let tx_json_uint_array = raw_asset_lock_proof
            .get_bytes("transaction")
            .map_err(|err| SerdeParsingError::new(err.to_string()))?;

        let output_index = raw_asset_lock_proof
            .as_object()
            .ok_or_else(|| SerdeParsingError::new("Expected asset lock to be an object"))?
            .get("outputIndex")
            .ok_or_else(|| {
                SerdeParsingError::new("Expect asset lock to have a 'transaction field'")
            })?
            .as_u64()
            .ok_or_else(|| SerdeParsingError::new("Expect outputIndex to be a number"))?;

        let validate_asset_lock_transaction_result = self
            .asset_lock_transaction_validator
            .validate(&tx_json_uint_array, output_index as usize)
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

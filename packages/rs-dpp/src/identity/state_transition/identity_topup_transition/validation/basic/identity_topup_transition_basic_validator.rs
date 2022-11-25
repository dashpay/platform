use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::identity::state_transition::asset_lock_proof::AssetLockProofValidator;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::util::protocol_data::get_protocol_version;
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

lazy_static! {
    static ref INDENTITY_CREATE_TRANSITION_SCHEMA: Value = serde_json::from_str(include_str!(
        "../../../../../schema/identity/stateTransition/identityTopUp.json"
    ))
    .unwrap();
}

const ASSET_LOCK_PROOF_PROPERTY_NAME: &str = "assetLockProof";

pub struct IdentityTopUoTransitionBasicValidator<SR: StateRepositoryLike> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
}

impl<SR: StateRepositoryLike> IdentityTopUoTransitionBasicValidator<SR> {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(INDENTITY_CREATE_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            asset_lock_proof_validator,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        identity_topup_transition_json: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result = self
            .json_schema_validator
            .validate(identity_topup_transition_json)?;

        let identity_transition_map =
            identity_topup_transition_json.as_object().ok_or_else(|| {
                SerdeParsingError::new("Expected identity top up transition to be a json object")
            })?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator
                .validate(get_protocol_version(identity_transition_map)?)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.asset_lock_proof_validator
                .validate_structure(
                    identity_transition_map
                        .get(ASSET_LOCK_PROOF_PROPERTY_NAME)
                        .ok_or_else(|| {
                            NonConsensusError::SerdeJsonError(String::from(
                                "identity state transition must contain an asset lock proof",
                            ))
                        })?,
                    execution_context,
                )
                .await?,
        );

        Ok(result)
    }
}

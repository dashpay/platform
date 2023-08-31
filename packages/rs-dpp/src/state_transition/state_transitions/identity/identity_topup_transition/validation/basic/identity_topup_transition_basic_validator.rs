use std::sync::Arc;

use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProofValidator;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError};

lazy_static! {
    pub static ref IDENTITY_TOP_UP_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(
        include_str!("../../../../../schema/identity/stateTransition/identityTopUp.json")
    )
    .unwrap();
    pub static ref IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(IDENTITY_TOP_UP_TRANSITION_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

const ASSET_LOCK_PROOF_PROPERTY_NAME: &str = "assetLockProof";

pub struct IdentityTopUpTransitionBasicValidator<SR: StateRepositoryLike> {
    protocol_version_validator: ProtocolVersionValidator,
    json_schema_validator: JsonSchemaValidator,
    asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
}

impl<SR: StateRepositoryLike> IdentityTopUpTransitionBasicValidator<SR> {
    pub fn new(
        protocol_version_validator: ProtocolVersionValidator,
        asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(IDENTITY_TOP_UP_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            asset_lock_proof_validator,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        identity_topup_transition_object: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut result = self.json_schema_validator.validate(
            &identity_topup_transition_object
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator.validate(
                identity_topup_transition_object
                    .get_integer("protocolVersion")
                    .map_err(NonConsensusError::ValueError)?,
            )?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.asset_lock_proof_validator
                .validate_structure(
                    identity_topup_transition_object
                        .get_value(ASSET_LOCK_PROOF_PROPERTY_NAME)
                        .map_err(NonConsensusError::ValueError)?,
                    execution_context,
                )
                .await?,
        );

        Ok(result)
    }

    pub fn protocol_version_validator(&mut self) -> &mut ProtocolVersionValidator {
        &mut self.protocol_version_validator
    }
}

use std::sync::Arc;

use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProofValidator;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::property_names;
use crate::identity::state_transition::validate_public_key_signatures::TPublicKeysSignaturesValidator;
use crate::identity::validation::TPublicKeysValidator;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{BlsModule, DashPlatformProtocolInitError, NonConsensusError};

lazy_static! {
    pub static ref IDENTITY_CREATE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(
        include_str!("../../../../../schema/identity/stateTransition/identityCreate.json")
    )
    .unwrap();
    pub static ref IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(IDENTITY_CREATE_TRANSITION_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

const ASSET_LOCK_PROOF_PROPERTY_NAME: &str = "assetLockProof";

pub struct IdentityCreateTransitionBasicValidator<T, S, SR: StateRepositoryLike, SV, BLS: BlsModule>
{
    protocol_version_validator: ProtocolVersionValidator,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<T>,
    public_keys_in_identity_transition_validator: Arc<S>,
    asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
    public_keys_signatures_validator: Arc<SV>,
    bls_adapter: BLS,
}

impl<
        T: TPublicKeysValidator,
        S: TPublicKeysValidator,
        SR: StateRepositoryLike,
        SV: TPublicKeysSignaturesValidator,
        BLS: BlsModule,
    > IdentityCreateTransitionBasicValidator<T, S, SR, SV, BLS>
{
    pub fn new(
        protocol_version_validator: ProtocolVersionValidator,
        public_keys_validator: Arc<T>,
        public_keys_in_identity_transition_validator: Arc<S>,
        asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
        bls_adapter: BLS,
        public_keys_signatures_validator: Arc<SV>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(IDENTITY_CREATE_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            public_keys_validator,
            public_keys_in_identity_transition_validator,
            asset_lock_proof_validator,
            public_keys_signatures_validator,
            bls_adapter,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        transition_object: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut result = self.json_schema_validator.validate(
            &transition_object
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator.validate(
                transition_object
                    .get_integer(property_names::PROTOCOL_VERSION)
                    .map_err(NonConsensusError::ValueError)?,
            )?,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        let public_keys = transition_object
            .get_array_slice("publicKeys")
            .map_err(NonConsensusError::ValueError)?;
        result.merge(self.public_keys_validator.validate_keys(public_keys)?);
        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.public_keys_signatures_validator
                .validate_public_key_signatures(transition_object, public_keys)?,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.public_keys_in_identity_transition_validator
                .validate_keys(public_keys)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.asset_lock_proof_validator
                .validate_structure(
                    transition_object
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

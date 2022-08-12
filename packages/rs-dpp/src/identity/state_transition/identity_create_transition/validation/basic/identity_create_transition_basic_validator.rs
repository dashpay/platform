use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::identity::state_transition::asset_lock_proof::AssetLockProofValidator;
use crate::identity::validation::TPublicKeysValidator;
use crate::state_repository::StateRepositoryLike;
use crate::util::protocol_data::{get_protocol_version, get_raw_public_keys};
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

lazy_static! {
    static ref INDENTITY_CREATE_TRANSITION_SCHEMA: Value = serde_json::from_str(include_str!(
        "../../../../../schema/identity/stateTransition/identityCreate.json"
    ))
    .unwrap();
}

const ASSET_LOCK_PROOF_PROPERTY_NAME: &str = "assetLockProof";

pub struct IdentityCreateTransitionBasicValidator<T, S, SR: StateRepositoryLike> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<T>,
    public_keys_in_identity_transition_validator: Arc<S>,
    asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
}

impl<T: TPublicKeysValidator, S: TPublicKeysValidator, SR: StateRepositoryLike>
    IdentityCreateTransitionBasicValidator<T, S, SR>
{
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<T>,
        public_keys_in_identity_transition_validator: Arc<S>,
        asset_lock_proof_validator: Arc<AssetLockProofValidator<SR>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(INDENTITY_CREATE_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            public_keys_validator,
            public_keys_in_identity_transition_validator,
            asset_lock_proof_validator,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        identity_topup_transition_json: &Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result = self
            .json_schema_validator
            .validate(identity_topup_transition_json)?;

        let identity_transition_map = identity_topup_transition_json
            .as_object()
            .ok_or_else(|| SerdeParsingError::new("Expected identity to be a json object"))?;

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

        let public_keys = get_raw_public_keys(identity_transition_map)?;

        result.merge(self.public_keys_validator.validate_keys(public_keys)?);

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
                    identity_transition_map
                        .get(ASSET_LOCK_PROOF_PROPERTY_NAME)
                        .ok_or_else(|| {
                            NonConsensusError::SerdeJsonError(String::from(
                                "identity state transition must contain an asset lock proof",
                            ))
                        })?,
                )
                .await?,
        );

        Ok(result)
    }
}

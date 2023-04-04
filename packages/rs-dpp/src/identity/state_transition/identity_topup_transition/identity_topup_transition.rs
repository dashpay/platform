use std::convert::{TryFrom, TryInto};

use platform_value::{BinaryData, Value};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::Identifier;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::{
    StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
};
use crate::version::LATEST_VERSION;
use crate::{NonConsensusError, ProtocolError};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransition {
    // Own ST fields
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    // Generic identity ST fields
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub signature: BinaryData,
    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
}

impl Default for IdentityTopUpTransition {
    fn default() -> Self {
        Self {
            transition_type: StateTransitionType::IdentityTopUp,
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            protocol_version: Default::default(),
            signature: Default::default(),
            execution_context: Default::default(),
        }
    }
}

/// Main state transition functionality implementation
impl IdentityTopUpTransition {
    pub fn new(raw_state_transition: Value) -> Result<Self, ProtocolError> {
        Self::from_raw_object(raw_state_transition)
    }

    pub fn from_raw_object(raw_object: Value) -> Result<IdentityTopUpTransition, ProtocolError> {
        let protocol_version = raw_object
            .get_optional_integer(property_names::PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(LATEST_VERSION);
        let signature = raw_object
            .get_optional_binary_data(property_names::SIGNATURE)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();
        let identity_id = Identifier::from(
            raw_object
                .get_hash256(property_names::IDENTITY_ID)
                .map_err(ProtocolError::ValueError)?,
        );

        let raw_asset_lock_proof = raw_object
            .get_value(property_names::ASSET_LOCK_PROOF)
            .map_err(ProtocolError::ValueError)?;
        let asset_lock_proof = AssetLockProof::try_from(raw_asset_lock_proof)?;

        Ok(IdentityTopUpTransition {
            protocol_version,
            signature,
            identity_id,
            asset_lock_proof,
            transition_type: StateTransitionType::IdentityTopUp,
            execution_context: Default::default(),
        })
    }

    /// Get State Transition type
    pub fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }

    /// Set asset lock
    pub fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), NonConsensusError> {
        self.asset_lock_proof = asset_lock_proof;

        Ok(())
    }

    /// Get asset lock proof
    pub fn get_asset_lock_proof(&self) -> &AssetLockProof {
        &self.asset_lock_proof
    }

    /// Set identity id
    pub fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    /// Returns identity id
    pub fn get_identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    /// Returns Owner ID
    pub fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }
}

impl StateTransitionConvert for IdentityTopUpTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID]
    }
    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }
}

impl StateTransitionLike for IdentityTopUpTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Returns ids of created identities
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![*self.get_identity_id()]
    }

    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
    }
}

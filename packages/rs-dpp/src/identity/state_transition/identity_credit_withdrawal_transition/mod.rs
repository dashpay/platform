use bincode::{Decode, Encode};
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryInto;

use crate::contracts::withdrawals_contract::property_names::OUTPUT_SCRIPT;
use crate::version::LATEST_VERSION;
use crate::{
    identity::{core_script::CoreScript, KeyID},
    prelude::{Identifier, Revision},
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    ProtocolError,
};

use super::properties::{
    PROPERTY_IDENTITY_ID, PROPERTY_SIGNATURE, PROPERTY_SIGNATURE_PUBLIC_KEY_ID,
};

mod action;
pub mod apply_identity_credit_withdrawal_transition_factory;
pub mod validation;
pub use action::{
    IdentityCreditWithdrawalTransitionAction, IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION,
};

#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode)]
pub enum Pooling {
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

impl std::default::Default for Pooling {
    fn default() -> Self {
        Pooling::Never
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    #[bincode(with_serde)]
    pub output_script: CoreScript,
    pub revision: Revision,
    pub signature_public_key_id: KeyID,
    pub signature: BinaryData,
}

impl std::default::Default for IdentityCreditWithdrawalTransition {
    fn default() -> Self {
        IdentityCreditWithdrawalTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Default::default(),
            amount: Default::default(),
            core_fee_per_byte: Default::default(),
            pooling: Default::default(),
            output_script: Default::default(),
            revision: Default::default(),
            signature_public_key_id: Default::default(),
            signature: Default::default(),
        }
    }
}

impl IdentityCreditWithdrawalTransition {
    pub fn from_value(value: Value) -> Result<Self, ProtocolError> {
        let transition: IdentityCreditWithdrawalTransition = platform_value::from_value(value)?;

        Ok(transition)
    }

    pub fn from_json(value: JsonValue) -> Result<Self, ProtocolError> {
        let mut value: Value = value.into();
        value
            .replace_at_paths(Self::binary_property_paths(), ReplacementType::BinaryBytes)
            .map_err(ProtocolError::ValueError)?;
        value
            .replace_at_paths(
                Self::identifiers_property_paths(),
                ReplacementType::Identifier,
            )
            .map_err(ProtocolError::ValueError)?;
        Self::from_value(value)
    }

    pub fn from_raw_object(
        raw_object: Value,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        Self::from_value(raw_object)
    }

    pub fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    pub fn get_revision(&self) -> Revision {
        self.revision
    }
}

impl StateTransitionIdentitySigned for IdentityCreditWithdrawalTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for IdentityCreditWithdrawalTransition {
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        self.transition_type
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
}

impl StateTransitionConvert for IdentityCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE, PROPERTY_SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE, OUTPUT_SCRIPT]
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value = platform_value::to_value(self)?;
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
        let mut value = platform_value::to_value(self)?;
        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }
        Ok(value)
    }
}

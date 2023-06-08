use crate::identity::SecurityLevel::MASTER;
use crate::identity::{KeyID, SecurityLevel};
use crate::platform_serialization::PlatformSignable;
use crate::prelude::Identifier;
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::{
    StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike, StateTransitionType,
};
use crate::version::LATEST_VERSION;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::{BinaryData, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

mod property_names {
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
    pub const RECIPIENT_ID: &str = "recipientId";
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PartialEq,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
pub struct IdentityCreditTransferTransition {
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    // Own ST fields
    pub identity_id: Identifier,
    pub recipient_id: Identifier,
    pub amount: u64,
    // Generic identity ST fields
    pub protocol_version: u32,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for IdentityCreditTransferTransition {
    fn default() -> Self {
        IdentityCreditTransferTransition {
            transition_type: StateTransitionType::IdentityCreditTransfer,
            identity_id: Identifier::default(),
            recipient_id: Identifier::default(),
            amount: Default::default(),
            protocol_version: LATEST_VERSION,
            signature_public_key_id: Default::default(),
            signature: Default::default(),
        }
    }
}

impl IdentityCreditTransferTransition {
    pub fn new(raw_state_transition: Value) -> Result<Self, ProtocolError> {
        Self::from_raw_object(raw_state_transition)
    }

    pub fn from_value(value: Value) -> Result<Self, ProtocolError> {
        let transition: IdentityCreditTransferTransition = platform_value::from_value(value)?;

        Ok(transition)
    }

    pub fn from_raw_object(
        raw_object: Value,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        Self::from_value(raw_object)
    }

    pub fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditTransfer
    }

    pub fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    pub fn get_identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn set_recipient_id(&mut self, recipient_id: Identifier) {
        self.recipient_id = recipient_id;
    }

    pub fn get_recipient_id(&self) -> &Identifier {
        &self.recipient_id
    }

    pub fn get_amount(&self) -> u64 {
        self.amount
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }
}

impl StateTransitionConvert for IdentityCreditTransferTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID, property_names::RECIPIENT_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
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

impl StateTransitionLike for IdentityCreditTransferTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn get_signature(&self) -> &BinaryData {
        &self.signature
    }

    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id, self.recipient_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionIdentitySigned for IdentityCreditTransferTransition {
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

    fn get_security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![MASTER]
    }
}

#[cfg(test)]
mod test {
    use crate::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use crate::identity::KeyID;
    use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::StateTransitionType;
    use crate::ProtocolError;
    use bincode::{config, Decode, Encode};
    use platform_serialization::{PlatformDeserialize, PlatformSerialize};
    use platform_value::{BinaryData, Identifier};
    use rand::Rng;
    use std::fmt::Debug;

    fn test_identity_credit_transfer_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
        transition: T,
    ) {
        let serialized = T::serialize(&transition).expect("expected to serialize");
        let deserialized = T::deserialize(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_identity_credit_transfer_transition1() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditTransferTransition {
            transition_type: StateTransitionType::IdentityCreditTransfer,
            identity_id: Identifier::random(),
            recipient_id: Identifier::random(),
            amount: rng.gen(),
            protocol_version: rng.gen(),
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_identity_credit_transfer_transition(transition);
    }
}

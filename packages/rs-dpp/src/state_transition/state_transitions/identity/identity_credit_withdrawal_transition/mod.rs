use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryInto;

use crate::contracts::withdrawals_contract::property_names::OUTPUT_SCRIPT;
use crate::version::{FeatureVersion, LATEST_VERSION};
use crate::{
    identity::{core_script::CoreScript, KeyID},
    prelude::{Identifier, Revision},
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    ProtocolError,
};

use super::properties::{
    PROPERTY_IDENTITY_ID, PROPERTY_SIGNATURE, PROPERTY_SIGNATURE_PUBLIC_KEY_ID,
};
use crate::serialization_traits::PlatformSerializable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

mod action;
pub mod apply_identity_credit_withdrawal_transition_factory;
pub mod validation;

use crate::identity::SecurityLevel;
use crate::identity::SecurityLevel::{CRITICAL, HIGH, MEDIUM};
pub use action::{
    IdentityCreditWithdrawalTransitionAction, IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION,
};

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
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
pub struct IdentityCreditWithdrawalTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub revision: Revision,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
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

    #[cfg(feature = "platform-value")]
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

    fn get_security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL, HIGH, MEDIUM]
    }
}

impl StateTransitionLike for IdentityCreditWithdrawalTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        self.protocol_version as FeatureVersion
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        self.transition_type
    }

    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
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

#[cfg(test)]
mod test {
    use crate::identity::core_script::CoreScript;
    use crate::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
    use crate::identity::KeyID;
    use crate::prelude::Revision;
    use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::StateTransitionType;
    use crate::ProtocolError;
    use bincode::{config, Decode, Encode};
    use platform_serialization::{PlatformDeserialize, PlatformSerialize};
    use platform_value::{BinaryData, Identifier};
    use rand::Rng;
    use std::fmt::Debug;

    // Structure with 1 property
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition1 {
        pub protocol_version: u32,
    }

    // Structure with 2 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition2 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
    }

    // Structure with 3 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition3 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
    }

    // Structure with 4 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition4 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
    }

    // Structure with 5 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition5 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
    }

    // Structure with 6 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition6 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
    }

    // Structure with 7 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition7 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
    }

    // Structure with 8 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition8 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub revision: Revision,
    }

    // Structure with 9 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition9 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub revision: Revision,
        pub signature_public_key_id: KeyID,
    }

    // Structure with 10 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_error_type(ProtocolError)]
    pub struct IdentityCreditWithdrawalTransition10 {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub revision: Revision,
        pub signature_public_key_id: KeyID,
        pub signature: BinaryData,
    }

    fn test_identity_credit_withdrawal_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
        transition: T,
    ) {
        let serialized = T::serialize(&transition).expect("expected to serialize");
        let deserialized = T::deserialize(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_1() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition1 {
            protocol_version: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_2() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition2 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal, // Generate random value or choose from the available types
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_3() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition3 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal, // Generate random value or choose from the available types
            identity_id: Identifier::random(), // Generate a random Identifier
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_4() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition4 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal, // Generate random value or choose from the available types
            identity_id: Identifier::random(), // Generate a random Identifier
            amount: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_5() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition5 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal, // Generate random value or choose from the available types
            identity_id: Identifier::random(), // Generate a random Identifier
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_6() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition6 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal, // Generate random value or choose from the available types
            identity_id: Identifier::random(), // Generate a random Identifier
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard, // Generate random value or choose from the available options
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_7() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition7 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_8() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition8 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            revision: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_9() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition9 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            revision: rng.gen(),
            signature_public_key_id: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_10() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransition10 {
            protocol_version: rng.gen(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            revision: rng.gen(),
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }
}

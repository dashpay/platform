pub mod apply_data_contract_create_transition_factory;
pub mod validation;
use anyhow::anyhow;

use std::convert::TryInto;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    util::json_value::{JsonValueExt, ReplaceWith},
    ProtocolError,
};

const PROPERTY_SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
const PROPERTY_DATA_CONTRACT: &str = "dataContract";
const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_ENTROPY: &str = "entropy";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataContractCreateTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    // we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[serde(skip_serializing)]
    pub data_contract: DataContract,
    pub entropy: [u8; 32],
    pub signature_public_key_id: KeyID,
    pub signature: Vec<u8>,
}

impl std::default::Default for DataContractCreateTransition {
    fn default() -> Self {
        DataContractCreateTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::DataContractCreate,
            entropy: [0u8; 32],
            signature_public_key_id: 0,
            signature: vec![],
            data_contract: DataContract::default(),
        }
    }
}

impl DataContractCreateTransition {
    pub fn from_raw_object(
        mut raw_data_contract_update_transition: JsonValue,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        Ok(DataContractCreateTransition {
            protocol_version: raw_data_contract_update_transition
                .get_u64(PROPERTY_PROTOCOL_VERSION)? as u32,
            signature: raw_data_contract_update_transition
                .get_bytes(PROPERTY_SIGNATURE)
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_update_transition
                .get_u64(PROPERTY_SIGNATURE)
                .unwrap_or_default(),
            entropy: raw_data_contract_update_transition
                .get_bytes(PROPERTY_SIGNATURE)
                .unwrap_or_default()
                .try_into()
                .map_err(|_| anyhow!("entropy isn't 32 bytes long"))?,
            data_contract: DataContract::from_raw_object(
                raw_data_contract_update_transition.remove(PROPERTY_DATA_CONTRACT)?,
            )?,
            ..Default::default()
        })
    }

    pub fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }

    pub fn get_entropy(&self) -> &[u8; 32] {
        &self.entropy
    }

    /// Get owner ID
    pub fn get_owner_id(&self) -> &Identifier {
        &self.data_contract.owner_id
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<&Identifier> {
        vec![&self.data_contract.id]
    }
}

impl StateTransitionIdentitySigned for DataContractCreateTransition {
    fn get_signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for DataContractCreateTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature
    }
    fn calculate_fee(&self) -> Result<u64, ProtocolError> {
        todo!("fee calculation")
    }
}

impl StateTransitionConvert for DataContractCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![
            PROPERTY_SIGNATURE,
            PROPERTY_SIGNATURE_PUBLIC_KEY_ID,
            PROPERTY_ENTROPY,
        ]
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        json_value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Base64)?;
        json_value
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Base58)?;

        json_value.insert(
            PROPERTY_DATA_CONTRACT.to_string(),
            self.data_contract.to_json()?,
        )?;

        Ok(json_value)
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object: JsonValue = serde_json::to_value(self)?;
        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_object {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }
        json_object.insert(
            String::from(PROPERTY_DATA_CONTRACT),
            self.data_contract.to_object(false)?,
        )?;
        Ok(json_object)
    }
}

use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::serialization_traits::PlatformSerializable;
use platform_serialization::PlatformSignable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
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
    Convertible, ProtocolError,
};

use super::property_names::*;

use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};

pub mod validation;

use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::DataContractCreate;
use crate::version::FeatureVersion;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    PlatformSignable,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
pub struct DataContractCreateTransitionV0 {
    pub data_contract: DataContract,
    pub entropy: Bytes32,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for DataContractCreateTransitionV0 {
    fn default() -> Self {
        DataContractCreateTransitionV0 {
            entropy: Bytes32::default(),
            signature_public_key_id: 0,
            signature: BinaryData::default(),
            data_contract: Default::default(),
        }
    }
}

impl DataContractCreateTransitionV0 {
    /// Creates a new instance of the DataContractCreateTransition from the provided data contract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A mutable `DataContract` instance, to be used in the transition.
    /// * `entropy` - A `Bytes32` value providing additional randomness.
    /// * `identity` - A reference to a `PartialIdentity` object.
    /// * `key_id` - A `KeyID` identifier for the public key used for signing the transition.
    /// * `signer` - A reference to an object implementing the `Signer` trait.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Result<Self, ProtocolError>` containing a `DataContractCreateTransition`
    /// object. Otherwise, returns `ProtocolError`.
    pub fn new_from_data_contract<S: Signer>(
        mut data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
    ) -> Result<Self, ProtocolError> {
        data_contract.owner_id = identity.id;
        data_contract.id = generate_data_contract_id(identity.id, entropy);
        let mut transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract,
            entropy: Default::default(),
            signature_public_key_id: key_id,
            signature: Default::default(),
        });
        let value = transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        transition.set_signature(signer.sign(public_key, &value)?.into());
        Ok(transition)
    }

    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<DataContractCreateTransitionV0, ProtocolError> {
        Ok(DataContractCreateTransitionV0 {
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_object
                .remove_optional_bytes_32(ENTROPY)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_object.remove(DATA_CONTRACT).map_err(|_| {
                    ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    )
                })?,
            )?,
            ..Default::default()
        })
    }

    #[cfg(feature = "platform-value")]
    pub fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractCreateTransitionV0, ProtocolError> {
        Ok(DataContractCreateTransitionV0 {
            signature: raw_data_contract_create_transition
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_create_transition
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_data_contract_create_transition
                .remove_optional_bytes_32(ENTROPY)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_data_contract_create_transition
                    .remove(DATA_CONTRACT)
                    .ok_or(ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    ))?,
            )?,
            ..Default::default()
        })
    }

    pub fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    pub fn get_protocol_version(&self) -> u32 {
        0
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id]
    }

    #[cfg(feature = "platform-value")]
    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        value.replace_at_paths(super::IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(super::BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(super::U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }
}

impl From<DataContractCreateTransitionV0> for StateTransition {
    fn from(value: DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractCreateTransitionV0> for StateTransition {
    fn from(value: &DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.clone().into();
        transition.into()
    }
}

impl StateTransitionIdentitySigned for DataContractCreateTransitionV0 {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract.owner_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for DataContractCreateTransitionV0 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        DataContractCreate
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

impl StateTransitionConvert for DataContractCreateTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ENTROPY]
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        object.insert(String::from(DATA_CONTRACT), self.data_contract.to_object()?)?;
        Ok(object)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        object.insert(
            String::from(DATA_CONTRACT),
            self.data_contract.to_cleaned_object()?,
        )?;
        Ok(object)
    }
}

impl From<DataContract> for DataContractCreateTransitionV0 {
    fn from(value: DataContract) -> Self {
        DataContractCreateTransitionV0 {
            data_contract: value,
            entropy: Default::default(),
            signature_public_key_id: 0,
            signature: Default::default(),
        }
    }
}

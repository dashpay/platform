use anyhow::anyhow;
use serde_json::{json, Number, Value as JsonValue};
use std::collections::BTreeMap;
use std::convert::TryFrom;

use crate::{
    data_contract::{self, generate_data_contract_id},
    decode_protocol_entity_factory::DecodeProtocolEntity,
    errors::{consensus::ConsensusError, ProtocolError},
    mocks,
    prelude::Identifier,
    util::entropy_generator,
    Convertible,
};

use super::{
    state_transition::{DataContractCreateTransition, DataContractUpdateTransition},
    DataContract,
};
use data_contract::state_transition::properties as st_prop;

pub struct DataContractFactory {
    protocol_version: u32,
    _validate_data_contract: mocks::ValidateDataContract,
    // TODO remove dependency on decode_protocol_entity
}

impl DataContractFactory {
    pub fn new(
        protocol_version: u32,
        _validate_data_contract: mocks::ValidateDataContract,
        decode_protocol_entity: DecodeProtocolEntity,
    ) -> Self {
        Self {
            protocol_version,
            _validate_data_contract,
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
    ) -> Result<DataContract, ProtocolError> {
        let entropy = entropy_generator::generate();
        let data_contract_id =
            Identifier::from_bytes(&generate_data_contract_id(owner_id.to_buffer(), entropy))?;

        let mut data_contract = DataContract {
            protocol_version: self.protocol_version,
            schema: String::from(data_contract::SCHEMA),
            id: data_contract_id,
            version: 1,
            owner_id,
            defs: BTreeMap::new(),
            entropy,

            ..Default::default()
        };

        if let JsonValue::Object(documents) = documents {
            for (document_name, value) in documents {
                data_contract.set_document_schema(document_name, value);
            }
        } else {
            return Err(ProtocolError::Generic(String::from(
                "attached documents are not in form a map",
            )));
        }

        Ok(data_contract)
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        raw_data_contract: JsonValue,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        if !skip_validation {
            let result = self
                ._validate_data_contract
                .validate_data_contract(&raw_data_contract)
                .await;
            if !result.is_valid() {
                return Err(ProtocolError::InvalidDataContractError {
                    errors: result.errors,
                    raw_data_contract,
                });
            }
        }
        DataContract::try_from(raw_data_contract)
    }

    /// Create Data Contract from buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        let (protocol_version, mut raw_data_contract) =
            DecodeProtocolEntity::decode_protocol_entity(buffer)?;

        match raw_data_contract {
            JsonValue::Object(ref mut m) => m.insert(
                String::from("protocolVersion"),
                JsonValue::Number(Number::from(protocol_version)),
            ),
            _ => {
                return Err(ConsensusError::SerializedObjectParsingError {
                    parsing_error: anyhow!("the '{:?}' is not a map", raw_data_contract),
                }
                .into())
            }
        };

        self.create_from_object(raw_data_contract, skip_validation)
            .await
    }

    pub fn create_data_contract_create_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        DataContractCreateTransition::from_raw_object(json!({
            st_prop::PROPERTY_PROTOCOL_VERSION: self.protocol_version,
            st_prop::PROPERTY_DATA_CONTRACT: data_contract.to_object()?,
            st_prop::PROPERTY_ENTROPY: data_contract.entropy,
        }))
    }

    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        DataContractUpdateTransition::from_raw_object(json!({
            st_prop::PROPERTY_PROTOCOL_VERSION: self.protocol_version,
            st_prop::PROPERTY_DATA_CONTRACT: data_contract.to_object()?,
        }))
    }
}

#[cfg(test)]
mod test {}

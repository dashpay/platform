use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::{
    data_contract::{self, generate_data_contract_id},
    errors::ProtocolError,
    mocks,
    prelude::Identifier,
    util::entropy_generator,
};

use super::DataContract;

pub struct DataContractFactory {
    dpp: mocks::DashPlatformProtocol,
    _validate_data_contract: mocks::ValidateDataContract,
    _decode_protocol_entity: mocks::DecodeProtocolIdentity,
}

impl DataContractFactory {
    pub fn new(
        dpp: mocks::DashPlatformProtocol,
        _validate_data_contract: mocks::ValidateDataContract,
        _decode_protocol_entity: mocks::DecodeProtocolIdentity,
    ) -> Self {
        Self {
            dpp,
            _validate_data_contract,
            _decode_protocol_entity,
        }
    }

    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
    ) -> Result<DataContract, ProtocolError> {
        let entropy = entropy_generator::generate();
        let data_contract_id =
            Identifier::from_bytes(&generate_data_contract_id(owner_id.to_buffer(), entropy))?;

        let mut documents_map: BTreeMap<String, JsonValue> = BTreeMap::new();
        if let JsonValue::Object(documents) = documents {
            for (document_name, value) in documents {
                documents_map.insert(document_name, value);
            }
        } else {
            return Err(ProtocolError::Generic(String::from(
                "attached documents are not in form a map",
            )));
        }

        let data_contract = DataContract {
            protocol_version: self.dpp.get_protocol_version(),
            schema: String::from(data_contract::SCHEMA),
            id: data_contract_id,
            version: 1,
            owner_id,
            documents: documents_map,
            defs: None,
            entropy,

            ..Default::default()
        };
        Ok(data_contract)
    }

    // TODO  implement the rest of the constructors
}

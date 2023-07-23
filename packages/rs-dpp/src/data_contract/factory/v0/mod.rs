use derive_more::From;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;

use platform_value::{Bytes32, Error, Value};

use crate::data_contract::errors::InvalidDataContractError;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::DataContract;
use crate::serialization_traits::{
    PlatformDeserializable, PlatformDeserializableFromVersionedStructure,
};
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
#[cfg(feature = "state-transitions")]
use crate::state_transition::StateTransitionType;
use crate::util::entropy_generator::{DefaultEntropyGenerator, EntropyGenerator};
use crate::version::{FeatureVersion, PlatformVersion, LATEST_PLATFORM_VERSION};
use crate::{
    data_contract::{self},
    errors::ProtocolError,
    prelude::Identifier,
    Convertible,
};

/// The version 0 implementation of the data contract factory.
///
/// This implementation manages the creation, validation, and serialization of data contracts.
/// It uses a protocol_version, a DataContractValidator, and an EntropyGenerator for its operations.
pub struct DataContractFactoryV0 {
    /// The feature version used by this factory.
    protocol_version: u32,

    /// An EntropyGenerator for generating entropy during data contract creation.
    entropy_generator: Box<dyn EntropyGenerator>,
}

impl DataContractFactoryV0 {
    pub fn new(
        protocol_version: u32,
        entropy_generator: Option<Box<dyn EntropyGenerator>>,
    ) -> Self {
        Self {
            protocol_version,
            entropy_generator: entropy_generator.unwrap_or(Box::new(DefaultEntropyGenerator)),
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: Value,
        config: Option<Value>,
        definitions: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
        let entropy = Bytes32::new(self.entropy_generator.generate()?);

        let platform_version = PlatformVersion::get(self.protocol_version)?;

        let data_contract_id =
            DataContract::generate_data_contract_id_v0(owner_id.to_buffer(), entropy.to_buffer());

        let definition_references = definitions
            .as_ref()
            .map(|defs| defs.to_btree_ref_string_map())
            .transpose()
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();

        // We need to transform the value into a data contract config
        let config = if let Some(config_value) = config {
            DataContractConfig::from_value(config_value, platform_version)?
        } else {
            DataContractConfig::default_for_version(platform_version)?
        };

        let document_types = DataContract::get_document_types_from_value(
            data_contract_id,
            &documents,
            &definition_references,
            config.documents_keep_history_contract_default(),
            config.documents_mutable_contract_default(),
            platform_version,
        )
        .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;

        let document_values = documents
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let documents = document_values
            .into_iter()
            .map(|(key, value)| Ok((key, value.try_into().map_err(ProtocolError::ValueError)?)))
            .collect::<Result<BTreeMap<String, JsonValue>, ProtocolError>>()?;

        let json_defs = if !definition_references.is_empty() {
            Some(
                definition_references
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            key,
                            value
                                .clone()
                                .try_into()
                                .map_err(ProtocolError::ValueError)?,
                        ))
                    })
                    .collect::<Result<BTreeMap<String, JsonValue>, ProtocolError>>()?,
            )
        } else {
            None
        };

        let mut data_contract: DataContract = match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => DataContractV0 {
                id: data_contract_id,
                schema: data_contract::DATA_CONTRACT_SCHEMA_URI_V0.to_string(),
                version: 1,
                owner_id,
                document_types,
                metadata: None,
                config,
                documents,
                defs: json_defs,
                binary_properties: Default::default(),
            }
            .into(),
            version => {
                return Err(ProtocolError::UnknownVersionMismatch {
                    method: "DataContractFactoryV0::create (for data_contract structure version)"
                        .to_string(),
                    known_versions: vec![0],
                    received: version,
                })
            }
        };

        data_contract.generate_binary_properties(platform_version)?;

        CreatedDataContract::from_contract_and_entropy(data_contract, entropy, platform_version)
    }

    #[cfg(feature = "platform-value")]
    /// Create Data Contract from plain object
    pub fn create_from_object(
        &self,
        mut data_contract_object: Value,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        #[cfg(feature = "validation")]
        {
            if !skip_validation {
                self.validate_data_contract(&data_contract_object)?;
            }
        }
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_object(data_contract_object, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractFactoryV0::create_from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Create Data Contract from buffer
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        let data_contract: DataContract = DataContract::versioned_deserialize(
            buffer.as_slice(),
            platform_version,
        )
        .map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;

        #[cfg(feature = "validation")]
        {
            if !skip_validation {
                self.validate_data_contract(&data_contract.to_cleaned_object()?)?;
            }
        }

        Ok(data_contract)
    }

    #[cfg(feature = "validation")]
    pub fn validate_data_contract(&self, raw_data_contract: &Value) -> Result<(), ProtocolError> {
        let result =
            DataContract::validate(self.data_contract_feature_version, raw_data_contract, false)?;

        if !result.is_valid() {
            return Err(ProtocolError::InvalidDataContractError(
                InvalidDataContractError::new(result.errors, raw_data_contract.to_owned()),
            ));
        }

        Ok(())
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_data_contract_create_transition(
        &self,
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        Ok(created_data_contract.into())
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;

        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .default_current_version
        {
            0 => DataContractUpdateTransitionV0 {
                data_contract,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_json_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::property_names;
    use crate::serialization_traits::PlatformSerializable;
    use crate::state_transition::StateTransitionLike;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::Convertible;

    pub struct TestData {
        created_data_contract: CreatedDataContract,
        raw_data_contract: Value,
        factory: DataContractFactoryV0,
    }

    fn get_test_data() -> TestData {
        let platform_version = PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, platform_version.protocol_version);
        let raw_data_contract = created_data_contract
            .into_data_contract()
            .as_v0()
            .unwrap()
            .to_object()
            .unwrap();

        let factory = DataContractFactoryV0::new(platform_version.protocol_version, None);
        TestData {
            created_data_contract,
            raw_data_contract,
            factory,
        }
    }

    #[test]
    fn should_create_data_contract_with_specified_name_and_docs_definition() {
        let TestData {
            created_data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let raw_defs = raw_data_contract
            .get_value(property_names::DEFINITIONS)
            .expect("documents property should exist")
            .clone();

        let raw_documents = raw_data_contract
            .get_value(property_names::DOCUMENTS)
            .expect("documents property should exist")
            .clone();

        let result = factory
            .create(data_contract.owner_id, raw_documents, None, Some(raw_defs))
            .expect("Data Contract should be created")
            .data_contract;

        assert_eq!(
            data_contract.data_contract_protocol_version,
            result.data_contract_protocol_version
        );
        // id is generated based on entropy which is different every time the `create` call is used
        assert_eq!(data_contract.id.len(), result.id.len());
        assert_ne!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
    }

    #[tokio::test]
    async fn should_crate_data_contract_from_object() {
        let TestData {
            created_data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let result = factory
            .create_from_object(raw_data_contract.into(), true)
            .await
            .expect("Data Contract should be created");

        assert_eq!(
            data_contract.data_contract_protocol_version,
            result.data_contract_protocol_version
        );
        assert_eq!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
        assert_eq!(data_contract.defs, result.defs);
    }

    #[tokio::test]
    async fn should_create_data_contract_from_buffer() {
        let TestData {
            created_data_contract,
            factory,
            ..
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let serialized_data_contract = data_contract
            .serialize()
            .expect("should be serialized to buffer");
        let result = factory
            .create_from_buffer(serialized_data_contract, false)
            .await
            .expect("Data Contract should be created from the buffer");

        assert_eq!(
            data_contract.data_contract_protocol_version,
            result.data_contract_protocol_version
        );
        assert_eq!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
        assert_eq!(data_contract.defs, result.defs);
    }

    #[test]
    fn should_create_data_contract_create_transition_from_data_contract() {
        let TestData {
            created_data_contract,
            factory,
            raw_data_contract,
        } = get_test_data();

        let result = factory
            .create_data_contract_create_transition(created_data_contract.clone())
            .expect("Data Contract Transition should be created");

        assert_eq!(1, result.state_transition_protocol_version());
        assert_eq!(&created_data_contract.entropy_used, &result.entropy);
        assert_eq!(
            raw_data_contract,
            result.data_contract().to_object().unwrap()
        );
    }
}

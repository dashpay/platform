use platform_value::{Bytes32, Value};
use platform_version::TryFromPlatformVersioned;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;

use crate::data_contract::config::DataContractConfig;
#[cfg(feature = "data-contract-value-conversion")]
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;

use crate::util::entropy_generator::{DefaultEntropyGenerator, EntropyGenerator};
use crate::version::PlatformVersion;
use crate::{errors::ProtocolError, prelude::Identifier};

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

        let defs = definitions
            .map(|defs| defs.into_btree_string_map())
            .transpose()
            .map_err(ProtocolError::ValueError)?;

        // We need to transform the value into a data contract config
        let config = if let Some(config_value) = config {
            DataContractConfig::from_value(config_value, platform_version)?
        } else {
            DataContractConfig::default_for_version(platform_version)?
        };

        let documents_map = documents
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let format = DataContractInSerializationFormat::V0(DataContractInSerializationFormatV0 {
            id: data_contract_id,
            config,
            version: 1,
            owner_id,
            document_schemas: documents_map,
            schema_defs: defs,
        });

        let data_contract =
            DataContract::try_from_platform_versioned(format, true, platform_version)?;

        CreatedDataContract::from_contract_and_entropy(data_contract, entropy, platform_version)
    }

    #[cfg(feature = "data-contract-value-conversion")]
    /// Create Data Contract from plain object
    pub fn create_from_object(
        &self,
        data_contract_object: Value,
        #[cfg(feature = "validation")] _skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        // TODO: We validate Data Contract on creation.
        //   Should we disable it when flag is off?
        // #[cfg(feature = "validation")]
        // {
        //     if !skip_validation {
        //         self.validate_data_contract(&data_contract_object)?;
        //     }
        // }
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_value(data_contract_object, platform_version)?.into()),
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
        #[cfg(not(feature = "validation"))]
        let skip_validation = true;

        let data_contract: DataContract = DataContract::versioned_deserialize(
            buffer.as_slice(),
            !skip_validation,
            platform_version,
        )
        .map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;

        // TODO: We validate Data Contract on creation.
        //   Should we disable it when flag is off?
        // #[cfg(feature = "validation")]
        // {
        //     if !skip_validation {
        //         self.validate_data_contract(&data_contract.to_cleaned_object()?)?;
        //     }
        // }

        Ok(data_contract)
    }

    // TODO: We validate Data Contract on creation.
    //   Should we disable it when flag is off?
    // #[cfg(feature = "validation")]
    // pub fn validate_data_contract(&self, raw_data_contract: &Value) -> Result<(), ProtocolError> {
    //     let platform_version = PlatformVersion::get(self.protocol_version)?;
    //     let data_contract = DataContract::from_object(raw_data_contract.clone(), platform_version)?;
    //     let result = data_contract.validate_schema(platform_version)?;
    //
    //     if !result.is_valid() {
    //         return Err(ProtocolError::InvalidDataContractError(
    //             InvalidDataContractError::new(result.errors, raw_data_contract.to_owned()),
    //         ));
    //     }
    //
    //     Ok(())
    // }

    #[cfg(feature = "state-transitions")]
    pub fn create_unsigned_data_contract_create_transition(
        &self,
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        DataContractCreateTransition::try_from_platform_versioned(
            created_data_contract,
            PlatformVersion::get(self.protocol_version)?,
        )
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_unsigned_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        DataContractUpdateTransition::try_from_platform_versioned(
            data_contract,
            PlatformVersion::get(self.protocol_version)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::serialized_version::v0::property_names;
    use crate::data_contract::DataContractMethodsV0;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
    use crate::state_transition::StateTransitionLike;
    use crate::tests::fixtures::get_data_contract_fixture;

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
            .data_contract()
            .to_value(platform_version)
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

        let data_contract = created_data_contract.data_contract_owned();

        let raw_defs = raw_data_contract
            .get_value(property_names::DEFINITIONS)
            .expect("documents property should exist")
            .clone();

        let raw_documents = raw_data_contract
            .get_value(property_names::DOCUMENTS)
            .expect("documents property should exist")
            .clone();

        let result = factory
            .create(
                data_contract.owner_id(),
                raw_documents,
                None,
                Some(raw_defs),
            )
            .expect("Data Contract should be created")
            .data_contract_owned();

        assert_eq!(data_contract.version(), result.version());
        // id is generated based on entropy which is different every time the `create` call is used
        assert_eq!(data_contract.id().len(), result.id().len());
        assert_ne!(data_contract.id(), result.id());
        assert_eq!(data_contract.document_schemas(), result.document_schemas());
        assert_eq!(data_contract.owner_id(), result.owner_id());
        assert_eq!(data_contract.document_types(), result.document_types());
        assert_eq!(data_contract.metadata(), result.metadata());
    }

    #[tokio::test]
    async fn should_crate_data_contract_from_object() {
        let _platform_version = PlatformVersion::latest();

        let TestData {
            created_data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let data_contract = created_data_contract.data_contract_owned();

        let result = factory
            .create_from_object(raw_data_contract.into(), false)
            .expect("Data Contract should be created");

        assert_eq!(data_contract.version(), result.version());
        assert_eq!(data_contract.id(), result.id());
        assert_eq!(data_contract.owner_id(), result.owner_id());
        assert_eq!(data_contract.document_types(), result.document_types());
        assert_eq!(data_contract.metadata(), result.metadata());
    }

    #[tokio::test]
    async fn should_create_data_contract_from_buffer() {
        let platform_version = PlatformVersion::latest();

        let TestData {
            created_data_contract,
            factory,
            ..
        } = get_test_data();

        let data_contract = created_data_contract.data_contract_owned();

        let serialized_data_contract = data_contract
            .serialize_with_platform_version(&platform_version)
            .expect("should be serialized to buffer");
        let result = factory
            .create_from_buffer(serialized_data_contract, false)
            .expect("Data Contract should be created from the buffer");

        assert_eq!(data_contract.version(), result.version());
        assert_eq!(data_contract.id(), result.id());
        assert_eq!(data_contract.owner_id(), result.owner_id());
        assert_eq!(data_contract.document_types(), result.document_types());
        assert_eq!(data_contract.metadata(), result.metadata());
    }

    #[test]
    fn should_create_data_contract_create_transition_from_data_contract() {
        let platform_version = PlatformVersion::latest();

        let TestData {
            created_data_contract,
            factory,
            raw_data_contract,
        } = get_test_data();

        let result = factory
            .create_unsigned_data_contract_create_transition(created_data_contract.clone())
            .expect("Data Contract Transition should be created");

        assert_eq!(1, result.state_transition_protocol_version());
        assert_eq!(&created_data_contract.entropy_used(), &result.entropy());

        let contract_value = DataContract::try_from_platform_versioned(
            result.data_contract().to_owned(),
            false,
            &platform_version,
        )
        .unwrap()
        .to_value(&platform_version)
        .unwrap();

        assert_eq!(raw_data_contract, contract_value);
    }
}

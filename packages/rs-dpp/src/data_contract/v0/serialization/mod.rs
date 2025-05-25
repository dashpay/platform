use crate::data_contract::document_type::DocumentType;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::version::PlatformVersionCurrentVersion;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::validation::operations::ProtocolValidationOperation;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for DataContractV0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data_contract: DataContract = self.clone().into();
        let serialization_format = DataContractInSerializationFormatV0::from(data_contract);
        serialization_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataContractV0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialization_format = DataContractInSerializationFormatV0::deserialize(deserializer)?;
        let current_version = PlatformVersion::get_current().map_err(|e| {
            serde::de::Error::custom(format!(
                "expected to be able to get current platform version: {}",
                e
            ))
        })?;
        // when deserializing from json/platform_value/cbor we always want to validate (as this is not coming from the state)
        DataContractV0::try_from_platform_versioned_v0(
            serialization_format,
            true,
            &mut vec![],
            current_version,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl DataContractV0 {
    pub(in crate::data_contract) fn try_from_platform_versioned(
        value: DataContractInSerializationFormat,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractInSerializationFormat::V0(serialization_format_v0) => {
                let data_contract = DataContractV0::try_from_platform_versioned_v0(
                    serialization_format_v0,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?;

                Ok(data_contract)
            }
            DataContractInSerializationFormat::V1(serialization_format_v1) => {
                let data_contract = DataContractV0::try_from_platform_versioned_v1(
                    serialization_format_v1,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?;

                Ok(data_contract)
            }
        }
    }

    pub(in crate::data_contract) fn try_from_platform_versioned_v0(
        data_contract_data: DataContractInSerializationFormatV0,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DataContractInSerializationFormatV0 {
            id,
            config,
            version,
            owner_id,
            document_schemas,
            schema_defs,
        } = data_contract_data;

        let document_types = DocumentType::create_document_types_from_document_schemas(
            id,
            document_schemas,
            schema_defs.as_ref(),
            &BTreeMap::new(),
            &config,
            full_validation,
            false,
            validation_operations,
            platform_version,
        )?;

        let data_contract = DataContractV0 {
            id,
            version,
            owner_id,
            document_types,
            metadata: None,
            config,
            schema_defs,
        };

        Ok(data_contract)
    }

    pub(in crate::data_contract) fn try_from_platform_versioned_v1(
        data_contract_data: DataContractInSerializationFormatV1,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DataContractInSerializationFormatV1 {
            id,
            config,
            version,
            owner_id,
            document_schemas,
            schema_defs,
            ..
        } = data_contract_data;

        let document_types = DocumentType::create_document_types_from_document_schemas(
            id,
            document_schemas,
            schema_defs.as_ref(),
            &BTreeMap::new(),
            &config,
            full_validation,
            false,
            validation_operations,
            platform_version,
        )?;

        let data_contract = DataContractV0 {
            id,
            version,
            owner_id,
            document_types,
            metadata: None,
            config,
            schema_defs,
        };

        Ok(data_contract)
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::DataContract;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::Identity;
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    };
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::PlatformVersion;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    #[test]
    #[cfg(feature = "random-identities")]
    fn data_contract_ser_de() {
        let platform_version = PlatformVersion::first();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let contract =
            get_data_contract_fixture(Some(identity.id()), 0, platform_version.protocol_version)
                .data_contract_owned();
        let bytes = contract
            .serialize_to_bytes_with_platform_version(LATEST_PLATFORM_VERSION)
            .expect("expected to serialize");
        let recovered_contract =
            DataContract::versioned_deserialize(&bytes, false, platform_version)
                .expect("expected to deserialize state transition");
        assert_eq!(contract, recovered_contract);
    }
}

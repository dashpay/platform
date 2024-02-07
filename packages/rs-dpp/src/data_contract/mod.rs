use crate::serialization::{
    PlatformDeserializableWithBytesLenFromVersionedStructure,
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformLimitDeserializableFromVersionedStructure, PlatformSerializableWithPlatformVersion,
};

pub use data_contract::*;
use derive_more::From;

use bincode::config::{BigEndian, Configuration};
pub use generate_data_contract::*;

pub mod errors;
pub mod extra;

mod generate_data_contract;

#[cfg(any(feature = "state-transitions", feature = "factories"))]
pub mod created_data_contract;
pub mod document_type;

mod v0;

#[cfg(feature = "factories")]
mod factory;
#[cfg(feature = "factories")]
pub use factory::*;
pub mod conversion;
#[cfg(feature = "client")]
mod data_contract_facade;
#[cfg(feature = "client")]
pub use data_contract_facade::DataContractFacade;
mod methods;
pub mod serialized_version;
pub use methods::*;
pub mod accessors;
pub mod config;
pub mod storage_requirements;

pub use v0::*;

use crate::data_contract::serialized_version::{
    DataContractInSerializationFormat, CONTRACT_DESERIALIZATION_LIMIT,
};
use crate::util::hash::hash_double_to_vec;

use crate::version::{FeatureVersion, PlatformVersion};
use crate::ProtocolError;
use crate::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};

use platform_version::TryIntoPlatformVersioned;
use platform_versioning::PlatformVersioned;
pub use serde_json::Value as JsonValue;

type JsonSchema = JsonValue;
type DefinitionName = String;
pub type DocumentName = String;
type PropertyPath = String;

/// Understanding Data Contract versioning
/// Data contract versioning is both for the code structure and for serialization.
///
/// The code structure is what is used in code to verify documents and is used in memory
/// There is generally only one code structure running at any given time, except in the case we
/// are switching protocol versions.
///
/// There can be a lot of serialization versions that are active, and serialization versions
/// should generally always be supported. This is because when we store something as version 1.
/// 10 years down the line when we unserialize this contract it will still be in version 1.
/// Deserialization of a data contract serialized in that version should be translated to the
/// current code structure version.
///
/// There are some scenarios to consider,
///
/// One such scenario is that the serialization version does not contain enough information for the
/// current code structure version.
///
/// Depending on the situation one of the following occurs:
/// - the contract structure can imply missing parts based on default behavior
/// - the contract structure can disable certain features dependant on missing information
/// - the contract might be unusable until it is updated by the owner
///

/// Here we use PlatformSerialize, because
#[derive(Debug, Clone, PartialEq, From, PlatformVersioned)]
pub enum DataContract {
    V0(DataContractV0),
}

impl PlatformSerializableWithPlatformVersion for DataContract {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let serialization_format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize DataContract: {}", e))
        })
    }

    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let serialization_format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize consume DataContract: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for DataContract {
    fn versioned_deserialize(
        data: &[u8],
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let data_contract_in_serialization_format: DataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "unable to deserialize DataContract: {}",
                        e
                    ))
                })?
                .0;
        DataContract::try_from_platform_versioned(
            data_contract_in_serialization_format,
            validate,
            platform_version,
        )
    }
}

impl PlatformDeserializableWithBytesLenFromVersionedStructure for DataContract {
    fn versioned_deserialize_with_bytes_len(
        data: &[u8],
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, usize), ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let (data_contract_in_serialization_format, len) = bincode::borrow_decode_from_slice::<
            DataContractInSerializationFormat,
            Configuration<BigEndian>,
        >(data, config)
        .map_err(|e| {
            PlatformDeserializationError(format!("unable to deserialize DataContract: {}", e))
        })?;
        Ok((
            DataContract::try_from_platform_versioned(
                data_contract_in_serialization_format,
                validate,
                platform_version,
            )?,
            len,
        ))
    }
}

impl PlatformLimitDeserializableFromVersionedStructure for DataContract {
    fn versioned_limit_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<CONTRACT_DESERIALIZATION_LIMIT>();
        let data_contract_in_serialization_format: DataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "unable to deserialize DataContract with limit: {}",
                        e
                    ))
                })?
                .0;
        // we always want to validate when we have a limit, because limit means the data isn't coming from Drive
        DataContract::try_from_platform_versioned(
            data_contract_in_serialization_format,
            true,
            platform_version,
        )
    }
}

impl DataContract {
    pub fn as_v0(&self) -> Option<&DataContractV0> {
        match self {
            DataContract::V0(v0) => Some(v0),
        }
    }

    pub fn as_v0_mut(&mut self) -> Option<&mut DataContractV0> {
        match self {
            DataContract::V0(v0) => Some(v0),
        }
    }

    pub fn into_v0(self) -> Option<DataContractV0> {
        match self {
            DataContract::V0(v0) => Some(v0),
        }
    }

    pub fn check_version_is_active(
        protocol_version: u32,
        data_contract_system_version: FeatureVersion,
    ) -> Result<bool, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        Ok(platform_version
            .dpp
            .contract_versions
            .contract_structure_version
            == data_contract_system_version)
    }

    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_double_to_vec(
            self.serialize_to_bytes_with_platform_version(platform_version)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::config::v0::DataContractConfigGettersV0;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use crate::data_contract::DataContract;
    use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use crate::system_data_contracts::load_system_data_contract;
    use crate::tests::fixtures::{
        get_dashpay_contract_fixture, get_dashpay_contract_with_generalized_encryption_key_fixture,
    };
    use crate::version::PlatformVersion;
    use data_contracts::SystemDataContract::Dashpay;

    #[test]
    fn test_contract_serialization() {
        let platform_version = PlatformVersion::latest();
        let data_contract = load_system_data_contract(Dashpay, platform_version)
            .expect("expected dashpay contract");
        let platform_version = PlatformVersion::latest();
        let serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected to serialize data contract");
        assert_eq!(
            serialized[0],
            platform_version
                .dpp
                .contract_versions
                .contract_serialization_version
                .default_current_version as u8
        );

        let unserialized = DataContract::versioned_deserialize(&serialized, true, platform_version)
            .expect("expected to deserialize data contract");

        assert_eq!(data_contract, unserialized);
    }

    #[test]
    fn test_contract_can_have_specialized_contract_encryption_decryption_keys() {
        let data_contract = get_dashpay_contract_with_generalized_encryption_key_fixture(None, 1)
            .data_contract_owned();
        assert_eq!(
            data_contract
                .config()
                .requires_identity_decryption_bounded_key(),
            Some(StorageKeyRequirements::Unique)
        );
        assert_eq!(
            data_contract
                .config()
                .requires_identity_encryption_bounded_key(),
            Some(StorageKeyRequirements::Unique)
        );
    }

    #[test]
    fn test_contract_document_type_can_have_specialized_contract_encryption_decryption_keys() {
        let data_contract = get_dashpay_contract_fixture(None, 1).data_contract_owned();
        assert_eq!(
            data_contract
                .document_type_for_name("contactRequest")
                .expect("expected document type")
                .requires_identity_decryption_bounded_key(),
            Some(StorageKeyRequirements::MultipleReferenceToLatest)
        );
        assert_eq!(
            data_contract
                .document_type_for_name("contactRequest")
                .expect("expected document type")
                .requires_identity_encryption_bounded_key(),
            Some(StorageKeyRequirements::MultipleReferenceToLatest)
        );
    }
}

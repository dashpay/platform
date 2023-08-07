use crate::serialization::{
    PlatformDeserializableFromVersionedStructure,
    PlatformDeserializableWithBytesLenFromVersionedStructure,
    PlatformLimitDeserializableFromVersionedStructure, PlatformSerializable,
    PlatformSerializableWithPlatformVersion,
};
use bincode::{config, Decode, Encode};
pub use data_contract::*;
use derive_more::From;

use bincode::config::{BigEndian, Configuration};
use bincode::enc::Encoder;
pub use generate_data_contract::*;
use platform_value::{Identifier, Value, ValueMapHelper};
use serde::Serialize;
use std::collections::BTreeMap;
use std::convert::TryInto;

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
mod data_contract_class_methods;
pub use data_contract_class_methods::*;
pub mod conversion;
#[cfg(feature = "client")]
mod data_contract_facade;
mod data_contract_methods;
pub mod serialized_version;
pub use data_contract_methods::*;
pub mod accessors;
pub mod data_contract_config;

pub use v0::*;

use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::serialized_version::{
    DataContractInSerializationFormat, CONTRACT_DESERIALIZATION_LIMIT,
};
use crate::data_contract::v0::data_contract::DataContractV0;
use crate::util::hash::hash_to_vec;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::{FeatureVersion, PlatformVersion};
use crate::ProtocolError;
use crate::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};
pub use serde_json::Value as JsonValue;

pub mod property_names {
    pub const ID: &str = "$id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const SCHEMA: &str = "$schema";
    pub const DOCUMENTS: &str = "documents";
    pub const DEFINITIONS: &str = "$defs";
    pub const ENTROPY: &str = "entropy"; // not a data contract field actually but at some point it can be there for some time
}

type JsonSchema = JsonValue;
type DefinitionName = String;
pub type DocumentName = String;
type PropertyPath = String;

pub trait DataContractLike<'a> {
    fn id() -> Identifier;
    fn owner_id() -> Identifier;
    fn contract_version() -> u32;
    fn document_types() -> BTreeMap<DocumentName, DocumentTypeRef<'a>>;
}

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
#[derive(Debug, Clone, PartialEq, From)] //PlatformSerdeVersionedSerialize, PlatformSerdeVersionedDeserialize,
                                         // #[serde(untagged)]
                                         // #[platform_serde_versioned(version_field = "$version")]
pub enum DataContract {
    //#[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(DataContractV0),
}

impl PlatformSerializableWithPlatformVersion for DataContract {
    fn serialize_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let serialization_format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        let config = config::standard().with_big_endian().with_no_limit();
        bincode::encode_to_vec(serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize DataContract: {}", e))
        })
    }

    fn serialize_consume_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let serialization_format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        let config = config::standard().with_big_endian().with_no_limit();
        bincode::encode_to_vec(serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize consume DataContract: {}", e))
        })
    }
}

impl PlatformDeserializableFromVersionedStructure for DataContract {
    fn versioned_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = config::standard().with_big_endian().with_no_limit();
        let data_contract_in_serialization_format: DataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "unable to deserialize DataContract: {}",
                        e
                    ))
                })?
                .0;
        data_contract_in_serialization_format.try_into_platform_versioned(platform_version)
    }
}

impl PlatformDeserializableWithBytesLenFromVersionedStructure for DataContract {
    fn versioned_deserialize_with_bytes_len(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(Self, usize), ProtocolError>
    where
        Self: Sized,
    {
        let config = config::standard().with_big_endian().with_no_limit();
        let (data_contract_in_serialization_format, len) = bincode::borrow_decode_from_slice::<
            DataContractInSerializationFormat,
            Configuration<BigEndian>,
        >(data, config)
        .map_err(|e| {
            PlatformDeserializationError(format!("unable to deserialize DataContract: {}", e))
        })?;
        Ok((
            data_contract_in_serialization_format.try_into_platform_versioned(platform_version)?,
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
        let config = config::standard()
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
        data_contract_in_serialization_format.try_into_platform_versioned(platform_version)
    }
}

impl DataContract {
    // TODO: Don't we need this method in DataContractV0?
    // Returns hash from Data Contract
    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_to_vec(
            self.serialize_with_platform_version(platform_version)?,
        ))
    }

    pub fn as_v0(&self) -> Option<&DataContractV0> {
        match self {
            DataContract::V0(v0) => Some(v0),
            _ => None,
        }
    }

    pub fn as_v0_mut(&mut self) -> Option<&mut DataContractV0> {
        match self {
            DataContract::V0(v0) => Some(v0),
            _ => None,
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

    // TODO: Remove
    // #[cfg(feature = "validation")]
    // pub fn validate(
    //     protocol_version: u32,
    //     raw_data_contract: &Value,
    //     allow_non_current_data_contract_versions: bool,
    // ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    //     let data_contract_system_version =
    //         match raw_data_contract.get_optional_integer::<FeatureVersion>(SYSTEM_VERSION) {
    //             Ok(Some(data_contract_system_version)) => data_contract_system_version,
    //             Ok(None) => {
    //                 return Ok(SimpleConsensusValidationResult::new_with_error(
    //                     ConsensusError::BasicError(BasicError::VersionError(
    //                         "no system version found on data contract object".into(),
    //                     )),
    //                 ));
    //             }
    //             Err(e) => {
    //                 return Ok(SimpleConsensusValidationResult::new_with_error(
    //                     ConsensusError::BasicError(BasicError::VersionError(
    //                         format!("version error: {}", e.to_string()).into(),
    //                     )),
    //                 ));
    //             }
    //         };
    //     if !allow_non_current_data_contract_versions {
    //         Self::check_version_is_active(protocol_version, data_contract_system_version)?;
    //     }
    //     match data_contract_system_version {
    //         0 => DataContractV0::validate(raw_data_contract),
    //         _ => Ok(SimpleConsensusValidationResult::new_with_error(
    //             ConsensusError::BasicError(BasicError::VersionError(
    //                 "system version found on data contract object".into(),
    //             )),
    //         )),
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::v0::DataContractV0;
    use crate::data_contract::DataContract;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use crate::system_data_contracts::load_system_data_contract;
    use crate::version::PlatformVersion;
    use data_contracts::SystemDataContract::Dashpay;
    use serde::Serialize;

    #[test]
    fn test_contract_serialization() {
        let platform_version = PlatformVersion::latest();
        let data_contract = load_system_data_contract(Dashpay, platform_version.protocol_version)
            .expect("expected dashpay contract");
        let platform_version = PlatformVersion::latest();
        let serialized = data_contract
            .serialize_with_platform_version(platform_version)
            .expect("expected to serialize data contract");
        assert_eq!(
            serialized[0],
            platform_version.contract.default_current_version
        );

        let unserialized = DataContract::deserialize_with_platform_version(platform_version);
        assert_eq!(data_contract, unserialized);
    }
}

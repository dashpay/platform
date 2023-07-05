use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use bincode::{config, Decode, Encode};
pub use data_contract::*;
use derive_more::From;

pub use generate_data_contract::*;
use platform_serialization::{PlatformDeserialize, PlatformDeserializeNoLimit, PlatformSerialize};
use platform_value::{Identifier, Value};
use serde::Serialize;
use std::collections::BTreeMap;

pub mod errors;
pub mod extra;

mod generate_data_contract;

#[cfg(feature = "state-transitions")]
pub mod created_data_contract;
pub mod document_type;

mod v0;

#[cfg(feature = "client")]
mod factory;
#[cfg(feature = "client")]
pub use factory::*;
#[cfg(feature = "client")]
mod data_contract_facade;
mod data_contract_class_methods;

#[cfg(feature = "state-transitions")]
pub use created_data_contract::CreatedDataContract;
pub use v0::*;

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::property_names::SYSTEM_VERSION;
use crate::data_contract::v0::data_contract::DataContractV0;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::{FeatureVersion, PlatformVersion};
use crate::ProtocolError;
use platform_versioning::PlatformSerdeVersionedDeserialize;
use crate::util::hash::hash_to_vec;

pub mod property_names {
    pub const SYSTEM_VERSION: &str = "systemVersion";
    pub const ID: &str = "$id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const SCHEMA: &str = "$schema";
    pub const DOCUMENTS: &str = "documents";
    pub const DEFINITIONS: &str = "$defs";
    pub const ENTROPY: &str = "entropy"; // not a data contract field actually but at some point it can be there for some time
}

pub trait DataContractLike<'a> {
    fn id() -> Identifier;
    fn owner_id() -> Identifier;
    fn contract_version() -> u32;
    fn document_types() -> BTreeMap<DocumentName, DocumentTypeRef<'a>>;
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    Serialize,
    PlatformSerdeVersionedDeserialize,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformDeserializeNoLimit,
    From,
)]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(15000)]
#[platform_serialize_limit(15000)]
#[serde(untagged)]
pub enum DataContract {
    #[versioned(0)]
    V0(DataContractV0),
}

impl Default for DataContract {
    fn default() -> Self {
        DataContract::V0(DataContractV0::default())
    }
}

impl DataContract {

    // Returns hash from Data Contract
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_to_vec(self.serialize()?))
    }

    pub fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id,
        }
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
            .contract
            .check_version(data_contract_system_version))
    }

    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(mut raw_object: Value) -> Result<DataContract, ProtocolError> {
        let data_contract_system_version =
            match raw_object.remove_optional_integer::<FeatureVersion>(SYSTEM_VERSION) {
                Ok(Some(data_contract_system_version)) => data_contract_system_version,
                Ok(None) => {
                    return Err(ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::VersionError(
                            "no system version found on data contract object".into(),
                        ))
                        .into(),
                    ));
                }
                Err(e) => {
                    return Err(ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::VersionError(
                            format!("version error: {}", e.to_string()).into(),
                        ))
                        .into(),
                    ));
                }
            };
        match data_contract_system_version {
            0 => Ok(DataContractV0::from_raw_object(raw_object).into()),
            _ => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::VersionError(
                    "system version found on data contract object".into(),
                ))
                .into(),
            )),
        }
    }

    #[cfg(feature = "validation")]
    pub fn validate(
        protocol_version: u32,
        raw_data_contract: &Value,
        allow_non_current_data_contract_versions: bool,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let data_contract_system_version =
            match raw_data_contract.get_optional_integer::<FeatureVersion>(SYSTEM_VERSION) {
                Ok(Some(data_contract_system_version)) => data_contract_system_version,
                Ok(None) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::VersionError(
                            "no system version found on data contract object".into(),
                        )),
                    ));
                }
                Err(e) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::VersionError(
                            format!("version error: {}", e.to_string()).into(),
                        )),
                    ));
                }
            };
        if !allow_non_current_data_contract_versions {
            Self::check_version_is_active(protocol_version, data_contract_system_version)?;
        }
        match data_contract_system_version {
            0 => DataContractV0::validate(raw_data_contract),
            _ => Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::VersionError(
                    "system version found on data contract object".into(),
                )),
            )),
        }
    }
}

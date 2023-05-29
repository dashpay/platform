use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use bincode::{config, BorrowDecode, Decode, Encode};
pub use data_contract::*;
pub use factory::*;
use derive_more::From;
pub use generate_data_contract::*;
use platform_serialization::{PlatformDeserialize, PlatformDeserializeNoLimit, PlatformSerialize};
use platform_value::Value;

mod data_contract_facade;

pub mod errors;
pub mod extra;

mod generate_data_contract;
pub mod state_transition;

mod factory;
mod v0;
pub mod created_data_contract;

pub use v0::*;

use crate::data_contract::v0::data_contract::DataContractV0;
use crate::version::{FeatureVersion, LATEST_PLATFORM_VERSION, PlatformVersion};
use crate::ProtocolError;
use platform_versioning::PlatformVersioned;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::property_names::{FEATURE_VERSION, SYSTEM_VERSION};
use crate::validation::SimpleConsensusValidationResult;

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

#[derive(
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformVersioned,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformDeserializeNoLimit,
    From,
)]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(15000)]
#[platform_serialize_limit(15000)]
pub enum DataContract {
    V0(DataContractV0),
}

impl Default for DataContract {
    fn default() -> Self {
        DataContract::V0(DataContractV0::default())
    }
}

impl DataContract {
    pub fn check_version_is_active(protocol_version: u32, data_contract_system_version: FeatureVersion) -> Result<bool, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        Ok(platform_version.contract.check_version(data_contract_system_version))
    }

    pub fn validate(
        active_protocol_version: u32,
        raw_data_contract: &Value,
        allow_non_current_data_contract_versions: bool,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let data_contract_system_version = match raw_data_contract.get_optional_integer::<FeatureVersion>(SYSTEM_VERSION) {
            Ok(Some(data_contract_system_version)) => { data_contract_system_version }
            Ok(None) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(BasicError::VersionError("no system version found on data contract object".into()))));
            }
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(BasicError::VersionError(format!("version error: {}", e.to_string()).into()))));
            }

        };
        if !allow_non_current_data_contract_versions {
            Self::check_version_is_active(active_protocol_version, data_contract_system_version)?;
        }
        match system_version {
            0 => { DataContractV0::validate(raw_data_contract)}
            _ => Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(BasicError::VersionError("system version found on data contract object".into()))))
        }
    }
}
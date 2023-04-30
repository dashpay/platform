use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use bincode::{BorrowDecode, Decode, Encode};
pub use data_contract::*;
pub use data_contract_factory::*;
use derive_more::From;
pub use generate_data_contract::*;
use platform_serialization::{PlatformDeserialize, PlatformDeserializeNoLimit, PlatformSerialize};
mod data_contract_facade;

pub mod errors;
pub mod extra;

mod generate_data_contract;
pub mod state_transition;

mod factory;
mod v0;

pub use v0::*;

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const ID: &str = "$id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const SCHEMA: &str = "$schema";
    pub const DOCUMENTS: &str = "documents";
    pub const DEFINITIONS: &str = "$defs";
    pub const ENTROPY: &str = "entropy"; // not a data contract field actually but at some point it can be there for some time
}

use crate::data_contract::v0::data_contract::DataContractV0;
use crate::version::LATEST_PLATFORM_VERSION;
use platform_versioning::PlatformVersioned;

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

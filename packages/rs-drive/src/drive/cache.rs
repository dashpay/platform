use std::collections::BTreeMap;
use dpp::identity::TimestampMillis;

mod data_contract;
mod protocol_version;
mod system_contracts;

pub use data_contract::DataContractCache;
use dpp::block::epoch::EpochIndex;
use platform_version::version::fee::FeeVersion;
pub use protocol_version::ProtocolVersionsCache;
pub use system_contracts::SystemDataContracts;

/// Drive cache struct
pub struct DriveCache {
    /// Cached contracts
    pub data_contracts: DataContractCache,
    // TODO: We probably don't need this since we have it genesis cache in the platform
    /// Genesis time in ms
    pub genesis_time_ms: parking_lot::RwLock<Option<TimestampMillis>>,
    // TODO: Make protocol versions cache thread-safe
    /// Lazy loaded counter of votes to upgrade protocol version
    pub protocol_versions_counter: parking_lot::RwLock<ProtocolVersionsCache>,
    /// Versioned system data contracts
    pub system_data_contracts: SystemDataContracts,
    /// Cached Epoch changed FeeVersion
    pub cached_fee_version: parking_lot::RwLock<BTreeMap<EpochIndex, FeeVersion>>,
}
